extern crate clap;

use self::clap::{App, Arg};

pub enum SerializationType {
    JSON,
    BSON,
}

pub struct TopicMetadata<'a> {
    pub name: &'a str,
    pub serialization: SerializationType,
}

pub enum SinkMetadata<'a> {
    StdOut,
    SinkTopic { metadata: TopicMetadata<'a> },
}
impl<'a> SinkMetadata<'a> {
    pub fn serialization(&self) -> &SerializationType {
        match self {
            &SinkMetadata::StdOut {} => &SerializationType::JSON,
            &SinkMetadata::SinkTopic { ref metadata } => &metadata.serialization,
        }
    }
}

pub fn string_to_serialization_type(string: &str) -> Option<SerializationType> {
    match string {
        "JSON" => Some(SerializationType::JSON),
        "BSON" => Some(SerializationType::BSON),
        _ => None,
    }
}

pub fn mk_topic_serialization<'a, 'b>(topic_string: &'a str) -> Option<TopicMetadata<'b>>
where
    'a: 'b,
{
    let split_string: Vec<&'a str> = topic_string.split(':').collect();
    let length: usize = split_string.len();
    if length == 1 {
        Some(TopicMetadata {
            name: split_string[0],
            serialization: SerializationType::JSON,
        })
    } else if length == 2 {
        string_to_serialization_type(split_string[1]).map(|s| TopicMetadata {
            name: split_string[0],
            serialization: s,
        })
    } else {
        None
    }
}

pub fn mk_cli_matches<'a, 'b>() -> App<'a, 'b>
where
    'a: 'b,
{
    App::new("Async example")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("Asynchronous computation example")
        .arg(
            Arg::with_name("jq_expression")
                .short("j")
                .long("jq-expression")
                .help("The jq expression to apply to json")
                .takes_value(true)
                .default_value("."),
        )
        .arg(
            Arg::with_name("brokers")
                .short("b")
                .long("brokers")
                .help("Broker list in kafka format")
                .takes_value(true)
                .default_value("localhost:9092"),
        )
        .arg(
            Arg::with_name("group-id")
                .short("g")
                .long("group-id")
                .help("Consumer group id")
                .takes_value(true)
                .default_value("example_consumer_group_id"),
        )
        .arg(
            Arg::with_name("log-conf")
                .long("log-conf")
                .help("Configure the logging format (example: 'rdkafka=trace')")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("input-topic")
                .long("input-topic")
                .help("Input topic")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("output-topic")
                .long("output-topic")
                .help("Output topic")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("parallelism")
                .short("p")
                .long("pool-size")
                .help("Parallelism of consumer pool")
                .takes_value(true)
                .default_value("4"),
        )
}
