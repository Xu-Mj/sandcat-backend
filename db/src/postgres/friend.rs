use async_trait::async_trait;
use nanoid::nanoid;
use sqlx::PgPool;
use tracing::debug;

use abi::errors::Error;
use abi::message::{
    AgreeReply, Friend, FriendDb, Friendship, FriendshipStatus, FriendshipWithUser, FsCreate,
    FsUpdate, User,
};

use crate::friend::FriendRepo;

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
    async fn create_fs(
        &self,
        fs: FsCreate,
    ) -> Result<(FriendshipWithUser, FriendshipWithUser), Error> {
        let user_id = fs.user_id.clone();
        let now = chrono::Utc::now().timestamp_millis();
        let mut transaction = self.pool.begin().await?;
        debug!("create_fs: {:?}", &fs);
        let fs_id: (String,) = sqlx::query_as(
            "INSERT INTO friendships
                (id, user_id, friend_id, status, apply_msg, req_remark, source, create_time, update_time)
             VALUES
                ($1, $2, $3, $4::friend_request_status, $5, $6, $7, $8, $8)
             ON CONFLICT (user_id, friend_id)
             DO UPDATE
                SET apply_msg = EXCLUDED.apply_msg, req_remark = EXCLUDED.req_remark,
                source = EXCLUDED.source, create_time = EXCLUDED.create_time, status = EXCLUDED.status
             RETURNING id",
        )
        .bind(nanoid!())
        .bind(&fs.user_id)
        .bind(&fs.friend_id)
        .bind(FriendshipStatus::Pending.to_string())
        .bind(&fs.apply_msg)
        .bind(&fs.req_remark)
        .bind(&fs.source)
        .bind(now)
        .fetch_one(&mut *transaction)
        .await?;

        // select user information
        let mut users: Vec<User> = sqlx::query_as("SELECT * FROM users WHERE id = $1 OR id = $2")
            .bind(&fs.user_id)
            .bind(&fs.friend_id)
            .fetch_all(&mut *transaction)
            .await?;
        transaction.commit().await?;

        let user1 = users.remove(0);
        let (user, friend) = if user1.id == user_id {
            (user1, users.remove(0))
        } else {
            (users.remove(0), user1)
        };

        let mut fs_req = FriendshipWithUser::from(friend);
        fs_req.fs_id.clone_from(&fs_id.0);
        fs_req.status = FriendshipStatus::Pending as i32;
        fs_req.source.clone_from(&fs.source);
        fs_req.create_time = now;
        fs_req.remark = fs.req_remark;

        let fs_send = FriendshipWithUser {
            fs_id: fs_id.0,
            user_id,
            name: user.name,
            avatar: user.avatar,
            gender: user.gender,
            age: user.age,
            region: user.region,
            status: FriendshipStatus::Pending as i32,
            apply_msg: fs.apply_msg,
            source: fs.source,
            create_time: now,
            account: user.account,
            remark: None,
            email: None,
        };
        Ok((fs_req, fs_send))
    }

    // it is no necessary to know who is the friend
    // async fn get_fs(&self, user_id: &str, friend_id: &str) -> Result<FriendshipWithUser, Error> {
    //     let fs = sqlx::query_as(
    //         "SELECT f.id as fs_id, f.user_id, u.name, u.avatar, u.gender, u.age, u.region,
    //           f.status as status, f.apply_msg, f.source, f.create_time
    //           FROM friendships f
    //           JOIN users u ON f.friend_id = u.id
    //           WHERE f.id = $1")
    //         .bind(user_id)
    //         .bind(friend_id)
    //         .fetch_one(&self.pool).await?;
    //     Ok(fs)
    // }

    /// get friendship list by user_id and offline_time
    async fn get_fs_list(
        &self,
        user_id: &str,
        offline_time: i64,
    ) -> Result<Vec<FriendshipWithUser>, Error> {
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
        let fs = sqlx::query_as(
            "SELECT f.id as fs_id, f.user_id, f.apply_msg, f.source, f.create_time,
             u.name, u.avatar, u.gender, u.age, u.region
             FROM friendships AS f
             JOIN users AS u ON f.user_id = u.id
             WHERE f.friend_id = $1 and f.update_time > $2",
        )
        .bind(user_id)
        .bind(offline_time)
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
        .bind(chrono::Utc::now().timestamp_millis())
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
    ) -> Result<FriendDb, Error> {
        let fs = sqlx::query_as(
            "UPDATE friends
            SET remark = $1 ,
            update_time = $2
            WHERE (user_id = $3 AND friend_id = $4)
            RETURNING *",
        )
        .bind(remark)
        .bind(chrono::Utc::now().timestamp_millis())
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
        offline_time: i64,
    ) -> Result<Vec<Friend>, Error> {
        let list = sqlx::query_as(
            "SELECT u.id as friend_id, u.name, u.account, u.email, u.avatar, u.gender, u.age, u.region, u.signature,
                        f.fs_id as fs_id, f.status, f.source, f.create_time, f.update_time, f.remark
            FROM friends AS f
            JOIN users AS u
            ON u.id =  f.friend_id
            WHERE f.user_id = $1
            AND f.update_time > $2
            AND u.is_delete = FALSE",)
              .bind(user_id)
              .bind(offline_time)
              .fetch_all(&self.pool)
              .await?;
        Ok(list)
        // )        let list = sqlx::query_as(
        //     "SELECT u.id as friend_id, u.name, u.account, u.avatar, u.gender, u.age, u.region, u.signature,
        //                 f.id as fs_id, f.status, f.source, f.create_time, f.accept_time,
        //       CASE
        //         WHEN f.user_id = $1 THEN f.resp_msg
        //         ELSE f.apply_msg
        //       END AS hello,
        //       CASE
        //         WHEN f.user_id = $1 THEN f.resp_remark
        //         ELSE f.req_remark
        //       END AS remark
        //     FROM friendships AS f
        //     JOIN users AS u
        //     ON u.id = CASE
        //                 WHEN f.user_id = $1 THEN f.friend_id
        //                 ELSE f.user_id
        //               END
        //     WHERE (f.user_id = $1 OR f.friend_id = $1)
        //       AND f.status = 'Accepted'
        //       AND f.accept_time > $2
        //       AND u.is_delete = FALSE",
        // )
        // let (tx, rx) = mpsc::channel(100);
        // while let Some(result) = list.try_next().await? {
        //     if tx.send(Ok(result)).await.is_err() {
        //         break;
        //     };
        // }
    }

    async fn agree_friend_apply_request(&self, fs: AgreeReply) -> Result<(Friend, Friend), Error> {
        let now = chrono::Utc::now().timestamp_millis();
        let mut transaction = self.pool.begin().await?;
        let friendship: Friendship = sqlx::query_as(
            "UPDATE friendships
            SET
                status = 'Accepted',
                update_time = $1,
                resp_msg = $2,
                resp_remark = $3
            WHERE id = $4
            RETURNING *",
        )
        .bind(now)
        .bind(&fs.resp_msg)
        .bind(&fs.resp_remark)
        .bind(fs.fs_id)
        .fetch_one(&mut *transaction)
        .await?;

        // insert into friends about two friends information
        // EXCLUDED is a special keyword in postgresql stands for the value of the column that is being conflicting
        // DO UPDATE only operate the conflicting row,
        // so we can use EXCLUDED to get the value of the column that is being conflicting
        sqlx::query(
            "INSERT INTO friends (fs_id, user_id, friend_id, remark, source, create_time, update_time)
        VALUES
        ($1, $2, $3, $4, $6, $7, $7),
        ($1, $3, $2, $5, $6, $7, $7)
        ON CONFLICT (user_id, friend_id)
        DO UPDATE SET
            remark =  EXCLUDED.remark,
            status = 'Accepted',
            source = EXCLUDED.source",
        )
        .bind(&friendship.id)
        .bind(&friendship.user_id)
        .bind(&friendship.friend_id)
        .bind(&friendship.req_remark)
        .bind(&friendship.resp_remark)
        .bind(&friendship.source)
        .bind(now)
        .execute(&mut *transaction)
        .await?;

        // select user information
        let mut users: Vec<User> = sqlx::query_as("SELECT * from users WHERE id = $1 OR id = $2")
            .bind(&friendship.user_id)
            .bind(&friendship.friend_id)
            .fetch_all(&mut *transaction)
            .await?;
        transaction.commit().await?;
        let user1 = users.remove(0);
        let (user, friend) = if user1.id == friendship.user_id {
            (user1, users.remove(0))
        } else {
            (users.remove(0), user1)
        };

        // pack the friendship with user information
        let req = Friend {
            fs_id: friendship.id.clone(),
            friend_id: friendship.user_id,
            name: user.name,
            avatar: user.avatar,
            gender: user.gender,
            age: user.age,
            region: user.region,
            status: friendship.status,
            remark: friendship.resp_remark,
            source: friendship.source.clone(),
            update_time: now,
            account: user.account,
            signature: user.signature,
            create_time: friendship.update_time,
            email: user.email,
        };
        let send = Friend {
            fs_id: friendship.id,
            friend_id: friendship.friend_id,
            name: friend.name,
            avatar: friend.avatar,
            gender: friend.gender,
            age: friend.age,
            region: friend.region,
            status: friendship.status,
            remark: friendship.req_remark,
            source: friendship.source,
            update_time: now,
            account: friend.account,
            signature: friend.signature,
            create_time: friendship.update_time,
            email: friend.email,
        };
        Ok((req, send))
    }

    async fn delete_friend(&self, fs_id: &str, user_id: &str) -> Result<(), Error> {
        // update two tables friendships and friends
        // so need to use transaction
        let update_time = chrono::Utc::now().timestamp_millis();
        let mut transaction = self.pool.begin().await?;

        sqlx::query("UPDATE friends SET status = 'Deleted', update_time = $2 WHERE fs_id = $1 AND (EXISTS (SELECT 1 FROM friends WHERE fs_id = $1 AND user_id = $3))")
            .bind(fs_id)
            .bind(update_time)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query("UPDATE friendships SET status = 'Deleted', update_time = $2 WHERE id = $1")
            .bind(fs_id)
            .bind(update_time)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
        Ok(())
    }
}
