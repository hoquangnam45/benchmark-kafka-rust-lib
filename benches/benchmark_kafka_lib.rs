use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kafka::client::FetchOffset;
use kafka::producer::{AsBytes, Record};
use rdkafka::config::FromClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::error::{KafkaError, RDKafkaErrorCode};
use rdkafka::producer::{BaseRecord, Producer, ThreadedProducer};
use rdkafka::ClientConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct KafkaMessage {
    id: String,
    content: String,
}

fn bench_kafka_produce(c: &mut Criterion) {
    let mut producer = kafka::producer::Producer::from_hosts(vec!["127.0.0.1:9094".to_owned()])
        .with_ack_timeout(Duration::from_secs(1))
        .with_required_acks(kafka::client::RequiredAcks::One)
        .create()
        .expect("failed to create producer");

    c.bench_function("bench_kafka_produce", |b| {
        b.iter(|| {
            let i = Utc::now().timestamp_millis();
            let payload = &KafkaMessage {
                id: i.to_string(),
                content: format!("message#{}", i.to_string()),
            };
            let content = serde_json::to_string(payload).expect("failed to serialize message");
            producer
                .send(&Record::from_key_value(
                    "benchmark-kafka",
                    i.to_string().as_bytes(),
                    content.as_bytes(),
                ))
                .expect("failed to send");
        })
    });
}

fn bench_rdkafka_produce(c: &mut Criterion) {
    let producer = ThreadedProducer::from_config(
        &ClientConfig::new()
            .set("bootstrap.servers", "127.0.0.1:9094")
            .set("acks", "1")
            .set("delivery.timeout.ms", "1000"),
    )
    .expect("failed to create producer");
    c.bench_function("bench_rdkafka_produce", |b| {
        b.iter(|| {
            let i = Utc::now().timestamp_millis();
            let key = i.to_string();
            let payload = &KafkaMessage {
                id: i.to_string(),
                content: format!("message#{}", i.to_string()),
            };
            let content = serde_json::to_string(payload).expect("failed to serialize message");

            let res = producer.send(
                BaseRecord::to("benchmark-rdkafka")
                    .key(&key)
                    .payload(content.as_bytes()),
            );
            if let Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), _x)) = res {
                producer.poll(Duration::from_secs(3));
                let _ = producer
                    .send(
                        BaseRecord::to("benchmark-rdkafka")
                            .key(&key)
                            .payload(content.as_bytes()),
                    )
                    .expect("failed to send");
            } else {
                res.expect("failed to send");
            }
        })
    });
    producer.flush(Duration::from_secs(30)).expect("failed to flush queue");
}

fn bench_kafka_consumer(c: &mut Criterion) {
    let mut consumer = kafka::consumer::Consumer::from_hosts(vec!["127.0.0.1:9094".to_owned()])
        .with_topic("benchmark-kafka".to_owned())
        .with_fallback_offset(FetchOffset::Earliest)
        .with_group("benchmark-kafka-group".to_owned())
        .create()
        .expect("consumer create failed");

    c.bench_function("bench_kafka_consumer", |b| {
        b.iter(|| {
            for ms in consumer.poll().iter() {
                for m in ms.iter() {
                    black_box(consumer.consume_messageset(m).expect("consumer have error"));
                    black_box(consumer.commit_consumed().expect("commit consumed failed"));
                }
            }
        })
    });
}

fn bench_rdkafka_consumer(c: &mut Criterion) {
    let consumer = BaseConsumer::from_config(
        &ClientConfig::new()
            .set("bootstrap.servers", "127.0.0.1:9094")
            .set("auto.offset.reset", "earliest")
            .set("group.id", "benchmark-rdkafka-group")
            .set("kafka.offsets.storage", "kafka")
            .set("enable.auto.commit", "false"),
    )
    .expect("failed to create consumer");

    consumer
        .subscribe(&["benchmark-rdkafka"])
        .expect("Can't subscribe to specified topics");

    c.bench_function("bench_rdkafka_consumer", |b| {
        b.iter(|| {
            for ms in consumer.poll(None).iter() {
                for m in ms.iter() {
                    black_box(m.detach());
                }
            }
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(1000).measurement_time(Duration::from_secs(10)).warm_up_time(Duration::from_secs(3));
    targets = bench_rdkafka_produce, bench_kafka_produce
}
criterion_main!(benches);
