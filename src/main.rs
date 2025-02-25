use chrono::Utc;
use kafka::producer::{AsBytes, Record};
use rdkafka::config::FromClientConfig;
use rdkafka::producer::{BaseRecord, Producer, ThreadedProducer};
use rdkafka::ClientConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct KafkaMessage {
    id: String,
    content: String,
}

fn main() {
    let mut producer = kafka::producer::Producer::from_hosts(vec!["127.0.0.1:9094".to_owned()])
        .with_ack_timeout(Duration::from_secs(1))
        .with_required_acks(kafka::client::RequiredAcks::One)
        .create()
        .expect("failed to create producer");

    let i = Utc::now().timestamp_millis();
    let payload = &KafkaMessage {
        id: i.to_string(),
        content: format!("message#{}", i.to_string()),
    };
    let content = serde_json::to_string(payload).expect("failed to serialize message");
    producer
        .send(&Record::from_key_value(
            "main-kafka",
            i.to_string().as_bytes(),
            content.as_bytes(),
        ))
        .expect("failed to send");

    let producer = ThreadedProducer::from_config(
        &ClientConfig::new()
            .set("bootstrap.servers", "127.0.0.1:9094")
            .set("acks", "1")
            .set("delivery.timeout.ms", "1000"),
    )
    .expect("failed to create producer");
    let i = Utc::now().timestamp_millis();
    let payload = &KafkaMessage {
        id: i.to_string(),
        content: format!("message#{}", i.to_string()),
    };
    let content = serde_json::to_string(payload).expect("failed to serialize message");
    producer
        .send(
            BaseRecord::to("main-rdkafka")
                .key(i.to_string().as_bytes())
                .payload(content.as_bytes()),
        )
        .expect("failed to send");
    producer
        .send(
            BaseRecord::to("main-rdkafka")
                .key(i.to_string().as_bytes())
                .payload(content.as_bytes()),
        )
        .expect("failed to send");
    println!("finished")
}
