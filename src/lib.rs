#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![feature(rustc_private)]
#![feature(thread_local_state)]
extern crate bson as extbson;
#[allow(dead_code)]
extern crate futures;
extern crate futures_cpupool;
#[macro_use]
extern crate log;
extern crate rdkafka;
extern crate tokio_core;

pub mod logging_utils;
pub mod cli;
mod jq;
mod bson;

use futures::Future;
use futures::future::join_all;
use futures::stream::Stream;
use futures::sync::oneshot::Canceled;
use futures_cpupool::Builder;
use tokio_core::reactor::Core;

use rdkafka::Message;
use rdkafka::consumer::Consumer;
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::config::ClientConfig;
use rdkafka::message::OwnedMessage;
use rdkafka::producer::FutureProducer;

use std::ffi::CString;
use std::ffi::CStr;
use std::sync::Arc;
use std::sync::Mutex;

use jq::ffi::*;
use cli::TopicMetadata;
use cli::SinkMetadata;
use cli::SerializationType;

use bson::bson_to_jv;
use bson::jv_to_bson;
use extbson::to_bson;
use extbson::from_bson;

fn str_to_jv(payload: &str, length: usize) -> jv {
    let c_string = CString::new(payload).unwrap();
    let c_str_ptr = c_string.as_ptr();
    unsafe { jv_parse_sized(c_str_ptr, length as i32) }
}

fn jv_to_string_bytes<'a>(jv_value: jv) -> Option<&'a [u8]> {
    let json_as_string = unsafe { jv_dump_string(jv_value, 0) };
    let c_msg = unsafe { jv_string_value(json_as_string) };
    let result = unsafe { CStr::from_ptr(c_msg).to_str().unwrap() };
    // cleanup
    unsafe { jv_free(json_as_string) };
    Some(result.as_bytes())
}

fn jv_to_bson_bytes<'a>(jv_value: jv) -> Option<&'a [u8]> {
    jv_to_bson(jv_value).and_then(|bson| from_bson(bson).ok())
}

fn jv_to_bytes<'a>(jv_value: jv, serialization: &SerializationType) -> Option<&'a [u8]> {
    match serialization {
        &SerializationType::JSON => jv_to_string_bytes(jv_value),
        &SerializationType::BSON => jv_to_bson_bytes(jv_value),
    }
}

fn exec_jq_expr<'a>(
    parsed_json: jv,
    output_serialization: &SerializationType,
    jq_state: *mut jq_state,
) -> Result<Vec<&'a [u8]>, Canceled> {
    let mut vec = Vec::with_capacity(10);
    if unsafe { jv_get_kind(parsed_json) == jv_kind::JV_KIND_INVALID } {
        error!("Unable to parse json");
        unsafe { jv_free(parsed_json) };
        Err(Canceled)
    } else {
        unsafe {
            // this consumes parsed_json
            jq_start(jq_state, parsed_json, 0);
        };
        let mut result = unsafe { jq_next(jq_state) };
        while unsafe { jv_get_kind(result) != jv_kind::JV_KIND_INVALID } {
            // this consumes result
            match jv_to_bytes(result, output_serialization) {
                Some(msg) => vec.push(msg),
                None => error!("Unable to transform JV to bytes"),
            }
            result = unsafe { jq_next(jq_state) };
        }
        Ok(vec)
    }
}

fn jq_computation<'a>(
    msg: &'a OwnedMessage,
    input_serialization: &SerializationType,
    output_serialization: &SerializationType,
    jq_state: *mut jq_state,
) -> Result<Vec<&'a [u8]>, Canceled> {
    match input_serialization {
        &SerializationType::BSON => match msg.payload_view::<[u8]>() {
            Some(Ok(payload)) => {
                let try_bson = to_bson(payload);
                match try_bson {
                    Ok(bson) => exec_jq_expr(bson_to_jv(&bson), output_serialization, jq_state),
                    Err(_) => {
                        error!("could not decode bson");
                        Err(Canceled)
                    }
                }
            }
            Some(Err(_)) => {
                error!("Error processing message payload {:?}", msg);
                Err(Canceled)
            }
            None => {
                error!("No payload");
                Err(Canceled)
            }
        },
        &SerializationType::JSON => match msg.payload_view::<str>() {
            Some(Ok(payload)) => exec_jq_expr(
                str_to_jv(payload, payload.len()),
                output_serialization,
                jq_state,
            ),
            Some(Err(_)) => {
                error!("Error processing message payload {:?}", msg);
                Err(Canceled)
            }
            None => {
                error!("No payload");
                Err(Canceled)
            }
        },
    }
}

