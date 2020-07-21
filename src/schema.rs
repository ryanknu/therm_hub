table! {
    ecobee_token (id) {
        id -> Int4,
        access_token -> Varchar,
        refresh_token -> Varchar,
        expires -> Date,
    }
}
