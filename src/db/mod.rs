/// Database module for persistent storage of vocabulary records.
/// This module handles SQLite initialization, schema management, and provides
/// the persistence layer for the application.

pub mod merge;
pub mod schema;

use rusqlite::Connection;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during database operations.
#[derive(Debug, Error)]
pub enum DbError {
    /// SQLite connection or query execution error.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Parser error (JSON/CSV syntax or validation).
    #[error("parse error: {0}")]
    Parse(#[from] crate::parser::ParseError),

    /// Error during data merge operations.
    #[error("merge error: {0}")]
    Merge(String),

    /// Data validation error.
    #[error("validation error: {0}")]
    Validation(String),
}

/// Initialize and return a SQLite database connection.
/// Creates the database file if it doesn't exist and sets up the schema.
///
/// # Parameters
/// - `db_path`: Path to the SQLite database file (will be created if missing)
///
/// # Returns
/// - `Ok(Connection)`: Ready-to-use database connection
/// - `Err(DbError)`: Connection or schema initialization failed
///
/// # Example
/// ```ignore
/// let db = init_db("./data.db")?;
/// // Now ready to use with repository
/// ```
pub fn init_db<P: AsRef<Path>>(db_path: P) -> Result<Connection, DbError> {
    let conn = Connection::open(db_path)?;

    // Enable foreign key constraints for referential integrity
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Initialize schema (create tables if not exist)
    schema::init_schema(&conn)?;

    Ok(conn)
}

/// Initialize the database schema by creating all required tables.
/// This function is idempotent—calling it multiple times is safe.
///
/// # Schema Overview
/// - `words`: Main table storing core word information (word, phonetic, timestamps, review count)
/// - `definitions`: Normalized table for word meanings (supports multiple definitions per word)
/// - `examples`: Examples of word usage (supports multiple examples per word)
/// - `tags`: Dictionary of available tags
/// - `word_tags`: Junction table linking words to tags (many-to-many relationship)
///
/// # Parameters
/// - `conn`: Active SQLite connection
///
/// # Returns
/// - `Ok(())`: Schema initialized successfully
/// - `Err(DbError)`: Table creation failed
pub fn init_memory_db() -> Result<Connection, DbError> {
    let conn = Connection::open_in_memory()?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    schema::init_schema(&conn)?;
    Ok(conn)
}


