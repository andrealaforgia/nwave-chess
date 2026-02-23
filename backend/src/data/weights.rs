//! Load and save evaluation weights from the database.

use crate::engine::eval::EvalWeights;
use rusqlite::{Connection, Result};

/// Load the latest weight version from the database.
/// Returns `Ok(None)` if no weights have been saved yet.
pub fn load_latest_weights(conn: &Connection) -> Result<Option<(u32, EvalWeights)>> {
    let mut stmt = conn.prepare(
        "SELECT version, data FROM weights ORDER BY version DESC LIMIT 1",
    )?;

    let mut rows = stmt.query([])?;

    match rows.next()? {
        Some(row) => {
            let version: u32 = row.get(0)?;
            let data: Vec<u8> = row.get(1)?;
            let weight_vec: Vec<f64> = rmp_serde::from_slice(&data)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Blob,
                    Box::new(e),
                ))?;
            let weights = EvalWeights::from_vec(&weight_vec)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e))?;
            Ok(Some((version, weights)))
        }
        None => Ok(None),
    }
}

/// Save weights to the database with the given version number.
pub fn save_weights(conn: &Connection, version: u32, weights: &EvalWeights) -> Result<()> {
    let data = rmp_serde::to_vec(&weights.to_vec())
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    conn.execute(
        "INSERT OR REPLACE INTO weights (version, data) VALUES (?1, ?2)",
        rusqlite::params![version, data],
    )?;
    Ok(())
}
