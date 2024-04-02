use crate::database::friend::FriendRepo;
use abi::errors::Error;
use abi::message::{Friendship, FriendshipStatus, FsCreate, FsUpdate};
use async_trait::async_trait;
use futures::TryStreamExt;
use nanoid::nanoid;
use sqlx::PgPool;
use tokio::sync::mpsc;

pub struct PostgresFriend {
    pool: PgPool,
}

impl PostgresFriend {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FriendRepo for PostgresFriend {
    async fn create_fs(&self, fs: FsCreate) -> Result<Friendship, Error> {
        let status = FriendshipStatus::try_from(fs.status)
            .map_err(|_| Error::InternalServer("Invalid status".to_string()))?;

        let fs = sqlx::query_as(
            "INSERT INTO friendships
            (id, user_id, friend_id, status, apply_msg, req_remark, source, create_time)
            VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
        )
        .bind(&nanoid!())
        .bind(&fs.user_id)
        .bind(&fs.friend_id)
        .bind(&status.to_string())
        .bind(&fs.apply_msg)
        .bind(&fs.req_remark)
        .bind(&fs.source)
        .bind(chrono::Local::now().timestamp_millis())
        .fetch_one(&self.pool)
        .await?;
        Ok(fs)
    }

    /// it is no necessary to know who is the friend
    async fn get_fs(&self, user_id: &str, friend_id: &str) -> Result<Friendship, Error> {
        let fs = sqlx::query_as("SELECT * FROM friendships WHERE (user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1)")
            .bind(user_id)
            .bind(friend_id)
            .fetch_one(&self.pool).await?;
        Ok(fs)
    }

    /// need to return the friendship with user information
    async fn get_friend_list(&self, user_id: &str) -> Result<mpsc::Receiver<Friendship>, Error> {
        let mut fs = sqlx::query_as("SELECT * FROM friendships WHERE user_id = $1")
            .bind(user_id)
            .fetch(&self.pool);
        let (tx, rx) = mpsc::channel(100);
        while let Some(result) = fs.try_next().await? {
            if tx.send(result).await.is_err() {
                break;
            };
        }
        Ok(rx)
    }

    async fn update_fs(&self, fs: FsUpdate) -> Result<Friendship, Error> {
        let fs = sqlx::query_as(
            "UPDATE friendships
            SET
            status = $1,
            apply_msg = $2,
            req_remark = $3,
            update_time = $4
            WHERE id = $5
            RETURNING *",
        )
        .bind(FriendshipStatus::Pending.as_str_name())
        .bind(&fs.apply_msg)
        .bind(&fs.req_remark)
        .bind(chrono::Local::now().timestamp_millis())
        .bind(&fs.id)
        .fetch_one(&self.pool)
        .await?;
        Ok(fs)
    }

    async fn update_friend_remark(
        &self,
        _user_id: &str,
        _friend_id: &str,
        _remark: &str,
    ) -> Result<Friendship, Error> {
        todo!()
    }

    async fn update_friend_status(
        &self,
        _user_id: &str,
        _friend_id: &str,
        _status: FriendshipStatus,
    ) -> Result<Friendship, Error> {
        todo!()
    }
}
