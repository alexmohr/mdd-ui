// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

mod app;
mod database;
mod tree;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
#[command(name = "mdd-ui", about = "Browse MDD diagnostic databases")]
struct Cli {
    /// Path to the MDD file to open
    mdd_file: String,

    /// Path to a theme configuration file (TOML format)
    #[arg(long = "theme")]
    theme_file: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load colour configuration (uses defaults if no config file exists)
    let config = app::config::load_config(cli.theme_file.as_deref()).unwrap_or_else(|e| {
        eprintln!("Warning: {e:#}. Using defaults.");
        app::config::AppConfig::default()
    });
    let theme = app::config::ResolvedTheme::from(&config.colors);

    eprintln!("Loading {}...", cli.mdd_file);
    let db = database::load_mdd(&cli.mdd_file)
        .with_context(|| format!("Failed to load: {}", cli.mdd_file))?;

    eprintln!("Building tree...");
    let nodes = tree::build_tree(&db);
    eprintln!("Loaded {} nodes. Starting UI...", nodes.len());

    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)
        .context("Failed to enable mouse capture")?;

    let result = app::App::new(nodes, theme).run(&mut terminal);

    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();

    result.context("TUI error")
}
