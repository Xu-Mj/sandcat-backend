use crate::relation_db::message::MsgRecBoxRepo;
use abi::config::Config;
use abi::errors::Error;
use abi::message::MsgToDb;
use async_trait::async_trait;
use bson::{doc, to_document, Document};
use mongodb::options::FindOptions;
use mongodb::{Client, Collection, Database};
use tokio::sync::mpsc;
use tonic::codegen::tokio_stream::StreamExt;

/// user receive box,
/// need to category message
/// like: group message, single message, system message, service message, third party message etc.
/// or we set everyone a collection,
pub struct MsgBox {
    db: Database,
}

#[allow(dead_code)]
impl MsgBox {
    pub async fn new(db: Database) -> Self {
        Self { db }
    }
    pub async fn from_config(config: &Config) -> Self {
        let db = Client::with_uri_str(config.db.mongodb.url())
            .await
            .unwrap()
            .database(&config.db.mongodb.database);
        Self { db }
    }
}

#[async_trait]
impl MsgRecBoxRepo for MsgBox {
    async fn save_message(&self, collection: &str, message: MsgToDb) -> Result<(), Error> {
        let chat = self.db.collection(collection);
        chat.insert_one(to_document(&message)?, None).await?;

        Ok(())
    }

    async fn delete_message(&self, collection: &str, message_id: &str) -> Result<(), Error> {
        let coll: Collection<Document> = self.db.collection(collection);
        let query = doc! {"server_id": message_id};
        coll.delete_one(query, None).await?;
        Ok(())
    }

    async fn delete_messages(
        &self,
        collection: &str,
        message_ids: Vec<String>,
    ) -> Result<(), Error> {
        let coll: Collection<Document> = self.db.collection(collection);

        let query = doc! {"server_id": {"$in": message_ids}};
        coll.delete_many(query, None).await?;

        Ok(())
    }

    async fn get_message(
        &self,
        collection: &str,
        message_id: &str,
    ) -> Result<Option<MsgToDb>, Error> {
        let coll: Collection<Document> = self.db.collection(collection);
        let doc = coll.find_one(doc! {"server_id": message_id}, None).await?;
        match doc {
            None => Ok(None),
            Some(doc) => Ok(Some(MsgToDb::try_from(doc)?)),
        }
    }

    async fn get_messages(
        &self,
        collection: &str,
        start: i64,
        end: i64,
    ) -> Result<mpsc::Receiver<Result<MsgToDb, Error>>, Error> {
        let coll: Collection<Document> = self.db.collection(collection);
        let query = doc! {
            "seq": {
                "$gte": start,
                "$lte": end
            }
        };

        // sort by seq
        let option = FindOptions::builder().sort(Some(doc! {"seq": 1})).build();

        // query
        let mut cursor = coll.find(query, Some(option)).await?;
        let (tx, rx) = mpsc::channel(100);
        while let Some(result) = cursor.next().await {
            match result {
                Ok(doc) => {
                    if tx.send(Ok(MsgToDb::try_from(doc)?)).await.is_err() {
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
    use super::*;
    use std::ops::Deref;
    use utils::mongodb_tester::MongoDbTester;
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
        let msg = MsgToDb {
            local_id: "123".to_string(),
            server_id: msg_id.to_string(),
            send_time: chrono::Local::now().timestamp(),
            content_type: 0,
            content: "test".to_string(),
            send_id: "123".to_string(),
            receiver_id: "111".to_string(),
            seq: 0,
        };
        let collection = "test";
        // save it into mongodb
        msg_box.save_message(collection, msg).await.unwrap();
        let msg = msg_box.get_message(collection, msg_id).await.unwrap();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().server_id, msg_id);
    }

    #[tokio::test]
    async fn mongodb_insert_and_delete_and_get_works() {
        let msg_box = TestConfig::new().await;
        let msg_id = "123";
        let msg = MsgToDb {
            local_id: "123".to_string(),
            server_id: msg_id.to_string(),
            send_time: chrono::Local::now().timestamp(),
            content_type: 0,
            content: "test".to_string(),
            send_id: "123".to_string(),
            receiver_id: "111".to_string(),
            seq: 0,
        };
        let collection = "test";
        // save it into mongodb
        msg_box.save_message(collection, msg).await.unwrap();

        // delete it
        msg_box.delete_message(collection, msg_id).await.unwrap();

        let msg = msg_box.get_message(collection, msg_id).await.unwrap();
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn mongodb_insert_and_batch_delete_and_get_should_works() {
        let msg_box = TestConfig::new().await;
        let msg_id = vec!["123".to_string(), "124".to_string(), "125".to_string()];
        let mut msg = MsgToDb {
            local_id: "123".to_string(),
            server_id: msg_id[0].clone(),
            send_time: chrono::Local::now().timestamp(),
            content_type: 0,
            content: "test".to_string(),
            send_id: "123".to_string(),
            receiver_id: "111".to_string(),
            seq: 0,
        };
        let collection = "test";
        // save it into mongodb
        msg_box.save_message(collection, msg.clone()).await.unwrap();

        msg.server_id = msg_id[1].clone();
        msg_box.save_message(collection, msg.clone()).await.unwrap();

        msg.server_id = msg_id[2].clone();
        msg_box.save_message(collection, msg.clone()).await.unwrap();

        // delete it
        msg_box
            .delete_messages(collection, msg_id.clone())
            .await
            .unwrap();

        let msg = msg_box.get_message(collection, &msg_id[0]).await.unwrap();
        assert!(msg.is_none());

        let msg = msg_box.get_message(collection, &msg_id[1]).await.unwrap();
        assert!(msg.is_none());

        let msg = msg_box.get_message(collection, &msg_id[2]).await.unwrap();
        assert!(msg.is_none());
    }
}