// Creates all the resources and runs the event loop. The event loop will:
//   1) receive a stream of messages from the `StreamConsumer`.
//   2) filter out eventual Kafka errors.
//   3) send the message to a thread pool for processing.
//   4) produce the result to the output topic.
// Moving each message from one stage of the pipeline to next one is handled by the event loop,
// that runs on a single thread. The expensive CPU-bound computation is handled by the `CpuPool`,
// without blocking the event loop.
pub fn run_async_processor(
    brokers: &str,
    group_id: &str,
    input_topic: &'static TopicMetadata,
    output_topic: &'static SinkMetadata,
    jq_expression: &str,
    parallelism: usize,
) {
    // Create the event loop. The event loop will run on a single thread and drive the pipeline.
    let mut core = Core::new().unwrap();

    // Initial jq state
    thread_local!(static jq_state: *mut jq_state = unsafe { jq_init() };);

    let arc_jq_expr = Arc::new(Mutex::new(CString::new(jq_expression).unwrap()));

    // Create the CPU pool, for CPU-intensive message processing.
    let cpu_pool = Builder::new()
        .pool_size(parallelism)
        .after_start(move || {
            jq_state.with(|state| unsafe {
                let cloned_jq_expr = arc_jq_expr.clone();
                let locked_jq_expr = cloned_jq_expr.lock().unwrap();
                let borrowed_jq_expr = locked_jq_expr.clone().into_raw();
                jq_compile(*state, borrowed_jq_expr);
            });
        })
        .create();

    // Create the `StreamConsumer`, to receive the messages from the topic in form of a `Stream`.
    let consumer = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .create::<StreamConsumer<_>>()
        .expect("Consumer creation failed");

    consumer
        .subscribe(&[input_topic.name])
        .expect("Can't subscribe to specified topic");

    // Create the `FutureProducer` to produce asynchronously.
    let producer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("produce.offset.report", "true")
        .create::<FutureProducer<_>>()
        .expect("Producer creation error");

    // Create a handle to the core, that will be used to provide additional asynchronous work
    // to the event loop.
    let handle = core.handle();

    // Create the outer pipeline on the message stream.
    let processed_stream = consumer
        .start()
        .filter_map(|result| {
            // Filter out errors
            match result {
                Ok(msg) => Some(msg),
                Err(kafka_error) => {
                    error!("Error while receiving from Kafka: {:?}", kafka_error);
                    None
                }
            }
        })
        .for_each(|msg| {
            // Process each message
            info!("Enqueuing message for computation");
            let producer = producer.clone();
            let owned_message = msg.detach();
            // Create the inner pipeline, that represents the processing of a single event.
            let process_message = cpu_pool
                .spawn_fn(move || {
                    jq_state.with(|state| {
                        let computation_results = jq_computation(
                            &owned_message,
                            &(input_topic.serialization),
                            &(output_topic.serialization()),
                            *state,
                        );
                        match computation_results {
                            Err(_) => {
                                error!("JQ Computation failed");
                                join_all(Vec::with_capacity(0))
                            }
                            Ok(ref results) => {
                                info!("Sending result");
                                let mut future_vector = Vec::with_capacity(results.len());
                                for computation_result in results {
                                    match output_topic {
                                        &SinkMetadata::StdOut => info!("{:?}", computation_result),
                                        // Send the result of the computation to Kafka, asynchronously.
                                        &SinkMetadata::SinkTopic { ref metadata } => future_vector
                                            .push(producer.send_copy::<[u8], ()>(
                                                metadata.name,
                                                None,
                                                Some(&computation_result),
                                                None,
                                                None,
                                                1000,
                                            )),
                                    }
                                }
                                join_all(future_vector)
                            }
                        }
                    })
                })
                .and_then(|d_reports| {
                    // Once the message has been produced, print the delivery report and terminate
                    // the pipeline.
                    for d_report in &d_reports {
                        info!("Delivery report for result: {:?}", d_report);
                    }
                    Ok(())
                })
                .or_else(|err| {
                    // In case of error, this closure will be executed instead.
                    error!("Error while processing message: {:?}", err);
                    Ok(())
                });
            // Spawns the inner pipeline in the same event pool.
            handle.spawn(process_message);
            Ok(())
        });

    info!("Starting event loop");
    // Runs the event pool until the consumer terminates.
    core.run(processed_stream).unwrap();
    info!("Stream processing terminated");
}

#[cfg(test)]
#[macro_use]
extern crate proptest;
mod tests {}
