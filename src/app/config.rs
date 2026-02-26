/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use std::path::PathBuf;

use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::Deserialize;

/// Top-level configuration loaded from TOML.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub colors: ColorTheme,
}

/// Complete color theme, split into logical sections.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ColorTheme {
    pub tree: TreeColors,
    pub ui: UiColors,
    pub table: TableColors,
    pub popup: PopupColors,
}

/// Colors used in the tree pane.
#[derive(Deserialize)]
#[serde(default)]
pub struct TreeColors {
    pub container: String,
    pub section_header: String,
    pub inherited_service: String,
    pub default_node: String,
}

/// General UI chrome colors.
#[derive(Deserialize)]
#[serde(default)]
pub struct UiColors {
    pub border_focused: String,
    pub border_unfocused: String,
    pub cursor_fg: String,
    pub cursor_bg: String,
    pub breadcrumb_fg: String,
    pub breadcrumb_bg: String,
    pub status_fg: String,
    pub separator: String,
}

/// Colors used in the detail pane tables.
#[derive(Deserialize)]
#[serde(default)]
pub struct TableColors {
    pub header: String,
    pub cell: String,
    pub jump_cell: String,
    pub focused_cell_fg: String,
    pub focused_cell_bg: String,
    pub tab_active_fg: String,
    pub tab_active_bg: String,
    pub tab_inactive_fg: String,
    pub tab_inactive_bg: String,
}

/// Colors for popups (help, detail).
#[derive(Deserialize)]
#[serde(default)]
pub struct PopupColors {
    pub help_border: String,
    pub help_text: String,
    pub detail_border: String,
    pub detail_bg: String,
    pub detail_text: String,
}

// -------------------------------------------------------------------
// Defaults (matching the original hard-coded values)
// -------------------------------------------------------------------

impl Default for TreeColors {
    fn default() -> Self {
        Self {
            container: "blue".into(),
            section_header: "yellow".into(),
            inherited_service: "darkgray".into(),
            default_node: "white".into(),
        }
    }
}

impl Default for UiColors {
    fn default() -> Self {
        Self {
            border_focused: "cyan".into(),
            border_unfocused: "darkgray".into(),
            cursor_fg: "white".into(),
            cursor_bg: "darkgray".into(),
            breadcrumb_fg: "cyan".into(),
            breadcrumb_bg: "reset".into(),
            status_fg: "gray".into(),
            separator: "darkgray".into(),
        }
    }
}

impl Default for TableColors {
    fn default() -> Self {
        Self {
            header: "yellow".into(),
            cell: "white".into(),
            jump_cell: "blue".into(),
            focused_cell_fg: "white".into(),
            focused_cell_bg: "cyan".into(),
            tab_active_fg: "white".into(),
            tab_active_bg: "cyan".into(),
            tab_inactive_fg: "white".into(),
            tab_inactive_bg: "none".into(),
        }
    }
}

impl Default for PopupColors {
    fn default() -> Self {
        Self {
            help_border: "cyan".into(),
            help_text: "white".into(),
            detail_border: "yellow".into(),
            detail_bg: "reset".into(),
            detail_text: "white".into(),
        }
    }
}

// -------------------------------------------------------------------
// Colour parsing
// -------------------------------------------------------------------

/// Resolve a colour theme into concrete `ratatui::style::Color` values.
/// This is cached at startup so we parse colour strings only once.
pub struct ResolvedTheme {
    // Tree
    pub tree_container: Color,
    pub tree_section_header: Color,
    pub tree_inherited_service: Color,
    pub tree_default_node: Color,
    // UI
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub cursor_fg: Color,
    pub cursor_bg: Color,
    pub breadcrumb_fg: Color,
    pub breadcrumb_bg: Color,
    pub status_fg: Color,
    pub separator: Color,
    // Table
    pub table_header: Color,
    pub table_cell: Color,
    pub table_jump_cell: Color,
    pub focused_cell_fg: Color,
    pub focused_cell_bg: Color,
    pub tab_active_bg: Color,
    pub tab_inactive_fg: Color,
    pub tab_inactive_bg: Color,
    // Popup
    pub help_border: Color,
    pub help_text: Color,
    pub detail_border: Color,
    pub detail_bg: Color,
    pub detail_text: Color,
}

impl Default for ResolvedTheme {
    fn default() -> Self {
        Self::from(&ColorTheme::default())
    }
}

