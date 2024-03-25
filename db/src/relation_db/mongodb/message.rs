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
