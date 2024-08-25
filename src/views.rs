use crate::schema::{directories, posts};

diesel::table! {
    directory_paths (directory_id) {
        directory_id -> Int4,
        path -> Text,
    }
}

diesel::table! {
    post_paths (post_id) {
        post_id -> Int4,
        path -> Text,
    }
}

diesel::joinable!(directory_paths -> directories (directory_id));
diesel::joinable!(post_paths -> posts (post_id));

// Unfortunately we have to do this for each table individually; if we try to
// associate two tables that have already been associated in schema.rs, it will
// complain
diesel::allow_tables_to_appear_in_same_query!(directory_paths, post_paths);

diesel::allow_tables_to_appear_in_same_query!(directory_paths, directories);
diesel::allow_tables_to_appear_in_same_query!(directory_paths, posts);

diesel::allow_tables_to_appear_in_same_query!(post_paths, directories);
diesel::allow_tables_to_appear_in_same_query!(post_paths, posts);
