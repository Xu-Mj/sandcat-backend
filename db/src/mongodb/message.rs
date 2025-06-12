use std::fmt::Debug;

use async_trait::async_trait;
use mongodb::options::{FindOptions, IndexOptions};
use mongodb::{
    Client, Collection, Database, IndexModel,
    bson::{Document, doc},
};
use tokio::sync::mpsc;
use tonic::codegen::tokio_stream::StreamExt;
use tracing::log::debug;

use abi::config::Config;
use abi::errors::Error;
use abi::message::{GroupMemSeq, Msg};

use crate::message::{MsgRecBoxCleaner, MsgRecBoxRepo};
use crate::mongodb::utils::to_doc;

/// user receive box,
/// need to category message
/// like: group message, single message, system message, service message, third party message etc.
/// or we set everyone a collection,
#[derive(Debug)]
pub struct MsgBox {
    /// for message box
    mb: Collection<Document>,
}

/// for all users single message receive box
const COLL_SINGLE_BOX: &str = "single_msg_box";

#[allow(dead_code)]
impl MsgBox {
    pub async fn new(db: Database) -> Self {
        let mb = db.collection(COLL_SINGLE_BOX);
        Self { mb }
    }
    pub async fn from_config(config: &Config) -> Self {
        let db = Client::with_uri_str(config.db.mongodb.url())
            .await
            .unwrap()
            .database(&config.db.mongodb.database);
        let mb = db.collection(COLL_SINGLE_BOX);

        // create server_id index
        let index_model = IndexModel::builder()
            .keys(doc! {"receiver_id": 1, "seq":1})
            .options(IndexOptions::builder().unique(false).build())
            .build();
        mb.create_index(index_model, None).await.unwrap();
        debug!("create [receiver_id, seq] index for message box");

        let index_model = IndexModel::builder()
            .keys(doc! {"send_id": 1, "send_seq":1})
            .options(IndexOptions::builder().unique(false).build())
            .build();
        mb.create_index(index_model, None).await.unwrap();
        debug!("create [send_id, send_seq] index for message box");

        Self { mb }
    }
}

#[async_trait]
impl MsgRecBoxRepo for MsgBox {
    async fn save_message(&self, message: &Msg) -> Result<(), Error> {
        self.mb.insert_one(to_doc(message)?, None).await?;

        Ok(())
    }

