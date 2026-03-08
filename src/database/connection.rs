use rusqlite::Connection;
use std::path::Path;

use super::schema;

/// Wrapper around the SQLite connection.
///
/// This struct manages the database lifecycle:
/// - Opens or creates the database file
/// - Initializes the schema (tables) on first run
pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Opens an existing database or creates a new one at the given path.
    ///
    /// # Arguments
    /// * `path` - Path to the SQLite database file (e.g., "data/supermarket.db")
    ///
    /// # Example
    /// ```
    /// let db = Database::open("data/supermarket.db")?;
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        schema::initialize(&conn)?;
        Ok(Self { conn })
    }

    /// Creates an in-memory database (useful for testing).
    pub fn in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        schema::initialize(&conn)?;
        Ok(Self { conn })
    }
}
