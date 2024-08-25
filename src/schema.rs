// @generated automatically by Diesel CLI.

diesel::table! {
    directories (id) {
        id -> Int4,
        title -> Text,
        slug -> Text,
        parent_directory_id -> Nullable<Int4>,
    }
}

diesel::table! {
    post_images (id) {
        id -> Int4,
        post_id -> Int4,
        order -> Int4,
        alt_text -> Text,
    }
}

diesel::table! {
    posts (id) {
        id -> Int4,
        title -> Text,
        slug -> Text,
        timestamp -> Timestamp,
        directory_id -> Int4,
        description -> Text,
    }
}

diesel::joinable!(post_images -> posts (post_id));
diesel::joinable!(posts -> directories (directory_id));

diesel::allow_tables_to_appear_in_same_query!(
    directories,
    post_images,
    posts,
);
