mod models;
mod schema;
mod views;

pub use self::models::*;
pub use self::schema::*;
pub use self::views::*;

/// Allows us to get a database connection as a request guard; see
/// `rocket_db_pools`.
#[derive(rocket_db_pools::Database)]
#[database("cem")]
pub struct CEMDB(rocket_db_pools::diesel::PgPool);
