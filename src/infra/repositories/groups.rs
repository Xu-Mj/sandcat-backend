use crate::handlers::groups::GroupRequest;
use crate::infra::db::schema::groups;
use crate::infra::errors::{adapt_infra_error, InfraError};
use deadpool_diesel::postgres::Pool;
use diesel::{Insertable, Queryable, RunQueryDsl, Selectable};
use nanoid::nanoid;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = groups)]
// 开启编译期字段检查，主要检查字段类型、数量是否匹配，可选
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GroupDb {
    pub id: String,
    pub owner: String,
    pub name: String,
    pub members: String,
    pub avatar: String,
    pub description: String,
    pub announcement: String,
    pub create_time: chrono::NaiveDateTime,
}

impl From<GroupRequest> for GroupDb {
    fn from(value: GroupRequest) -> Self {
        GroupDb {
            id: nanoid!(),
            owner: value.owner,
            name: value.group_name,
            members: value.members_id.join(","),
            avatar: value.avatar.join(","),
            description: String::new(),
            announcement: String::new(),
            create_time: chrono::Local::now().naive_local(),
        }
    }
}

/// create a new record
pub async fn create_group(pool: &Pool, group: GroupDb) -> Result<GroupDb, InfraError> {
    let conn = pool
        .get()
        .await
        .map_err(|err| InfraError::InternalServerError(err.to_string()))?;
    let group = conn
        .interact(move |conn| {
            diesel::insert_into(groups::table)
                .values(&group)
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(group)
}
