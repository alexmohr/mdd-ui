/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod app;
mod database;
mod diff;
#[cfg(feature = "mcp")]
mod mcp;
mod tree;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mdd-ui", about = "Browse and compare MDD diagnostic databases")]
struct Cli {
    /// Path to the MDD file to browse (default mode)
    mdd_file: Option<String>,

    /// Path to a theme configuration file (TOML format)
    #[arg(long = "theme")]
    theme_file: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Compare two MDD diagnostic databases
    Diff {
        /// Path to the old/reference MDD file
        old_file: String,

        /// Path to the new MDD file
        new_file: String,

        /// Path to a theme configuration file (TOML format)
        #[arg(long = "theme")]
        theme_file: Option<String>,
    },
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

/// Restores the terminal to its original state.
/// This disables mouse capture and restores the terminal mode.
fn restore_terminal() {
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Diff {
            old_file,
            new_file,
            theme_file,
        }) => run_diff(&old_file, &new_file, theme_file.as_deref()),
        Some(Command::ExportDiff {
            old_file,
            new_file,
            output,
        }) => run_export_diff(&old_file, &new_file, output.as_deref()),
        #[cfg(feature = "mcp")]
        Some(Command::Mcp) => mcp::run_mcp(),
        None => {
            let Some(mdd_file) = cli.mdd_file else {
                bail!("No MDD file specified. Usage: mdd-ui <MDD_FILE> [--theme <THEME_FILE>]");
            };
            run_browse(&mdd_file, cli.theme_file.as_deref())
        }
    }
}

fn load_theme(theme_file: Option<&str>) -> app::config::ResolvedTheme {
    let config = app::config::load_config(theme_file).unwrap_or_else(|e| {
        eprintln!("Warning: {e:#}. Using defaults.");
        app::config::AppConfig::default()
    });
    app::config::ResolvedTheme::from(&config.colors)
}

/// Shared TUI bootstrap: terminal init, panic hook, `App::run()`, and restore.
fn run_tui(
    nodes: Vec<tree::TreeNode>,
    ecu_name: String,
    theme: app::config::ResolvedTheme,
    is_diff_mode: bool,
) -> Result<()> {
    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)
        .context("Failed to enable mouse capture")?;

    // Install a panic hook that restores the terminal before printing the panic message.
    // This prevents mouse escape sequences from leaking to the terminal on panic.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));

    let result = app::App::new(nodes, ecu_name, theme, is_diff_mode).run(&mut terminal);

    restore_terminal();

    result.context("TUI error")
}

fn run_browse(mdd_file: &str, theme_file: Option<&str>) -> Result<()> {
    let theme = load_theme(theme_file);

    eprintln!("Loading {mdd_file}...");
    let db = database::load_mdd(mdd_file).with_context(|| format!("Failed to load: {mdd_file}"))?;

    eprintln!("Building tree...");
    let (nodes, ecu_name) = tree::build_tree(&db, mdd_file);
    eprintln!("Loaded {} nodes. Starting UI...", nodes.len());

    run_tui(nodes, ecu_name, theme, false)
}

fn run_diff(old_file: &str, new_file: &str, theme_file: Option<&str>) -> Result<()> {
    let theme = load_theme(theme_file);

    eprintln!("Loading {old_file}...");
    let db_old =
        database::load_mdd(old_file).with_context(|| format!("Failed to load: {old_file}"))?;

    eprintln!("Loading {new_file}...");
    let db_new =
        database::load_mdd(new_file).with_context(|| format!("Failed to load: {new_file}"))?;

    eprintln!("Building diff tree...");
    let (nodes, ecu_name) = diff::diff_tree::build_diff_tree(&db_old, &db_new, old_file, new_file);
    eprintln!("Built {} diff tree nodes. Starting UI...", nodes.len());

    run_tui(nodes, ecu_name, theme, true)
}

fn run_export_diff(old_file: &str, new_file: &str, output: Option<&str>) -> Result<()> {
    eprintln!("Loading {old_file}...");
    let db_old =
        database::load_mdd(old_file).with_context(|| format!("Failed to load: {old_file}"))?;

    eprintln!("Loading {new_file}...");
    let db_new =
        database::load_mdd(new_file).with_context(|| format!("Failed to load: {new_file}"))?;

    eprintln!("Extracting snapshots...");
    let snap_old = diff::snapshot::EcuSnapshot::from_database(&db_old)
        .context("Failed to extract old database snapshot")?;
    let snap_new = diff::snapshot::EcuSnapshot::from_database(&db_new)
        .context("Failed to extract new database snapshot")?;

    eprintln!("Comparing...");
    let diff_result = diff::compare::compare(&snap_old, &snap_new);

    if let Some(path) = output {
        let mut file = std::fs::File::create(path)
            .with_context(|| format!("Failed to create output file: {path}"))?;
        diff::export::write_text_report(&mut file, &diff_result)
            .context("Failed to write report")?;
        eprintln!("Report written to {path}");
    } else {
        let mut stdout = std::io::stdout().lock();
        diff::export::write_text_report(&mut stdout, &diff_result)
            .context("Failed to write report")?;
    }

    Ok(())
}
