pub mod models;
pub mod schema;
pub mod views;

use rocket_db_pools::diesel::prelude::*;
use rocket_db_pools::Database as _;

use self::models::{Directory, Post};
use self::schema::{directories, posts};
use self::views::{directory_paths, post_paths};

#[derive(rocket_db_pools::Database)]
#[database("cem")]
struct CEMDB(rocket_db_pools::diesel::PgPool);

struct Breadcrumb {
    path: String,
    label: String,
}

#[derive(askama::Template)]
#[template(path = "hello.html")]
struct HelloTemplate {
    world: String,
}

#[derive(askama::Template)]
#[template(path = "post.html")]
struct PostTemplate {
    breadcrumbs: Vec<Breadcrumb>,
    post: Post,
    prev_post: Option<Post>,
    next_post: Option<Post>,
}

fn log_error<T>(_: T) -> rocket::http::Status {
    // TODO: actually log it
    rocket::http::Status::InternalServerError
}

#[rocket::get("/")]
async fn index() -> HelloTemplate {
    HelloTemplate { world: "worlb".to_string() }
}

#[rocket::get("/<path..>", rank = 1000)]
async fn post(
    mut db: rocket_db_pools::Connection<CEMDB>,
    path: std::path::PathBuf,
) -> Result<Option<PostTemplate>, rocket::http::Status> {
    let path = format!("/{}", path.display());
    let result = posts::table
        .inner_join(post_paths::table)
        .filter(post_paths::path.eq(path))
        .select(Post::as_select())
        .first(&mut db)
        .await
        .optional()
        .map_err(log_error)?;

    let Some(post) = result else { return Ok(None) };

    let mut parent_id = Some(post.directory_id);
    let mut breadcrumbs = Vec::new();

    while let Some(id) = parent_id {
        let directory = directories::table
            .find(id)
            .inner_join(directory_paths::table)
            .select(Directory::as_select())
            .first(&mut db)
            .await
            .map_err(log_error)?;

        breadcrumbs.push(Breadcrumb {
            path: directory.path,
            label: directory.title,
        });

        parent_id = directory.parent_directory_id;
    }

    let breadcrumbs = breadcrumbs.into_iter().rev().collect();

    let next_post = posts::table
        .inner_join(post_paths::table)
        .filter(posts::timestamp.gt(post.timestamp))
        .order(posts::timestamp.asc())
        .select(Post::as_select())
        .first(&mut db)
        .await
        .optional()
        .map_err(log_error)?;

    let prev_post = posts::table
        .inner_join(post_paths::table)
        .filter(posts::timestamp.lt(post.timestamp))
        .order(posts::timestamp.desc())
        .select(Post::as_select())
        .first(&mut db)
        .await
        .optional()
        .map_err(log_error)?;

    Ok(Some(PostTemplate {
        breadcrumbs: breadcrumbs,
        post: post,
        prev_post: prev_post,
        next_post: next_post,
    }))
}

#[rocket::launch]
fn rocket() -> _ {
    rocket::build()
        .attach(CEMDB::init())
        .mount("/", rocket::routes![index, post])
        .mount(
            "/static",
            rocket::fs::FileServer::from("static"),
        )
}
