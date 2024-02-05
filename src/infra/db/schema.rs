// @generated automatically by Diesel CLI.

diesel::table! {
    friends (id) {
        id -> Varchar,
        user_id -> Varchar,
        friend_id -> Varchar,
        #[max_length = 1]
        status -> Bpchar,
        remark -> Nullable<Varchar>,
        source -> Nullable<Varchar>,
        create_time -> Timestamp,
        update_time -> Timestamp,
    }
}

diesel::table! {
    friendships (id) {
        id -> Varchar,
        user_id -> Varchar,
        friend_id -> Varchar,
        #[max_length = 1]
        status -> Bpchar,
        apply_msg -> Nullable<Varchar>,
        source -> Nullable<Varchar>,
        is_delivered -> Bool,
        create_time -> Timestamp,
        update_time -> Timestamp,
    }
}

diesel::table! {
    messages (msg_id) {
        msg_id -> Varchar,
        msg_type -> Varchar,
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
        avatar -> Varchar,
        gender -> Varchar,
        age -> Int4,
        #[max_length = 20]
        phone -> Nullable<Varchar>,
        #[max_length = 64]
        email -> Nullable<Varchar>,
        #[max_length = 1024]
        address -> Nullable<Varchar>,
        birthday -> Nullable<Timestamp>,
        create_time -> Timestamp,
        update_time -> Timestamp,
        is_delete -> Bool,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    friends,
    friendships,
    messages,
    users,
);
