diesel::table! {
    app_settings (id) {
        id -> Text,
        settings_data -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

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

diesel::allow_tables_to_appear_in_same_query!(app_settings, servers,);
