use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use kafka::Error;

fn main() {
    tracing_subscriber::fmt::init();

    let broker = "localhost:9092".to_owned();
    let topic = "test".to_owned();
    // consumer group; only one consumer can receive the message in the same group
    let group = "my-group".to_owned();

    if let Err(e) = consume_messages(group, topic, vec![broker]) {
        println!("Failed consuming messages: {}", e);
    }
}

fn consume_messages(group: String, topic: String, brokers: Vec<String>) -> Result<(), Error> {
    let mut con = Consumer::from_hosts(brokers)
        .with_topic(topic)
        .with_group(group)
        .with_client_id("test-consumer".to_string())
        .with_fallback_offset(FetchOffset::Earliest)
        .with_offset_storage(Some(GroupOffsetStorage::Kafka))
        .create()?;

    loop {
        match con.poll() {
            Ok(mss) => {
                for ms in mss.iter() {
                    for m in ms.messages() {
                        println!(
                            "Received a message: {:?}",
                            String::from_utf8(m.value.to_vec())
                        );
                        // Handle message
                    }
                    // Mark message as consumed
                    con.consume_messageset(ms)
                        .expect("Error marking message as consumed");
                }
            }
            Err(e) => println!("Error polling messages: {:?}", e),
        }

        // Commit offsets of consumed messages
        con.commit_consumed().expect("Failed to commit messages");
    }
}
