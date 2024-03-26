use crate::relation_db::message::MsgRecBoxRepo;
use abi::config::Config;
use abi::errors::Error;
use abi::message::MsgToDb;
use async_trait::async_trait;
use bson::{doc, to_document, Document};
use mongodb::{Client, Collection, Database};

/// user receive box,
/// need to category message
/// like: group message, single message, system message, service message, third party message etc.
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
    async fn save_message(&self, message: MsgToDb, collection: String) -> Result<(), Error> {
        let chat = self.db.collection(&collection);
        chat.insert_one(to_document(&message)?, None).await?;

        Ok(())
    }

    async fn delete_message(&self, message_id: String, collection: String) -> Result<(), Error> {
        let coll: Collection<Document> = self.db.collection(&collection);
        let query = doc! {"server_id": message_id};
        coll.delete_one(query, None).await?;
        Ok(())
    }

    async fn delete_messages(
        &self,
        message_ids: Vec<String>,
        collection: String,
    ) -> Result<(), Error> {
        let coll: Collection<Document> = self.db.collection(&collection);

        let query = doc! {"server_id": {"$in": message_ids}};
        coll.delete_many(query, None).await?;

        Ok(())
    }

    async fn get_message(
        &self,
        message_id: String,
        collection: String,
    ) -> Result<Option<MsgToDb>, Error> {
        let coll: Collection<Document> = self.db.collection(&collection);
        let doc = coll.find_one(doc! {"server_id": message_id}, None).await?;
        match doc {
            None => Ok(None),
            Some(doc) => Ok(Some(MsgToDb::try_from(doc)?)),
        }
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
        let collection = "test".to_string();
        // save it into mongodb
        msg_box.save_message(msg, collection.clone()).await.unwrap();
        let msg = msg_box
            .get_message(msg_id.to_string(), collection)
            .await
            .unwrap();
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
        let collection = "test".to_string();
        // save it into mongodb
        msg_box.save_message(msg, collection.clone()).await.unwrap();

        // delete it
        msg_box
            .delete_message(msg_id.to_string(), collection.clone())
            .await
            .unwrap();

        let msg = msg_box
            .get_message(msg_id.to_string(), collection)
            .await
            .unwrap();
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
        let collection = "test".to_string();
        // save it into mongodb
        msg_box
            .save_message(msg.clone(), collection.clone())
            .await
            .unwrap();

        msg.server_id = msg_id[1].clone();
        msg_box
            .save_message(msg.clone(), collection.clone())
            .await
            .unwrap();

        msg.server_id = msg_id[2].clone();
        msg_box
            .save_message(msg.clone(), collection.clone())
            .await
            .unwrap();

        // delete it
        msg_box
            .delete_messages(msg_id.clone(), collection.clone())
            .await
            .unwrap();

        let msg = msg_box
            .get_message(msg_id[0].to_owned(), collection.clone())
            .await
            .unwrap();
        assert!(msg.is_none());

        let msg = msg_box
            .get_message(msg_id[1].to_owned(), collection.clone())
            .await
            .unwrap();
        assert!(msg.is_none());

        let msg = msg_box
            .get_message(msg_id[2].to_owned(), collection)
            .await
            .unwrap();
        assert!(msg.is_none());
    }
}
