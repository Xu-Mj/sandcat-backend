use abi::config::Config;
use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use kafka::Error;

fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::load("./abi/fixtures/im.yml").unwrap();

    let topic = "xmj".to_owned();
    // consumer group; only one consumer can receive the message in the same group
    let group = "my-group".to_owned();

    if let Err(e) = consume_messages(group, topic, config.kafka.hosts) {
        println!("Failed consuming messages: {}", e);
    }
}

fn consume_messages(group: String, topic: String, brokers: Vec<String>) -> Result<(), Error> {
    let mut con = Consumer::from_hosts(brokers)
        .with_topic(topic)
        .with_group(group)
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
        if let Err(e) = con.commit_consumed() {
            println!("Error committing offsets: {:?}", e);
        }
    }
}
