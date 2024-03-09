// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "friend_request_status"))]
    pub struct FriendRequestStatus;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::FriendRequestStatus;

    friends (id) {
        id -> Varchar,
        friendship_id -> Varchar,
        user_id -> Varchar,
        friend_id -> Varchar,
        status -> FriendRequestStatus,
        remark -> Nullable<Varchar>,
        hello -> Nullable<Varchar>,
        source -> Nullable<Varchar>,
        create_time -> Timestamp,
        update_time -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::FriendRequestStatus;

    friendships (id) {
        id -> Varchar,
        user_id -> Varchar,
        friend_id -> Varchar,
        status -> FriendRequestStatus,
        apply_msg -> Nullable<Varchar>,
        req_remark -> Nullable<Varchar>,
        response_msg -> Nullable<Varchar>,
        res_remark -> Nullable<Varchar>,
        source -> Nullable<Varchar>,
        is_delivered -> Bool,
        create_time -> Timestamp,
        update_time -> Timestamp,
    }
}

diesel::table! {
    group_members (id) {
        id -> Int8,
        group_id -> Varchar,
        user_id -> Varchar,
        #[max_length = 128]
        group_name -> Nullable<Varchar>,
        #[max_length = 128]
        group_remark -> Nullable<Varchar>,
        delivered -> Bool,
        joined_at -> Timestamp,
    }
}

diesel::table! {
    groups (id) {
        id -> Varchar,
        #[max_length = 256]
        owner -> Varchar,
        #[max_length = 256]
        name -> Varchar,
        avatar -> Text,
        description -> Text,
        announcement -> Text,
        create_time -> Timestamp,
        update_time -> Timestamp,
    }
}

diesel::table! {
    messages (msg_id) {
        msg_id -> Varchar,
        content -> Varchar,
        content_type -> Varchar,
        send_id -> Varchar,
        friend_id -> Varchar,
        is_read -> Bool,
        delivered -> Bool,
        create_time -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Varchar,
        name -> Varchar,
        account -> Varchar,
        password -> Varchar,
        is_online -> Bool,
        avatar -> Varchar,
        gender -> Varchar,
        age -> Int4,
        #[max_length = 20]
        phone -> Nullable<Varchar>,
        #[max_length = 64]
        email -> Nullable<Varchar>,
        #[max_length = 1024]
        address -> Nullable<Varchar>,
        #[max_length = 1024]
        region -> Nullable<Varchar>,
        birthday -> Nullable<Timestamp>,
        create_time -> Timestamp,
        update_time -> Timestamp,
        is_delete -> Bool,
    }
}

diesel::joinable!(group_members -> groups (group_id));
diesel::joinable!(group_members -> users (user_id));
diesel::joinable!(groups -> users (owner));

diesel::allow_tables_to_appear_in_same_query!(
    friends,
    friendships,
    group_members,
    groups,
    messages,
    users,
);
