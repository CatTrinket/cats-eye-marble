pub mod db;

/// Config specific to Cat's Eye Marble.
///
/// These values are taken from the `cem` table in Rocket.toml.
#[derive(Debug, rocket::serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CEMConfig {
    pub upload_dir: std::path::PathBuf,
    pub base_url: String,
}
