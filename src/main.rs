/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

#[cfg(feature = "mcp")]
mod mcp;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mdd_core::database;

#[derive(Parser)]
#[command(name = "mdd-ui", about = "Browse and compare MDD diagnostic databases")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Export diff between two MDD databases as plain text
    ExportDiff {
        /// Path to the old/reference MDD file
        old_file: String,

        /// Path to the new MDD file
        new_file: String,

        /// Output file path (prints to stdout if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Start an MCP (Model Context Protocol) server over stdio
    #[cfg(feature = "mcp")]
    Mcp,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::ExportDiff {
            old_file,
            new_file,
            output,
        } => run_export_diff(&old_file, &new_file, output.as_deref()),
        #[cfg(feature = "mcp")]
        Command::Mcp => mcp::run_mcp(),
    }
}

fn run_export_diff(old_file: &str, new_file: &str, output: Option<&str>) -> Result<()> {
    eprintln!("Loading {old_file}...");
    let db_old =
        database::load_mdd(old_file).with_context(|| format!("Failed to load: {old_file}"))?;

    eprintln!("Loading {new_file}...");
    let db_new =
        database::load_mdd(new_file).with_context(|| format!("Failed to load: {new_file}"))?;

    eprintln!("Extracting snapshots...");
    let snap_old = mdd_core::diff::snapshot::EcuSnapshot::from_database(&db_old)
        .context("Failed to extract old database snapshot")?;
    let snap_new = mdd_core::diff::snapshot::EcuSnapshot::from_database(&db_new)
        .context("Failed to extract new database snapshot")?;

    eprintln!("Comparing...");
    let diff_result = mdd_core::diff::compare::compare(&snap_old, &snap_new);

    if let Some(path) = output {
        let mut file = std::fs::File::create(path)
            .with_context(|| format!("Failed to create output file: {path}"))?;
        mdd_core::diff::export::write_text_report(&mut file, &diff_result)
            .context("Failed to write report")?;
        eprintln!("Report written to {path}");
    } else {
        let mut stdout = std::io::stdout().lock();
        mdd_core::diff::export::write_text_report(&mut stdout, &diff_result)
            .context("Failed to write report")?;
    }

    Ok(())
}
