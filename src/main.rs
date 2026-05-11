/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod llm;
#[cfg(feature = "mcp")]
mod mcp;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mdd_core::database;

#[derive(Parser)]
#[command(name = "mdd-ui", about = "MDD diagnostic database viewer")]
struct Cli {
    /// MDD file to open on startup
    file: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
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
        Some(Command::ExportDiff {
            old_file,
            new_file,
            output,
        }) => run_export_diff(&old_file, &new_file, output.as_deref()),
        #[cfg(feature = "mcp")]
        Some(Command::Mcp) => mcp::run_mcp(),
        None => {
            run_tauri_app(cli.file);
            Ok(())
        }
    }
}

fn run_tauri_app(initial_file: Option<String>) {
    // Work around WebKit2GTK black screen on Linux systems where
    // GPU DMA-BUF rendering fails silently. Must be set before GTK/WebKit init.
    #[cfg(target_os = "linux")]
    // SAFETY: called before any threads are spawned
    unsafe {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commands::AppState::default())
        .manage(commands::UdsState(tauri::async_runtime::Mutex::new(
            std::collections::HashMap::new(),
        )))
        .manage(commands::InitialFile(std::sync::Mutex::new(initial_file)))
        .invoke_handler(tauri::generate_handler![
            commands::load_mdd,
            commands::load_diff,
            commands::get_visible_nodes,
            commands::get_node_detail,
            commands::get_node_variant,
            commands::toggle_expand,
            commands::search,
            commands::clear_search,
            commands::cycle_search_scope,
            commands::set_search_scope,
            commands::toggle_sort,
            commands::expand_all,
            commands::expand_first_level,
            commands::collapse_all,
            commands::toggle_hide_unchanged,
            commands::set_ignore_first_level,
            commands::navigate_to,
            commands::get_node_path,
            commands::switch_tab,
            commands::close_tab,
            commands::get_open_tabs,
            commands::get_recent_files,
            commands::add_recent_file,
            commands::clear_recent_files,
            commands::clear_all_caches,
            commands::remove_recent_file,
            commands::get_ui_prefs,
            commands::save_ui_prefs,
            commands::register_mdd_association,
            commands::get_initial_file,
            llm::get_llm_settings,
            llm::save_llm_settings,
            llm::clear_llm_token,
            llm::start_ghe_device_flow,
            llm::poll_ghe_device_flow,
            llm::fetch_llm_models,
            llm::llm_chat,
            commands::uds_load,
            commands::uds_list_services,
            commands::uds_lookup,
            commands::uds_encode,
            commands::service_schema,
            commands::uds_list_variants,
            commands::uds_select_variant,
        ])
        .setup(|app| {
            use tauri::Manager;

            let icon_bytes = include_bytes!("../icons/128x128.png");

            // Set the GTK default window icon (affects taskbar on GTK-based DEs)
            // and force dark theme variant for WebKit's prefers-color-scheme.
            #[cfg(target_os = "linux")]
            {
                use gtk::{gdk_pixbuf::PixbufLoader, prelude::*};
                let loader = PixbufLoader::with_type("png")?;
                loader.write(icon_bytes)?;
                loader.close()?;
                let pixbuf = loader.pixbuf().expect("Failed to get pixbuf");
                gtk::Window::set_default_icon(&pixbuf);

                if let Some(settings) = gtk::Settings::default() {
                    settings.set_gtk_application_prefer_dark_theme(true);
                }
            }

            // Also set the individual window icon via Tauri API
            let window = app
                .get_webview_window("main")
                .expect("main window not found");
            let icon = tauri::image::Image::from_bytes(icon_bytes)?;
            window.set_icon(icon)?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|#[allow(unused_variables)] app_handle, event| {
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let tauri::RunEvent::Opened { urls } = event {
                use tauri::{Emitter, Manager};
                let path = urls
                    .iter()
                    .filter_map(|url| url.to_file_path().ok())
                    .find_map(|p| p.to_str().map(str::to_owned));
                if let Some(path) = path {
                    // Store for frontend initial-load (event may fire before webview mounts)
                    if let Some(state) = app_handle.try_state::<commands::InitialFile>() {
                        if let Ok(mut guard) = state.0.lock() {
                            *guard = Some(path.clone());
                        }
                    }
                    // Emit for already-mounted frontend
                    let _ = app_handle.emit("open-file", path);
                }
            }
        });
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
