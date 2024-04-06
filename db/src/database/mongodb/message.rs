use async_trait::async_trait;
use bson::{doc, Document};
use mongodb::options::FindOptions;
use mongodb::{Client, Collection, Database};
use tokio::sync::mpsc;
use tonic::codegen::tokio_stream::StreamExt;

use abi::config::Config;
use abi::errors::Error;
use abi::message::Msg;

use crate::database::message::MsgRecBoxRepo;
use crate::database::mongodb::utils::to_doc;

/// user receive box,
/// need to category message
/// like: group message, single message, system message, service message, third party message etc.
/// or we set everyone a collection,
pub struct MsgBox {
    /// for message box
    mb: Collection<Document>,
}

/// for all users single message receive box
const COLL_SINGLE_BOX: &str = "single_msg_box";

#[allow(dead_code)]
impl MsgBox {
    pub async fn new(db: Database) -> Self {
        let sb = db.collection(COLL_SINGLE_BOX);
        Self { mb: sb }
    }
    pub async fn from_config(config: &Config) -> Self {
        let db = Client::with_uri_str(config.db.mongodb.url())
            .await
            .unwrap()
            .database(&config.db.mongodb.database);
        let sb = db.collection(COLL_SINGLE_BOX);
        Self { mb: sb }
    }
}

#[async_trait]
impl MsgRecBoxRepo for MsgBox {
    async fn save_message(&self, message: &Msg) -> Result<(), Error> {
        self.mb.insert_one(to_doc(message)?, None).await?;

        Ok(())
    }

    async fn save_group_msg(&self, mut message: Msg, members: Vec<String>) -> Result<(), Error> {
        let mut messages = Vec::with_capacity(members.len());
        // modify message receiver id
        for id in members {
            message.receiver_id = id;
            messages.push(to_doc(&message)?);
        }
        self.mb.insert_many(messages, None).await?;
        Ok(())
    }

    async fn delete_message(&self, message_id: &str) -> Result<(), Error> {
        let query = doc! {"server_id": message_id};
        self.mb.delete_one(query, None).await?;
        Ok(())
    }

    async fn delete_messages(&self, message_ids: Vec<String>) -> Result<(), Error> {
        let query = doc! {"server_id": {"$in": message_ids}};
        self.mb.delete_many(query, None).await?;

        Ok(())
    }

    async fn get_message(&self, message_id: &str) -> Result<Option<Msg>, Error> {
        let doc = self
            .mb
            .find_one(doc! {"server_id": message_id}, None)
            .await?;
        match doc {
            None => Ok(None),
            Some(doc) => Ok(Some(Msg::try_from(doc)?)),
        }
    }

    async fn get_messages(
        &self,
        user_id: &str,
        start: i64,
        end: i64,
    ) -> Result<mpsc::Receiver<Result<Msg, Error>>, Error> {
        let query = doc! {
            "receiver_id": user_id,
            "seq": {
                "$gte": start,
                "$lte": end
            }
        };

        // sort by seq
        let option = FindOptions::builder().sort(Some(doc! {"seq": 1})).build();

        // query
        let mut cursor = self.mb.find(query, Some(option)).await?;
        let (tx, rx) = mpsc::channel(100);
        while let Some(result) = cursor.next().await {
            match result {
                Ok(doc) => {
                    if tx.send(Ok(Msg::try_from(doc)?)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    if tx.send(Err(e.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use abi::message::MsgType;
    use utils::mongodb_tester::MongoDbTester;

    use super::*;

    struct TestConfig {
        box_: MsgBox,
        _tdb: MongoDbTester,
    }

    impl Deref for TestConfig {
        type Target = MsgBox;
        fn deref(&self) -> &Self::Target {
            &self.box_
        }
    }

    impl TestConfig {
        pub async fn new() -> Self {
            let config = Config::load("../abi/fixtures/im.yml").unwrap();
            let tdb = MongoDbTester::new(
                &config.db.mongodb.host,
                config.db.mongodb.port,
                &config.db.mongodb.user,
                &config.db.mongodb.password,
            )
            .await;
            let msg_box = MsgBox::new(tdb.database().await).await;
            Self {
                box_: msg_box,
                _tdb: tdb,
            }
        }
    }

    #[tokio::test]
    async fn mongodb_insert_and_get_works() {
        let msg_box = TestConfig::new().await;
        let msg_id = "123";
        let msg = get_test_msg(msg_id.to_string());
        // save it into mongodb
        msg_box.save_message(&msg).await.unwrap();
        let msg = msg_box.get_message(msg_id).await.unwrap();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().server_id, msg_id);
    }

    #[tokio::test]
    async fn mongodb_insert_and_delete_and_get_works() {
        let msg_box = TestConfig::new().await;
        let msg_id = "123";
        let msg = get_test_msg(msg_id.to_string());
        // save it into mongodb
        msg_box.save_message(&msg).await.unwrap();

        // delete it
        msg_box.delete_message(msg_id).await.unwrap();

        let msg = msg_box.get_message(msg_id).await.unwrap();
        assert!(msg.is_none());
    }

    fn get_test_msg(msg_id: String) -> Msg {
        Msg {
            local_id: "123".to_string(),
            server_id: msg_id,
            create_time: 0,
            send_time: chrono::Local::now().timestamp(),
            content_type: 0,
            content: "test".to_string().into_bytes(),
            send_id: "123".to_string(),
            receiver_id: "111".to_string(),
            seq: 0,
            group_id: "".to_string(),
            msg_type: MsgType::SingleMsg as i32,
            is_read: false,
            sdp: None,
            sdp_mid: None,
            sdp_m_index: None,
            call_agree: false,
        }
    }
    #[tokio::test]
    async fn mongodb_insert_and_batch_delete_and_get_should_works() {
        let msg_box = TestConfig::new().await;
        let msg_id = vec!["123".to_string(), "124".to_string(), "125".to_string()];
        let mut msg = get_test_msg(msg_id[0].clone());
        // save it into mongodb
        msg_box.save_message(&msg).await.unwrap();

        msg.server_id = msg_id[1].clone();
        msg_box.save_message(&msg).await.unwrap();

        msg.server_id = msg_id[2].clone();
        msg_box.save_message(&msg).await.unwrap();

        // delete it
        msg_box.delete_messages(msg_id.clone()).await.unwrap();

        let msg = msg_box.get_message(&msg_id[0]).await.unwrap();
        assert!(msg.is_none());

        let msg = msg_box.get_message(&msg_id[1]).await.unwrap();
        assert!(msg.is_none());

        let msg = msg_box.get_message(&msg_id[2]).await.unwrap();
        assert!(msg.is_none());
    }
}
