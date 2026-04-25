// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::load_mdd,
            commands::load_diff,
            commands::get_visible_nodes,
            commands::get_node_detail,
            commands::toggle_expand,
            commands::search,
            commands::clear_search,
            commands::cycle_search_scope,
            commands::toggle_sort,
            commands::expand_all,
            commands::collapse_all,
            commands::toggle_hide_unchanged,
            commands::navigate_to,
            commands::get_node_path,
            commands::get_recent_files,
            commands::add_recent_file,
            commands::clear_recent_files,
            commands::remove_recent_file,
            commands::get_ui_prefs,
            commands::save_ui_prefs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
