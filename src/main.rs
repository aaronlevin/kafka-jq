extern crate clap;
extern crate kafka_jq;
#[macro_use]
extern crate lazy_static;

use kafka_jq::run_async_processor;
use kafka_jq::logging_utils::setup_logger;
use kafka_jq::cli::TopicMetadata;
use kafka_jq::cli::SinkMetadata;
use kafka_jq::cli::mk_cli_matches;
use kafka_jq::cli::mk_topic_serialization;
use clap::ArgMatches;

fn main() {
    lazy_static! {
        static ref MATCHES: ArgMatches<'static> = mk_cli_matches().get_matches();
        static ref INPUT_TOPIC: TopicMetadata<'static> =
            mk_topic_serialization(MATCHES.value_of("input-topic").unwrap()).unwrap();
        static ref OUTPUT_TOPIC: SinkMetadata<'static> =
            MATCHES
            .value_of("output-topic")
            .and_then(|topic| mk_topic_serialization(topic))
            .map(|topic| SinkMetadata::SinkTopic{ metadata: topic })
            .unwrap_or(SinkMetadata::StdOut);
    }

    setup_logger(true, MATCHES.value_of("log-conf"));

    let brokers = MATCHES.value_of("brokers").unwrap();
    let group_id = MATCHES.value_of("group-id").unwrap();
    let jq_expression = MATCHES.value_of("jq_expression").unwrap();
    let parallelism = str::parse::<usize>(MATCHES.value_of("parallelism").unwrap()).unwrap();

    run_async_processor(
        brokers,
        group_id,
        &INPUT_TOPIC,
        &OUTPUT_TOPIC,
        jq_expression,
        parallelism,
    );
}
