// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Int4,
        name -> Varchar,
        account -> Varchar,
        avatar -> Varchar,
        gender -> Varchar,
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
