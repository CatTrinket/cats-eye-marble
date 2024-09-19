#[derive(diesel::Queryable, diesel::Selectable, diesel::Identifiable)]
#[diesel(table_name = crate::schema::directories)]
pub struct Directory {
    pub id: i32,
    pub title: String,
    pub has_proper_title: bool,
    pub slug: String,
    pub parent_directory_id: Option<i32>,

    // Requires joining to the directory_paths view, which is fine; I always
    // want the path
    #[diesel(select_expression = crate::views::directory_paths::path)]
    #[diesel(select_expression_type = crate::views::directory_paths::path)]
    pub path: String,
}

#[derive(
    diesel::Queryable,
    diesel::Selectable,
    diesel::Identifiable,
    diesel::Associations,
)]
#[diesel(belongs_to(Directory, foreign_key=directory_id))]
#[diesel(table_name = crate::schema::posts)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub has_proper_title: bool,
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

#[derive(
    diesel::Queryable,
    diesel::Selectable,
    diesel::Identifiable,
    diesel::Associations,
)]
#[diesel(belongs_to(Post, foreign_key=post_id))]
#[diesel(table_name = crate::schema::post_images)]
pub struct PostImage {
    pub id: i32,
    pub post_id: i32,
    pub order: i32,
    pub alt_text: String,
}