impl From<&ColorTheme> for ResolvedTheme {
    fn from(theme: &ColorTheme) -> Self {
        Self {
            tree_container: parse_color(&theme.tree.container),
            tree_section_header: parse_color(&theme.tree.section_header),
            tree_inherited_service: parse_color(&theme.tree.inherited_service),
            tree_default_node: parse_color(&theme.tree.default_node),

            border_focused: parse_color(&theme.ui.border_focused),
            border_unfocused: parse_color(&theme.ui.border_unfocused),
            cursor_fg: parse_color(&theme.ui.cursor_fg),
            cursor_bg: parse_color(&theme.ui.cursor_bg),
            breadcrumb_fg: parse_color(&theme.ui.breadcrumb_fg),
            breadcrumb_bg: parse_color(&theme.ui.breadcrumb_bg),
            status_fg: parse_color(&theme.ui.status_fg),
            separator: parse_color(&theme.ui.separator),

            table_header: parse_color(&theme.table.header),
            table_cell: parse_color(&theme.table.cell),
            table_jump_cell: parse_color(&theme.table.jump_cell),
            focused_cell_fg: parse_color(&theme.table.focused_cell_fg),
            focused_cell_bg: parse_color(&theme.table.focused_cell_bg),
            tab_active_bg: parse_color(&theme.table.tab_active_bg),
            tab_inactive_fg: parse_color(&theme.table.tab_inactive_fg),
            tab_inactive_bg: parse_color(&theme.table.tab_inactive_bg),

            help_border: parse_color(&theme.popup.help_border),
            help_text: parse_color(&theme.popup.help_text),
            detail_border: parse_color(&theme.popup.detail_border),
            detail_bg: parse_color(&theme.popup.detail_bg),
            detail_text: parse_color(&theme.popup.detail_text),
        }
    }
}

/// Parse a colour name or hex string into a `Color`.
/// Supported formats:
///  - Named: `"red"`, `"blue"`, `"darkgray"`, etc.
///  - Hex:   `"#ff00ff"` or `"ff00ff"`
///  - ANSI index: `"123"` (0-255)
fn parse_color(s: &str) -> Color {
    let s = s.trim().to_lowercase();

    // Try hex (#RRGGBB or RRGGBB)
    let hex = s.strip_prefix('#').unwrap_or(&s);
    if hex.len() == 6
        && let Ok(r) = u8::from_str_radix(&hex[0..2], 16)
        && let Ok(g) = u8::from_str_radix(&hex[2..4], 16)
        && let Ok(b) = u8::from_str_radix(&hex[4..6], 16)
    {
        return Color::Rgb(r, g, b);
    }

    // Try ANSI index
    if let Ok(idx) = s.parse::<u8>() {
        return Color::Indexed(idx);
    }

    // Named colours
    match s.as_str() {
        "none" | "reset" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" | "dark_gray" | "dark_grey" => Color::DarkGray,
        "lightred" | "light_red" => Color::LightRed,
        "lightgreen" | "light_green" => Color::LightGreen,
        "lightyellow" | "light_yellow" => Color::LightYellow,
        "lightblue" | "light_blue" => Color::LightBlue,
        "lightmagenta" | "light_magenta" => Color::LightMagenta,
        "lightcyan" | "light_cyan" => Color::LightCyan,
        // "white" and any unrecognised string fall back to White
        _ => Color::White,
    }
}

// -------------------------------------------------------------------
// Loading
// -------------------------------------------------------------------

/// Return the default configuration directory for `mdd-ui`.
///
/// - Linux:  `$XDG_CONFIG_HOME/mdd-ui` or `$HOME/.config/mdd-ui`
/// - macOS:  `$HOME/Library/Application Support/mdd-ui`
fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Library/Application Support/mdd-ui"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|d| d.join("mdd-ui"))
    }
}

/// Load configuration from a file.
///
/// If `override_path` is provided, that file is used directly.
/// Otherwise the standard location is checked
/// (`$XDG_CONFIG_HOME/mdd-ui/config.toml` on Linux,
///  `~/Library/Application Support/mdd-ui/config.toml` on macOS, etc.)
///
/// Returns the default configuration if no file exists or parsing fails.
pub fn load_config(override_path: Option<&str>) -> Result<AppConfig> {
    let path = if let Some(p) = override_path {
        PathBuf::from(p)
    } else {
        let Some(dir) = config_dir() else {
            return Ok(AppConfig::default());
        };
        dir.join("config.toml")
    };

    if !path.exists() {
        if override_path.is_some() {
            anyhow::bail!("Theme file not found: {}", path.display());
        }
        return Ok(AppConfig::default());
    }

    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;

    toml::from_str(&text).with_context(|| format!("Failed to parse config: {}", path.display()))
}
