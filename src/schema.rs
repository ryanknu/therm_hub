table! {
    ecobee_token (id) {
        id -> Int4,
        access_token -> Varchar,
        refresh_token -> Varchar,
        expires -> Timestamp,
    }
}

table! {
    thermostats (id) {
        id -> Int4,
        name -> Varchar,
        time -> Timestamp,
        is_hygrostat -> Bool,
        temperature -> Int4,
        relative_humidity -> Int4,
    }
}

allow_tables_to_appear_in_same_query!(ecobee_token, thermostats,);
