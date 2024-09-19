pub mod models;
pub mod schema;
pub mod views;

use rocket_db_pools::diesel::prelude::*;
use rocket_db_pools::Database as _;

use self::models::{Directory, Post, PostImage};
use self::schema::{directories, post_images, posts};
use self::views::{directory_paths, post_paths};

#[derive(rocket_db_pools::Database)]
#[database("cem")]
struct CEMDB(rocket_db_pools::diesel::PgPool);

#[derive(rocket::serde::Deserialize)]
#[serde(crate = "rocket::serde")]
struct CEMConfig {
    upload_dir: std::path::PathBuf,
    base_url: String,
}

/// A cachebust timestamp appended to the URL of static files.
static CACHEBUST: std::sync::LazyLock<i64> =
    std::sync::LazyLock::new(|| chrono::Utc::now().timestamp());

/// Return the cachebust timestamp.
///
/// This wrapper function is necessary as Askama doesn't seem to like *.
fn cachebust() -> i64 {
    *CACHEBUST
}

/// Midnight on the date the site launched (2024-08-25).
///
/// Anything dated earlier is labelled "Originally posted on..." and doesn't
/// show up in the Atom feed.
const SITE_LAUNCH: chrono::naive::NaiveDateTime = {
    // expect/unwrap in const aren't available yet at the time of writing
    let Some(date) = chrono::naive::NaiveDate::from_ymd_opt(2024, 8, 25)
    else {
        panic!("Expected launch date")
    };
    let Some(datetime) = date.and_hms_opt(0, 0, 0) else {
        panic!("Expected launch datetime")
    };
    datetime
};

struct Breadcrumb {
    path: String,
    label: String,
}

#[derive(askama::Template)]
#[template(path = "hello.html")]
struct HelloTemplate {}

/// The Atom feed template.
#[derive(askama::Template)]
#[template(path = "feed.xml")]
struct FeedTemplate {
    posts: Vec<Post>,
    files: Vec<Vec<PostImage>>,
    base_url: String,
    domain: String,
}

/// A wrapper around the Atom template to set the Content-Type.
#[derive(rocket::Responder)]
#[response(content_type = "application/atom+xml")]
struct FeedResponse {
    template: FeedTemplate,
}

#[derive(askama::Template)]
#[template(path = "post.html")]
struct PostTemplate {
    breadcrumbs: Vec<Breadcrumb>,
    post: Post,
    files: Vec<PostImage>,
    prev_post: Option<Post>,
    next_post: Option<Post>,
}

#[derive(askama::Template)]
#[template(path = "directory.html")]
struct DirectoryTemplate {
    breadcrumbs: Vec<Breadcrumb>,
    directory: Directory,
    posts: Vec<Post>,
    subdirs: Vec<Directory>,
}

#[derive(rocket::Responder)]
enum PathResponse {
    File(rocket::fs::NamedFile),
    Post(PostTemplate),
    Directory(DirectoryTemplate),
}

fn log_error<T>(_: T) -> rocket::http::Status {
    // TODO: actually log it
    rocket::http::Status::InternalServerError
}

#[rocket::get("/")]
async fn index() -> HelloTemplate {
    HelloTemplate {}
}

/// Serve the Atom feed.
#[rocket::get("/feed.xml")]
async fn feed(
    mut db: rocket_db_pools::Connection<CEMDB>,
    config: &rocket::State<CEMConfig>,
) -> Result<FeedResponse, rocket::http::Status> {
    let posts = posts::table
        .inner_join(post_paths::table)
        .filter(posts::timestamp.ge(SITE_LAUNCH))
        .order(posts::timestamp)
        .select(Post::as_select())
        .load(&mut db)
        .await
        .map_err(log_error)?;

    let files = PostImage::belonging_to(&posts)
        .order(post_images::order)
        .select(PostImage::as_select())
        .load(&mut db)
        .await
        .map_err(log_error)?
        .grouped_by(&posts);

    let domain = rocket::http::uri::Absolute::parse(&config.base_url)
        .expect("Expected valid base URL")
        .authority()
        .expect("Expected base URL authority")
        .host()
        .to_string();

    Ok(FeedResponse {
        template: FeedTemplate {
            posts: posts,
            files: files,
            base_url: config.base_url.clone(),
            domain: domain,
        },
    })
}

