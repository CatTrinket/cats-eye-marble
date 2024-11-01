//! A CLI for managing the contents of the Cat's Eye Marble database.

use std::default::Default;
use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;

use clap::Parser as _;
use diesel::prelude::*;

use cem::db;

/// A CLI for managing the contents of the Cat's Eye Marble database.
#[derive(Debug, clap::Parser)]
#[command(about)]
struct CLI {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Create a new post.
    PostNew,
    /// Edit an existing post.
    PostEdit { path: String },
}

/// A bundle of arguments that need to get passed around everywhere in the
/// course of editing or creating a post.
struct PostContext<'a> {
    post_id: Option<i32>,
    connection: &'a mut diesel::PgConnection,
    config: cem::CEMConfig,
}

/// A post, as edited in TOML form (as part of EditPostWithFiles)
#[derive(
    Default,
    diesel::Queryable,
    diesel::Selectable,
    serde::Deserialize,
    serde::Serialize,
)]
#[diesel(table_name = db::posts)]
struct EditPost {
    #[diesel(
        select_expression = db::post_paths::path,
        select_expression_type = db::post_paths::path
    )]
    path: String,
    title: String,
    has_proper_title: bool,
    #[serde(
        serialize_with = "chrono_to_toml",
        deserialize_with = "toml_to_chrono",
        default
    )]
    #[diesel(
        select_expression = db::posts::timestamp.nullable(),
        select_expression_type = diesel::dsl::Nullable<db::posts::timestamp>
    )]
    pub timestamp: Option<chrono::NaiveDateTime>,
    description: String,
}

/// A post file, as edited in TOML form (as part of EditPostWithFiles)
#[derive(Default, serde::Deserialize, serde::Serialize)]
struct EditPostFile {
    local_path: Option<PathBuf>,
    alt_text: String,
}

/// A post and list of post files, as edited in TOML form
#[derive(serde::Deserialize, serde::Serialize)]
struct EditPostWithFiles {
    post: EditPost,
    files: Vec<EditPostFile>,
}

/// A post, to be saved either in an update or insert statement
#[derive(diesel::AsChangeset, diesel::Insertable)]
#[diesel(table_name = db::posts)]
struct SavePost {
    title: String,
    has_proper_title: bool,
    slug: String,
    timestamp: Option<chrono::NaiveDateTime>,
    description: String,
    directory_id: i32,
}

/// A post file, to be saved in an insert statement.
///
/// Existing post files don't need to be updated; they're all cleared out and
/// reinserted every time.
#[derive(diesel::Insertable)]
#[diesel(table_name = db::post_images)]
struct SavePostFile {
    post_id: i32,
    order: i32,
    alt_text: String,
}

/// Serialize a chrono datetime as a toml datetime.
fn chrono_to_toml<S: serde::Serializer>(
    timestamp: &Option<chrono::NaiveDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match timestamp {
        Some(timestamp) => {
            let toml_dt: toml::value::Datetime =
                timestamp.to_string().parse().unwrap();
            serializer.serialize_some(&toml_dt)
        }
        None => serializer.serialize_none(),
    }
}

/// Deserialize a toml datetime into a chrono datetime.
fn toml_to_chrono<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<chrono::NaiveDateTime>, D::Error> {
    let toml_dt: Option<toml::value::Datetime> =
        serde::Deserialize::deserialize(deserializer)?;

    match toml_dt {
        Some(timestamp) => {
            let time: chrono::DateTime<chrono::Utc> =
                timestamp.to_string().parse().unwrap();
            Ok(Some(time.naive_utc()))
        }
        None => Ok(None),
    }
}

