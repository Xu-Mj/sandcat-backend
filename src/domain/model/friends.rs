use chrono::TimeZone;

// 朋友模型，对应friends表，提供对朋友的增删改查
// 好友请求，同意请求，拒绝等

pub struct Friend {
    id: i32,
    user_id: i32,
    friend_id: i32,
    status: i32,
    created_time: i64,
    updated_time: i64,
}
impl Friend {
    pub fn new(user_id: i32, friend_id: i32, status: i32) -> Friend {
        Friend {
            id: 0,
            user_id,
            friend_id,
            status,
            created_time: chrono::Local::now().timestamp_millis(),
            updated_time: chrono::Local::now().timestamp_millis(),
        }
    }
    pub fn get_id(&self) -> i32 {
        self.id
    }
}

// 提供sql接口
impl Friend {
    // pub fn get_by_id(id: i32) -> Self {
    //
    // }
}