/// Respond to anything involving an arbitrary path.
///
/// At the time of writing, Rocket only lets you have a multi-segment parameter
/// at the end of the path.  TODO: look into request guards instead
#[rocket::get("/<path..>?<height>", rank = 1000)]
async fn path(
    mut db: rocket_db_pools::Connection<CEMDB>,
    path: std::path::PathBuf,
    height: Option<i32>,
    config: &rocket::State<CEMConfig>,
) -> Result<Option<PathResponse>, rocket::http::Status> {
    // Tried to write this with .or_else but couldn't figure it out with async
    if let Some(file) = file(&mut db, &path, &config.upload_dir).await? {
        // First because it might not even hit the db
        Ok(Some(PathResponse::File(file)))
    } else if let Some(thumbnail) =
        thumbnail(&mut db, &path, height, &config.upload_dir).await?
    {
        Ok(Some(PathResponse::File(thumbnail)))
    } else if let Some(post) = post(&mut db, &path).await? {
        Ok(Some(PathResponse::Post(post)))
    } else if let Some(directory) = directory(&mut db, &path).await? {
        Ok(Some(PathResponse::Directory(directory)))
    } else {
        Ok(None)
    }
}

/// Parse a URL path for an individual post file.
///
/// e.g. `PathBuf::from("some/post/files/123")` -> `Some(("/some/post", 123))`
/// (note that it adds the leading slash.)
fn parse_file_path(path: &std::path::PathBuf) -> Option<(String, i32)> {
    let num = path.file_name()?.to_str()?.parse().ok()?;

    let path = path.parent()?;
    if path.file_name()?.to_str()? != "files" {
        return None;
    };

    let path = format!("/{}", path.parent()?.display());

    Some((path, num))
}

async fn file(
    db: &mut rocket_db_pools::Connection<CEMDB>,
    path: &std::path::PathBuf,
    upload_dir: &std::path::PathBuf,
) -> Result<Option<rocket::fs::NamedFile>, rocket::http::Status> {
    let Some((path, num)) = parse_file_path(path) else { return Ok(None) };

    let result = posts::table
        .inner_join(post_images::table)
        .inner_join(post_paths::table)
        .filter(post_paths::path.eq(path))
        .filter(post_images::order.eq(num))
        .select((PostImage::as_select(), Post::as_select()))
        .first(db)
        .await
        .optional()
        .map_err(log_error)?;
    let Some((image, post)) = result else { return Ok(None) };

    // Temporarily hardcoding this just to get this out the door
    let suffix = match (post.slug.as_str(), image.order) {
        ("a-bubble-blower-very-cool", 2) => "jpg",
        _ => "png",
    };

    let local_path = upload_dir
        .join(format!("{}/files/{}.{}", post.id, image.order, suffix));
    let file =
        rocket::fs::NamedFile::open(local_path).await.map_err(log_error)?;

    Ok(Some(file))
}

async fn thumbnail(
    db: &mut rocket_db_pools::Connection<CEMDB>,
    path: &std::path::PathBuf,
    height: Option<i32>,
    upload_dir: &std::path::PathBuf,
) -> Result<Option<rocket::fs::NamedFile>, rocket::http::Status> {
    if path.file_name().and_then(|s| s.to_str()) != Some("thumbnail") {
        return Ok(None);
    }
    let Some(path) = path.parent() else { return Ok(None) };
    let path = format!("/{}", path.display());

    let result = posts::table
        .inner_join(post_paths::table)
        .filter(post_paths::path.eq(path))
        .select(Post::as_select())
        .first(db)
        .await
        .optional()
        .map_err(log_error)?;
    let Some(post) = result else { return Ok(None) };

    let height = height.unwrap_or(200);
    let local_path =
        upload_dir.join(format!("{}/thumbnails/{}.png", post.id, height));
    let file =
        rocket::fs::NamedFile::open(local_path).await.map_err(log_error)?;

    Ok(Some(file))
}