    async fn save_group_msg(
        &self,
        mut message: Msg,
        members: Vec<GroupMemSeq>,
    ) -> Result<(), Error> {
        let mut messages = Vec::with_capacity(members.len() + 1);
        // save message for sender
        messages.push(to_doc(&message)?);

        // reset message send_seq
        message.send_seq = 0;

        // modify message receiver id
        for seq in members {
            // increase members sequence
            message.seq = seq.cur_seq;

            message.receiver_id = seq.mem_id;

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

    async fn delete_messages(&self, user_id: &str, msg_seq: Vec<i64>) -> Result<(), Error> {
        let query = doc! {"receiver_id": user_id, "seq": {"$in": msg_seq}};
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

    async fn get_messages_stream(
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

    async fn get_messages(&self, user_id: &str, start: i64, end: i64) -> Result<Vec<Msg>, Error> {
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
        let mut messages = Vec::with_capacity((end - start) as usize);
        while let Some(result) = cursor.next().await {
            let msg = Msg::try_from(result?)?;
            messages.push(msg)
        }

        Ok(messages)
    }

    async fn get_msgs(
        &self,
        user_id: &str,
        send_start: i64,
        send_end: i64,
        rec_start: i64,
        rec_end: i64,
    ) -> Result<Vec<Msg>, Error> {
        let pipeline = vec![
            doc! {
                "$match": {
                    "$or": [
                        {
                            "receiver_id": user_id,
                            "seq": { "$gt": rec_start, "$lte": rec_end }
                        },
                        {
                            "send_id": user_id,
                            "send_seq": { "$gt": send_start, "$lte": send_end }
                        }
                    ]
                }
            },
            doc! {
                "$addFields": {
                    "sort_field": {
                        "$cond": {
                            "if": { "$eq": ["$send_id", user_id] },
                            "then": "$send_seq",
                            "else": "$seq"
                        }
                    }
                }
            },
            doc! {
                "$sort": {
                    "send_time": 1,
                    "sort_field": 1
                }
            },
        ];

        let len = send_end - send_start + (rec_end - rec_start);
        // query
        let mut cursor = self.mb.aggregate(pipeline, None).await?;

        let mut messages = Vec::with_capacity((len) as usize);
        while let Some(result) = cursor.next().await {
            let mut msg = Msg::try_from(result?)?;
            // set seq to 0 if the message is sent by the user
            if user_id == msg.send_id {
                msg.seq = 0;
            }
            messages.push(msg)
        }
        Ok(messages)
    }

    async fn msg_read(&self, user_id: &str, msg_seq: &[i64]) -> Result<(), Error> {
        if msg_seq.is_empty() {
            return Ok(());
        }
        let query = doc! {"receiver_id":{"$eq":user_id},"seq":{"$in":msg_seq}};
        let update = doc! {"$set":{"is_read":true}};
        self.mb.update_many(query, update, None).await?;
        Ok(())
    }
}

impl MsgRecBoxCleaner for MsgBox {
    fn clean_receive_box(&self, period: i64, types: Vec<i32>) {
        let mb = self.mb.clone();

        tokio::spawn(async move {
            let retention_duration = chrono::Duration::days(period);
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(24 * 60 * 60)); // 每24小时执行一次
            loop {
                interval.tick().await;
                let now = chrono::Utc::now();
                let cutoff_time = (now - retention_duration).timestamp();

                let result = mb
                    .delete_many(
                        doc! {
                            "send_time": { "$lt": cutoff_time },
                            "msg_type": { "$nin": types.clone()}
                        },
                        None,
                    )
                    .await;

                match result {
                    Ok(delete_result) => {
                        println!("Deleted {} expired messages", delete_result.deleted_count);
                    }
                    Err(e) => {
                        eprintln!("Error deleting expired messages: {:?}", e);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use abi::message::{MsgType, PlatformType};
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
            let config = Config::load("../config.yml").unwrap();
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
            client_id: "123".to_string(),
            server_id: msg_id,
            create_time: 0,
            send_time: chrono::Local::now().timestamp(),
            content_type: 0,
            content: "test".to_string().into_bytes(),
            send_id: "123".to_string(),
            receiver_id: "111".to_string(),
            seq: 0,
            send_seq: 0,
            msg_type: MsgType::SingleMsg as i32,
            is_read: false,
            platform: PlatformType::Mobile as i32,
            // sdp: None,
            // sdp_mid: None,
            // sdp_m_index: None,
            group_id: "".to_string(),
            avatar: "".to_string(),
            nickname: "".to_string(),
            related_msg_id: None,
        }
    }
    #[tokio::test]
    async fn mongodb_insert_and_batch_delete_and_get_should_works() {
        let msg_box = TestConfig::new().await;
        let msg_id = vec!["123".to_string(), "124".to_string(), "125".to_string()];
        let mut msg = get_test_msg(msg_id[0].clone());

        let msg_seq = vec![12, 123, 1234];
        // save it into mongodb
        msg_box.save_message(&msg).await.unwrap();

        msg.server_id.clone_from(&msg_id[1]);
        msg.seq = msg_seq[1];
        msg_box.save_message(&msg).await.unwrap();

        msg.seq = msg_seq[2];
        msg.server_id.clone_from(&msg_id[2]);
        msg_box.save_message(&msg).await.unwrap();

        // delete it
        msg_box
            .delete_messages("111", msg_seq.clone())
            .await
            .unwrap();

        let msg = msg_box.get_message(&msg_id[0]).await.unwrap();
        assert!(msg.is_none());

        let msg = msg_box.get_message(&msg_id[1]).await.unwrap();
        assert!(msg.is_none());

        let msg = msg_box.get_message(&msg_id[2]).await.unwrap();
        assert!(msg.is_none());
    }
}
