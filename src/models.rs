//! Structs that database query results are mapped to.

/// A directory containing posts and subdirectories.
#[derive(diesel::Queryable, diesel::Selectable, diesel::Identifiable)]
#[diesel(table_name = crate::schema::directories)]
pub struct Directory {
    pub id: i32,
    pub title: String,
    /// If true, the title of this directory should be italicized
    pub has_proper_title: bool,
    /// This directory's title as it appears in the URL, lowercase and
    /// dash-separated
    pub slug: String,
    pub parent_directory_id: Option<i32>,

    /// The full path for this directory, including all parent directories
    // Requires joining to the directory_paths view, which is fine; I always
    // want the path
    #[diesel(select_expression = crate::views::directory_paths::path)]
    #[diesel(select_expression_type = crate::views::directory_paths::path)]
    pub path: String,
}

/// A post.
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
    /// If true, the title of this post should be italicized
    pub has_proper_title: bool,
    /// This post's title as it appears in the URL, lowercase and
    /// dash-separated
    pub slug: String,
    /// The time this post was made, which may be backdated to the time it was
    /// originally made on another site
    pub timestamp: chrono::NaiveDateTime,
    pub directory_id: i32,
    pub description: String,

    /// The full path for this post, including all parent directories
    // Requires joining to the post_paths view, which is fine; I always want
    // the path
    #[diesel(select_expression = crate::views::post_paths::path)]
    #[diesel(select_expression_type = crate::views::post_paths::path)]
    pub path: String,
}

/// A file attached to a post.
///
/// TODO: I plan to rename this to `PostFile`, since I may want to post audio
/// or video or attach other files to blog posts.  But for now the name is
/// `PostImage` and it is in fact just images.
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
