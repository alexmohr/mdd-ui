/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

//! Diff module for comparing two MDD diagnostic databases.
//!
//! This module provides functionality to compare two MDD files and output
//! their structural and content differences.

mod output;
mod tree_differ;

use anyhow::{Context, Result};
pub use output::OutputFormat;
use tree_differ::TreeDiffer;

/// Run the diff comparison between two MDD files.
pub fn run_diff(base_file: &str, compare_file: &str, format: OutputFormat) -> Result<()> {
    eprintln!("Loading base file: {base_file}...");
    let base_db = crate::database::load_mdd(base_file)
        .with_context(|| format!("Failed to load base file: {base_file}"))?;

    eprintln!("Loading compare file: {compare_file}...");
    let compare_db = crate::database::load_mdd(compare_file)
        .with_context(|| format!("Failed to load compare file: {compare_file}"))?;

    eprintln!("Building trees...");
    let (base_nodes, base_ecu) = crate::tree::build_tree(&base_db, base_file);
    let (compare_nodes, compare_ecu) = crate::tree::build_tree(&compare_db, compare_file);

    eprintln!(
        "Comparing {} nodes from '{base_ecu}' with {} nodes from '{compare_ecu}'...",
        base_nodes.len(),
        compare_nodes.len()
    );

    let differ = TreeDiffer::new(&base_nodes, &compare_nodes);
    let diff_result = differ.compute_diff();

    output::write_diff(&diff_result, base_file, compare_file, format);

    Ok(())
}
