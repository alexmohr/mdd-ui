/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

pub mod reader;

use anyhow::{Context, Result};
use cda_database::{DatabaseConfig, datatypes::DiagnosticDatabase, load_ecudata};
use cda_interfaces::datatypes::FlatbBufConfig;
// Re-export commonly used items
pub use reader::{extract_data, get_ecu_summary};

/// Load an MDD file and return a `DiagnosticDatabase`.
///
/// # Errors
///
/// Returns an error if the file cannot be read or the database cannot be parsed.
pub fn load_mdd(path: &str) -> Result<DiagnosticDatabase> {
    load_mdd_with_config(path, DatabaseConfig::default())
}

/// Load an MDD file with protocol-agnostic lookups enabled.
///
/// This is used by the UDS translator where the exact protocol name recorded
/// in the database may not match the one requested at runtime (e.g.
/// `UDS_Ethernet_DoIP` missing).  With `ignore_protocol: true` the CDA falls
/// back to the single available protocol.
///
/// When the database contains multiple distinct protocols, `ignore_protocol`
/// is not allowed (ambiguous resolution).  In that case we fall back to a
/// normal load — the caller is expected to select the protocol explicitly
/// via the `EcuManager`.
///
/// # Errors
///
/// Returns an error if the file cannot be read or the database cannot be parsed.
pub fn load_mdd_ignore_protocol(path: &str) -> Result<DiagnosticDatabase> {
    load_mdd_with_config(
        path,
        DatabaseConfig {
            ignore_protocol: true,
            ..Default::default()
        },
    )
    .or_else(|_| load_mdd(path))
}

fn load_mdd_with_config(path: &str, db_config: DatabaseConfig) -> Result<DiagnosticDatabase> {
    let (ecu_name, blob) = load_ecudata(path)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("Failed to load MDD file: {path}"))?;

    let config = FlatbBufConfig::default();

    let db = DiagnosticDatabase::new_from_bytes(path.to_owned(), blob, config, db_config)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("Failed to parse database for ECU: {ecu_name}"))?;

    Ok(db)
}
