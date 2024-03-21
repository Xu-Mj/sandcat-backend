use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub members_id: Vec<String>,
    pub create_time: chrono::NaiveDateTime,
    pub publish_msg: String,
}