/// Open the given string in a text editor, call the given save function on the
/// edited result, and repeat if there's an error.
fn open_in_editor<T>(
    mut input: String,
    context: &mut T,
    save: fn(&str, &mut T) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    loop {
        input = edit::edit_with_builder(
            input,
            edit::Builder::new().suffix(".toml"),
        )?;
        let result = save(input.as_str(), context);

        match result {
            Ok(()) => return Ok(()),
            Err(error) => {
                let error = error.to_string();

                // Ask "Continue editing?" until we get either y or n
                eprintln!("{}", error);

                let stdin = std::io::stdin();
                let mut response = String::new();
                loop {
                    eprint!("Continue editing? (y/n): ");
                    response.clear();
                    stdin.read_line(&mut response)?;

                    match response.trim() {
                        "y" => break,
                        "n" => return Err("Exiting at user request".into()),
                        _ => {}
                    }
                }

                // Append error to toml as comment before re-editing,
                // overwriting any previous error (assuming ### ERROR ###
                // will never legitimately appear in a string or anything)
                const ERROR_HEADER: &'static str = "### ERROR ###\n";

                if let Some(index) = input.find(ERROR_HEADER) {
                    input.truncate(index);
                }

                input.push_str(ERROR_HEADER);
                for line in error.lines() {
                    writeln!(input, "# {}", line)?;
                }
            }
        }
    }
}

/// Create thumbnails in various sizes from the given source image.
fn create_thumbnails(
    thumbnails_dir: &PathBuf,
    image_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    for height in [100, 200, 300, 400, 1080] {
        let source = image_path.display().to_string();
        let dest =
            thumbnails_dir.join(format!("{height}.png")).display().to_string();

        #[rustfmt::skip]
        let result = std::process::Command::new("convert")
            .args([
                &source,
                // Some colours will be off by one after the gamma roundtrip.
                // 0.456 is slightly higher than 1/2.2 but seems to result in
                // the least error I can get out of ImageMagick's rounding.
                "-gamma", "0.456",
                "-filter", "box",
                "-scale", &format!("x{height}"),
                "-gamma", "2.2",
                &dest
            ])
            .output()?;

        if !result.status.success() {
            return Err(String::from_utf8_lossy(&result.stderr).into());
        }

        let result = std::process::Command::new("optipng")
            .args(["-strip", "all", &dest])
            .output()?;

        if !result.status.success() {
            return Err(String::from_utf8_lossy(&result.stderr).into());
        }
    }

    Ok(())
}

/// Copy post files into the right places and generate thumbnails; return the
/// filesystem path of the directory containing these files.
fn handle_files(
    files: &[EditPostFile],
    context: &mut PostContext,
) -> Result<PathBuf, Box<dyn Error>> {
    // Create directories
    let id_string = match context.post_id {
        Some(id) => id.to_string(),
        None => "tmp".to_string(),
    };

    let post_dir = context.config.upload_dir.join(id_string);
    let files_dir = post_dir.join("files");
    let thumbnails_dir = post_dir.join("thumbnails");

    std::fs::create_dir_all(&files_dir)?;
    std::fs::create_dir_all(&thumbnails_dir)?;

    // Process files
    for (i, file) in (1..).zip(files.iter()) {
        if let Some(path) = file.local_path.as_ref() {
            let filename = files_dir.join(format!("{i}.png"));
            std::fs::copy(&path, &filename)?;

            if i == 1 {
                create_thumbnails(&thumbnails_dir, &filename)?;
            }
        }
    }

    Ok(post_dir)
}

/// Resolve a post or directory's URL path to the parent directory ID and slug.
fn find_parent_id(
    path: &str,
    connection: &mut diesel::PgConnection,
) -> Result<(i32, String), Box<dyn Error>> {
    let mut split_path = path.rsplitn(2, '/');
    let Some(slug) = split_path.next() else {
        return Err("Invalid post path".into());
    };
    let Some(dir_path) = split_path.next() else {
        return Err("Invalid post path".into());
    };

    let dir_id = db::directory_paths::table
        .filter(db::directory_paths::path.eq(dir_path))
        .select(db::directory_paths::directory_id)
        .first(connection)
        .optional()?;

    let Some(dir_id) = dir_id else {
        return Err(format!("Directory not found: {dir_path}").into());
    };

    Ok((dir_id, slug.to_string()))
}

