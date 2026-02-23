//! SQLite database schema initialization.

use rusqlite::{Connection, Result};

/// Open (or create) the SQLite database and ensure the schema exists.
pub fn open_database(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;

    // Enable WAL mode for better concurrent read performance.
    conn.pragma_update(None, "journal_mode", "WAL")?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS weights (
            version  INTEGER PRIMARY KEY,
            data     BLOB NOT NULL,
            created  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS games (
            id       INTEGER PRIMARY KEY,
            pgn      TEXT NOT NULL,
            result   TEXT NOT NULL,
            created  TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    Ok(conn)
}
