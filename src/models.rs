#[derive(diesel::prelude::Queryable, diesel::prelude::Selectable)]
#[diesel(table_name = crate::schema::directories)]
pub struct Directory {
    pub id: i32,
    pub title: String,
    pub slug: String,
    pub parent_directory_id: Option<i32>,

    // Requires joining to the directory_paths view, which is fine; I always
    // want the path
    #[diesel(select_expression = crate::views::directory_paths::path)]
    #[diesel(select_expression_type = crate::views::directory_paths::path)]
    pub path: String,
}

#[derive(diesel::prelude::Queryable, diesel::prelude::Selectable)]
#[diesel(table_name = crate::schema::posts)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub slug: String,
    pub timestamp: chrono::NaiveDateTime,
    pub directory_id: i32,
    pub description: String,

    // Requires joining to the post_paths view, which is fine; I always want
    // the path
    #[diesel(select_expression = crate::views::post_paths::path)]
    #[diesel(select_expression_type = crate::views::post_paths::path)]
    pub path: String,
}
