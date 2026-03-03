/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod app;
mod database;
mod diff;
mod tree;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
#[command(name = "mdd-ui", about = "Browse and compare MDD diagnostic databases")]
struct Cli {
    /// Path to the MDD file to open (view mode), or the base file (diff mode)
    mdd_file: String,

    /// Compare two MDD files and show differences
    #[arg(long = "diff")]
    diff_file: Option<String>,

    /// Path to a theme configuration file (TOML format)
    #[arg(long = "theme")]
    theme_file: Option<String>,

    /// Output format for diff: text (default), json
    #[arg(long, default_value = "text")]
    format: diff::OutputFormat,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(compare_file) = cli.diff_file {
        diff::run_diff(&cli.mdd_file, &compare_file, cli.format)
    } else {
        run_view(&cli.mdd_file, cli.theme_file.as_deref())
    }
}

fn run_view(mdd_file: &str, theme_file: Option<&str>) -> Result<()> {
    // Load colour configuration (uses defaults if no config file exists)
    let config = app::config::load_config(theme_file).unwrap_or_else(|e| {
        eprintln!("Warning: {e:#}. Using defaults.");
        app::config::AppConfig::default()
    });
    let theme = app::config::ResolvedTheme::from(&config.colors);

    eprintln!("Loading {mdd_file}...");
    let db = database::load_mdd(mdd_file).with_context(|| format!("Failed to load: {mdd_file}"))?;

    eprintln!("Building tree...");
    let (nodes, ecu_name) = tree::build_tree(&db, mdd_file);
    eprintln!("Loaded {} nodes. Starting UI...", nodes.len());

    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)
        .context("Failed to enable mouse capture")?;

    let result = app::App::new(nodes, ecu_name, theme).run(&mut terminal);

    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();

    result.context("TUI error")
}
