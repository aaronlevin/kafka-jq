kafka-jq
========

```
kafka | jq | kafka
````

`kafka-jq` is a fun experiment in building a command-line-driven streaming application framework. It uses `rust-rdkafka` to pipe data from one [Kafka](https://kafka.apache.org/) topic to another, but allows you to transform that data using `jq`. `kafka-jq` supports data encoded as `JSON` or `BSON` on the intput or the output.

# Developing

Running:

```sh
$ nix-shell default.nix
```

will create a shell environment with the necessary libraries on your path (`jq` and `rdkafka`). Then you can start `zookeeper` and `kafka` locally:

```sh
[nix-shell:~/kafka-jq-rs]$ ./zookeeper-start.sh
ZooKeeper JMX enabled by default
Using config: ./zoo.cfg
Starting zookeeper ... STARTED


[nix-shell:~/kafka-jq-rs]$ ./kafka-broker-start.sh
```

This will start a single broker connected to zookeeper. 

## Initializing

If you haven't initialized a topic, run:

```sh
[nix-shell:~/stripe/kafka-jq-rs]$ ./kafka-topic-init.sh
```

Which will create a topic named `test` with a single partition. From here you can invoke `kafka-jq` with:

```sh
[nix-shell:~/stripe/kafka-jq-rs]$ kafka-jq --input-topic benchmark_topic_1KB:BSON --output-topic benchmark_topic_1KB-out:JSON --jq-expression '.key'
```

## Testing

`kafka-jq` uses `proptest` for the `JSON<->BSON` translation layer, but otherwise lacks tests. I currently use a forked [`kafka-benchmark`](https://github.com/fede1024/kafka-benchmark) to generate thousands of messages and push them into a topic that I'm reading from with `kafka-jq`. Ideally this would be improved soon.

## Feedback

This is my first nontrivial rust project and quite frankly I don't know what I'm doing, so if you have any feedback, please feel free to submit a ticket, PR, or send me a note!

## Credits

This project began during the Stripe Hackathon III, where [Nat Wilson](http://natw.49fold.com/) and I attempted to write `kafka-jq` in C to experience modern C development. It was fun but difficult and we thought the best way to address those two concerns was to re-write it in rust.