/// Save a post to the database.
fn save_post_db(
    connection: &mut diesel::PgConnection,
    bundle: EditPostWithFiles,
    post_id: Option<i32>,
    directory_id: i32,
    slug: String,
) -> Result<i32, Box<dyn Error>> {
    // Save post
    let new_post = SavePost {
        title: bundle.post.title,
        has_proper_title: bundle.post.has_proper_title,
        slug: slug,
        timestamp: bundle.post.timestamp,
        description: bundle.post.description,
        directory_id: directory_id,
    };

    let id = match post_id {
        // Update post
        Some(id) => {
            diesel::update(db::posts::table)
                .filter(db::posts::id.eq(id))
                .set(&new_post)
                .execute(connection)?;

            // Delete file rows so we can reinsert them
            diesel::delete(db::post_images::table)
                .filter(db::post_images::post_id.eq(id))
                .execute(connection)?;

            id
        }

        // Insert new post
        None => diesel::insert_into(db::posts::table)
            .values(&new_post)
            .returning(db::posts::id)
            .get_result(connection)?,
    };

    diesel::sql_query("refresh materialized view post_paths;")
        .execute(connection)?;

    // Save files
    let new_files: Vec<SavePostFile> = (1..)
        .zip(bundle.files.into_iter())
        .map(|(i, file)| SavePostFile {
            post_id: id,
            order: i,
            alt_text: file.alt_text,
        })
        .collect();

    diesel::insert_into(db::post_images::table)
        .values(new_files)
        .execute(connection)?;

    Ok(id)
}

/// Save a post, either edited or new, and add any files and thumbnails.
fn save_post(
    input: &str,
    context: &mut PostContext,
) -> Result<(), Box<dyn Error>> {
    // These steps are ordered based on what should error out first

    // Parse edited toml form
    let bundle: EditPostWithFiles = toml::from_str(input)?;

    // Confirm directory ID before making any changes
    let (directory_id, slug) =
        find_parent_id(&bundle.post.path, context.connection)?;

    // Move files around *before* db stuff so we can avoid eating a post ID if
    // there's an error here

    // XXX: When editing a post, we might mess with the existing files and then
    // end up bailing.  Doing this last wouldn't fully avoid that, either.  I
    // can fix it manually if it happens I guess.
    let files_dir = handle_files(&bundle.files, context)?;

    // Save everything to db
    let new_id = context.connection.transaction(|connection| {
        save_post_db(connection, bundle, context.post_id, directory_id, slug)
    })?;

    // Rename file directory if we didn't have the post ID before
    if context.post_id.is_none() {
        std::fs::rename(
            &files_dir,
            &files_dir.with_file_name(new_id.to_string()),
        )?;
    }

    Ok(())
}

/// Create a new post.
fn new_post(
    connection: &mut diesel::PgConnection,
    config: cem::CEMConfig,
) -> Result<(), Box<dyn Error>> {
    let empty_post = EditPostWithFiles {
        post: Default::default(),
        files: vec![EditPostFile {
            local_path: Some("".into()),
            ..Default::default()
        }],
    };
    let mut context =
        PostContext { post_id: None, connection: connection, config: config };

    open_in_editor(toml::to_string(&empty_post)?, &mut context, save_post)
}

/// Edit an existing post.
fn edit_post(
    connection: &mut diesel::PgConnection,
    path: String,
    config: cem::CEMConfig,
) -> Result<(), Box<dyn Error>> {
    let (id, post): (i32, EditPost) = db::posts::table
        .inner_join(db::post_paths::table)
        .filter(db::post_paths::path.eq(path))
        .select((db::posts::id, EditPost::as_select()))
        .first(connection)?;

    let files = db::post_images::table
        .filter(db::post_images::post_id.eq(id))
        .order(db::post_images::order)
        .select(db::post_images::alt_text)
        .load(connection)?
        .into_iter()
        .map(|text| EditPostFile { alt_text: text, local_path: None })
        .collect();

    let bundle = EditPostWithFiles { post: post, files: files };
    let mut context = PostContext {
        post_id: Some(id),
        connection: connection,
        config: config,
    };

    open_in_editor(toml::to_string(&bundle)?, &mut context, save_post)
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = CLI::parse();
    let config = rocket::Config::figment();
    let cem_config: cem::CEMConfig = config.extract_inner("cem")?;
    let db_url: String = config.extract_inner("databases.cem.url")?;
    let mut connection = diesel::PgConnection::establish(&db_url)?;

    match cli.command {
        Command::PostNew => new_post(&mut connection, cem_config),
        Command::PostEdit { path } => {
            edit_post(&mut connection, path, cem_config)
        }
    }
}
