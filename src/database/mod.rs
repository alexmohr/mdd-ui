// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

pub mod reader;

use anyhow::{Context, Result};
use cda_database::{datatypes::DiagnosticDatabase, load_ecudata};
use cda_interfaces::datatypes::FlatbBufConfig;
// Re-export commonly used items
pub use reader::{extract_data, get_ecu_summary};

/// Load an MDD file and return a `DiagnosticDatabase`.
pub fn load_mdd(path: &str) -> Result<DiagnosticDatabase> {
    let (ecu_name, blob) = load_ecudata(path)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("Failed to load MDD file: {path}"))?;

    let config = FlatbBufConfig::default();

    let db = DiagnosticDatabase::new(path.to_owned(), blob, config)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("Failed to parse database for ECU: {ecu_name}"))?;

    Ok(db)
}
