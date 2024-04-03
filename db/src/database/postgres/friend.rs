use crate::database::friend::FriendRepo;
use abi::errors::Error;
use abi::message::{Friend, Friendship, FriendshipStatus, FsCreate, FsReply, FsUpdate};
use async_trait::async_trait;
use futures::TryStreamExt;
use nanoid::nanoid;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

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
        let fs = sqlx::query_as(
            "INSERT INTO friendships
            (id, user_id, friend_id, status, apply_msg, req_remark, source, create_time)
            VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
        )
        .bind(&nanoid!())
        .bind(&fs.user_id)
        .bind(&fs.friend_id)
        .bind(&FriendshipStatus::Pending.to_string())
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
    async fn get_fs_list(&self, user_id: &str) -> Result<Vec<Friendship>, Error> {
        // let mut fs = sqlx::query_as("SELECT * FROM friendships WHERE user_id = $1")
        //     .bind(user_id)
        //     .fetch(&self.pool);
        // let (tx, rx) = mpsc::channel(100);
        // while let Some(result) = fs.try_next().await? {
        //     if tx.send(result).await.is_err() {
        //         break;
        //     };
        // }
        // Ok(rx)
        let fs = sqlx::query_as("SELECT * FROM friendships WHERE friend_id = $1")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(fs)
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
        user_id: &str,
        friend_id: &str,
        remark: &str,
    ) -> Result<Friendship, Error> {
        let fs = sqlx::query_as(
            "UPDATE friendships
            SET req_remark = CASE
                    WHEN user_id = $2 THEN $1
                    ELSE req_remark
                END,
                resp_remark = CASE
                    WHEN friend_id = $2 THEN $1
                    ELSE resp_remark
                END
            WHERE (user_id = $2 AND friend_id = $3)
            OR (user_id = $3 AND friend_id = $2)",
        )
        .bind(remark)
        .bind(user_id)
        .bind(friend_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(fs)
    }

    async fn update_friend_status(
        &self,
        user_id: &str,
        friend_id: &str,
        status: FriendshipStatus,
    ) -> Result<Friendship, Error> {
        let fs = sqlx::query_as(
            "UPDATE friendships
            SET status = $1
            WHERE (user_id = $2 AND friend_id = $3) OR (user_id = $3 AND friend_id = $2)",
        )
        .bind(status.to_string())
        .bind(user_id)
        .bind(friend_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(fs)
    }

    async fn get_friend_list(
        &self,
        user_id: &str,
    ) -> Result<Receiver<Result<Friend, Error>>, Error> {
        let mut list = sqlx::query_as(
            "SELECT
              u.id,
              u.name,
              u.account,
              u.avatar,
              u.gender,
              u.age,
              u.region,
              f.status,
              f.source,
              f.accept_time,
              CASE
                WHEN f.user_id = $1 THEN f.response_msg
                ELSE f.apply_msg
              END AS hello,
              CASE
                WHEN f.user_id = $1 THEN f.res_remark
                ELSE f.req_remark
              END AS remark
            FROM friendships AS f
            JOIN users AS u
            ON u.id = CASE
                        WHEN f.user_id = $1 THEN f.friend_id
                        ELSE f.user_id
                      END
            WHERE (f.user_id = $1 OR f.friend_id = $1)
              AND f.status = 'Accepted'
              AND u.is_delete = FALSE",
        )
        .bind(user_id)
        .fetch(&self.pool);
        let (tx, rx) = mpsc::channel(100);
        while let Some(result) = list.try_next().await? {
            if tx.send(Ok(result)).await.is_err() {
                break;
            };
        }
        Ok(rx)
    }

    async fn agree_friend_apply_request(&self, fs: FsReply) -> Result<Friendship, Error> {
        let fs = sqlx::query_as(
            "UPDATE friendships
            SET
                status = 'Accepted',
                accept_time = $1,
                resp_msg = $2,
                resp_remark = $3,
            WHERE id = $4",
        )
        .bind(chrono::Local::now().timestamp_millis())
        .bind(&fs.resp_msg)
        .bind(&fs.resp_remark)
        .bind(fs.id)
        .fetch_one(&self.pool)
        .await?;
        Ok(fs)
    }
}
