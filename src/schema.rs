// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Int4,
        name -> Nullable<Varchar>,
        account -> Varchar,
        avatar -> Varchar,
        gender -> Varchar,
        #[max_length = 20]
        phone -> Varchar,
        #[max_length = 64]
        email -> Varchar,
        #[max_length = 1024]
        address -> Varchar,
        birthday -> Timestamp,
        create_time -> Timestamp,
        update_time -> Timestamp,
        is_delete -> Bool,
    }
}
