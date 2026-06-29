diesel::table! {
    servers (id) {
        id -> Integer,
        name -> Text,
        host -> Text,
        port -> Integer,
        username -> Text,
        authentication -> Text,
        password -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}