async fn post(
    db: &mut rocket_db_pools::Connection<CEMDB>,
    path: &std::path::PathBuf,
) -> Result<Option<PostTemplate>, rocket::http::Status> {
    let path = format!("/{}", path.display());
    let result = posts::table
        .inner_join(post_paths::table)
        .filter(post_paths::path.eq(path))
        .select(Post::as_select())
        .first(db)
        .await
        .optional()
        .map_err(log_error)?;

    let Some(post) = result else { return Ok(None) };

    let files = PostImage::belonging_to(&post)
        .order(post_images::order)
        .select(PostImage::as_select())
        .load(db)
        .await
        .map_err(log_error)?;

    let mut parent_id = Some(post.directory_id);
    let mut breadcrumbs = Vec::new();

    while let Some(id) = parent_id {
        let directory = directories::table
            .find(id)
            .inner_join(directory_paths::table)
            .select(Directory::as_select())
            .first(db)
            .await
            .map_err(log_error)?;

        breadcrumbs
            .push(Breadcrumb { path: directory.path, label: directory.title });

        parent_id = directory.parent_directory_id;
    }

    let breadcrumbs = breadcrumbs.into_iter().rev().collect();

    let next_post = posts::table
        .inner_join(post_paths::table)
        .filter(posts::timestamp.gt(post.timestamp))
        .order(posts::timestamp.asc())
        .select(Post::as_select())
        .first(db)
        .await
        .optional()
        .map_err(log_error)?;

    let prev_post = posts::table
        .inner_join(post_paths::table)
        .filter(posts::timestamp.lt(post.timestamp))
        .order(posts::timestamp.desc())
        .select(Post::as_select())
        .first(db)
        .await
        .optional()
        .map_err(log_error)?;

    Ok(Some(PostTemplate {
        breadcrumbs: breadcrumbs,
        post: post,
        files: files,
        prev_post: prev_post,
        next_post: next_post,
    }))
}

async fn directory(
    db: &mut rocket_db_pools::Connection<CEMDB>,
    path: &std::path::PathBuf,
) -> Result<Option<DirectoryTemplate>, rocket::http::Status> {
    let path = format!("/{}", path.display());
    let result = directories::table
        .inner_join(directory_paths::table)
        .filter(directory_paths::path.eq(path))
        .select(Directory::as_select())
        .first(db)
        .await
        .optional()
        .map_err(log_error)?;

    let Some(directory) = result else { return Ok(None) };

    let mut parent_id = directory.parent_directory_id;
    let mut breadcrumbs = Vec::new();

    while let Some(id) = parent_id {
        let directory = directories::table
            .find(id)
            .inner_join(directory_paths::table)
            .select(Directory::as_select())
            .first(db)
            .await
            .map_err(log_error)?;

        breadcrumbs
            .push(Breadcrumb { path: directory.path, label: directory.title });

        parent_id = directory.parent_directory_id;
    }

    let breadcrumbs = breadcrumbs.into_iter().rev().collect();

    let posts = Post::belonging_to(&directory)
        .inner_join(post_paths::table)
        .order(posts::timestamp)
        .select(Post::as_select())
        .load(db)
        .await
        .map_err(log_error)?;

    let subdirs = directories::table
        .inner_join(directory_paths::table)
        .filter(directories::parent_directory_id.eq(directory.id))
        .order(directories::title)
        .select(Directory::as_select())
        .load(db)
        .await
        .map_err(log_error)?;

    Ok(Some(DirectoryTemplate {
        breadcrumbs: breadcrumbs,
        directory: directory,
        posts: posts,
        subdirs: subdirs,
    }))
}

#[rocket::launch]
fn rocket() -> _ {
    let rocket = rocket::build();
    let config: CEMConfig =
        rocket.figment().extract_inner("cem").expect("Expected valid config");

    // Putting the cachebust in the /static/ path rather than the query means
    // CSS can use relative URLs for background images etc. and avoid needing
    // the cachebust parameter in the CSS itself.

    // TODO: the Cache-Control header for static files is currently added by
    // nginx.  Come Rocket 0.6, use FileServer's header rewrite method instead.

    rocket
        .attach(CEMDB::init())
        .manage(config)
        .mount("/", rocket::routes![index, feed, path])
        .mount(
            format!("/static/{}", *CACHEBUST),
            rocket::fs::FileServer::from("static"),
        )
}
