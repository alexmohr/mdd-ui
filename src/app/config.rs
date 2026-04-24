/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use std::{fmt, path::PathBuf};

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
    pub diff: DiffColors,
}

/// A validated color string that is parsed to `ratatui::style::Color` at
/// deserialization time.  Invalid values cause a parse-time error rather than
/// a silent fallback.
#[derive(Clone, Debug)]
pub struct ColorString(pub Color);

impl ColorString {
    pub fn color(&self) -> Color {
        self.0
    }
}

impl TryFrom<String> for ColorString {
    type Error = String;
    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        try_parse_color(&s).map(ColorString)
    }
}

impl<'de> Deserialize<'de> for ColorString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ColorString::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for ColorString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Colors used in the tree pane.
#[derive(Deserialize)]
#[serde(default)]
pub struct TreeColors {
    pub container: ColorString,
    pub section_header: ColorString,
    pub inherited_service: ColorString,
    pub default_node: ColorString,
}

/// General UI chrome colors.
#[derive(Deserialize)]
#[serde(default)]
pub struct UiColors {
    pub border_focused: ColorString,
    pub border_unfocused: ColorString,
    pub cursor_fg: ColorString,
    pub cursor_bg: ColorString,
    pub breadcrumb_fg: ColorString,
    pub breadcrumb_bg: ColorString,
    pub status_fg: ColorString,
    pub separator: ColorString,
}

/// Colors used in the detail pane tables.
#[derive(Deserialize)]
#[serde(default)]
pub struct TableColors {
    pub header: ColorString,
    pub cell: ColorString,
    pub jump_cell: ColorString,
    pub focused_cell_fg: ColorString,
    pub focused_cell_bg: ColorString,
    pub tab_active_fg: ColorString,
    pub tab_active_bg: ColorString,
    pub tab_inactive_fg: ColorString,
    pub tab_inactive_bg: ColorString,
}

/// Colors for popups (help, detail).
#[derive(Deserialize)]
#[serde(default)]
pub struct PopupColors {
    pub help_border: ColorString,
    pub help_text: ColorString,
    pub detail_border: ColorString,
    pub detail_bg: ColorString,
    pub detail_text: ColorString,
}

/// Colors for diff annotations in the tree.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct DiffColors {
    pub added: ColorString,
    pub removed: ColorString,
    pub modified: ColorString,
    pub unchanged: ColorString,
}

// -------------------------------------------------------------------
// Defaults (matching the original hard-coded values)
// -------------------------------------------------------------------

/// Helper: create a `ColorString` from a known-good literal (panics on bad input).
fn cs(s: &str) -> ColorString {
    ColorString::try_from(s.to_owned()).expect("default color must be valid")
}

impl Default for TreeColors {
    fn default() -> Self {
        Self {
            container: cs("blue"),
            section_header: cs("yellow"),
            inherited_service: cs("darkgray"),
            default_node: cs("white"),
        }
    }
}

impl Default for UiColors {
    fn default() -> Self {
        Self {
            border_focused: cs("cyan"),
            border_unfocused: cs("darkgray"),
            cursor_fg: cs("white"),
            cursor_bg: cs("darkgray"),
            breadcrumb_fg: cs("cyan"),
            breadcrumb_bg: cs("reset"),
            status_fg: cs("gray"),
            separator: cs("darkgray"),
        }
    }
}

impl Default for TableColors {
    fn default() -> Self {
        Self {
            header: cs("yellow"),
            cell: cs("white"),
            jump_cell: cs("blue"),
            focused_cell_fg: cs("white"),
            focused_cell_bg: cs("cyan"),
            tab_active_fg: cs("white"),
            tab_active_bg: cs("cyan"),
            tab_inactive_fg: cs("white"),
            tab_inactive_bg: cs("none"),
        }
    }
}

impl Default for PopupColors {
    fn default() -> Self {
        Self {
            help_border: cs("cyan"),
            help_text: cs("white"),
            detail_border: cs("yellow"),
            detail_bg: cs("reset"),
            detail_text: cs("white"),
        }
    }
}

impl Default for DiffColors {
    fn default() -> Self {
        Self {
            added: cs("green"),
            removed: cs("red"),
            modified: cs("yellow"),
            unchanged: cs("darkgray"),
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
    pub tab_active_fg: Color,
    pub tab_active_bg: Color,
    pub tab_inactive_fg: Color,
    pub tab_inactive_bg: Color,
    // Popup
    pub help_border: Color,
    pub help_text: Color,
    pub detail_border: Color,
    pub detail_bg: Color,
    pub detail_text: Color,
    // Diff
    pub diff_added: Color,
    pub diff_removed: Color,
    pub diff_modified: Color,
    pub diff_unchanged: Color,
}

impl Default for ResolvedTheme {
    fn default() -> Self {
        Self::from(&ColorTheme::default())
    }
}

impl From<&ColorTheme> for ResolvedTheme {
    fn from(theme: &ColorTheme) -> Self {
        Self {
            tree_container: theme.tree.container.color(),
            tree_section_header: theme.tree.section_header.color(),
            tree_inherited_service: theme.tree.inherited_service.color(),
            tree_default_node: theme.tree.default_node.color(),

            border_focused: theme.ui.border_focused.color(),
            border_unfocused: theme.ui.border_unfocused.color(),
            cursor_fg: theme.ui.cursor_fg.color(),
            cursor_bg: theme.ui.cursor_bg.color(),
            breadcrumb_fg: theme.ui.breadcrumb_fg.color(),
            breadcrumb_bg: theme.ui.breadcrumb_bg.color(),
            status_fg: theme.ui.status_fg.color(),
            separator: theme.ui.separator.color(),

            table_header: theme.table.header.color(),
            table_cell: theme.table.cell.color(),
            table_jump_cell: theme.table.jump_cell.color(),
            focused_cell_fg: theme.table.focused_cell_fg.color(),
            focused_cell_bg: theme.table.focused_cell_bg.color(),
            tab_active_fg: theme.table.tab_active_fg.color(),
            tab_active_bg: theme.table.tab_active_bg.color(),
            tab_inactive_fg: theme.table.tab_inactive_fg.color(),
            tab_inactive_bg: theme.table.tab_inactive_bg.color(),

            help_border: theme.popup.help_border.color(),
            help_text: theme.popup.help_text.color(),
            detail_border: theme.popup.detail_border.color(),
            detail_bg: theme.popup.detail_bg.color(),
            detail_text: theme.popup.detail_text.color(),

            diff_added: theme.diff.added.color(),
            diff_removed: theme.diff.removed.color(),
            diff_modified: theme.diff.modified.color(),
            diff_unchanged: theme.diff.unchanged.color(),
        }
    }
}

/// Try to parse a colour name or hex string into a `Color`.
/// Returns `Err` for unrecognised values so callers can fail early.
///
/// Supported formats:
///  - Named: `"red"`, `"blue"`, `"darkgray"`, etc.
///  - Hex:   `"#ff00ff"` or `"ff00ff"`
///  - ANSI index: `"123"` (0-255)
fn try_parse_color(s: &str) -> std::result::Result<Color, String> {
    let s = s.trim().to_lowercase();

    // Try hex (#RRGGBB or RRGGBB)
    let hex = s.strip_prefix('#').unwrap_or(&s);
    if hex.len() == 6
        && let Ok(r) = u8::from_str_radix(&hex[0..2], 16)
        && let Ok(g) = u8::from_str_radix(&hex[2..4], 16)
        && let Ok(b) = u8::from_str_radix(&hex[4..6], 16)
    {
        return Ok(Color::Rgb(r, g, b));
    }

    // Try ANSI index
    if let Ok(idx) = s.parse::<u8>() {
        return Ok(Color::Indexed(idx));
    }

    // Named colours
    match s.as_str() {
        "none" | "reset" => Ok(Color::Reset),
        "black" => Ok(Color::Black),
        "red" => Ok(Color::Red),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "blue" => Ok(Color::Blue),
        "magenta" => Ok(Color::Magenta),
        "cyan" => Ok(Color::Cyan),
        "gray" | "grey" => Ok(Color::Gray),
        "darkgray" | "darkgrey" | "dark_gray" | "dark_grey" => Ok(Color::DarkGray),
        "lightred" | "light_red" => Ok(Color::LightRed),
        "lightgreen" | "light_green" => Ok(Color::LightGreen),
        "lightyellow" | "light_yellow" => Ok(Color::LightYellow),
        "lightblue" | "light_blue" => Ok(Color::LightBlue),
        "lightmagenta" | "light_magenta" => Ok(Color::LightMagenta),
        "lightcyan" | "light_cyan" => Ok(Color::LightCyan),
        "white" => Ok(Color::White),
        _ => Err(format!("unrecognised color: '{s}'")),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_named_colors() {
        assert_eq!(try_parse_color("red"), Ok(Color::Red));
        assert_eq!(try_parse_color("Blue"), Ok(Color::Blue));
        assert_eq!(try_parse_color(" CYAN "), Ok(Color::Cyan));
        assert_eq!(try_parse_color("darkgray"), Ok(Color::DarkGray));
        assert_eq!(try_parse_color("dark_grey"), Ok(Color::DarkGray));
        assert_eq!(try_parse_color("none"), Ok(Color::Reset));
        assert_eq!(try_parse_color("reset"), Ok(Color::Reset));
    }

    #[test]
    fn parse_hex_colors() {
        assert_eq!(try_parse_color("#ff0000"), Ok(Color::Rgb(255, 0, 0)));
        assert_eq!(try_parse_color("00ff00"), Ok(Color::Rgb(0, 255, 0)));
        assert_eq!(try_parse_color("#ABCDEF"), Ok(Color::Rgb(0xAB, 0xCD, 0xEF)));
    }

    #[test]
    fn parse_ansi_index() {
        assert_eq!(try_parse_color("42"), Ok(Color::Indexed(42)));
        assert_eq!(try_parse_color("0"), Ok(Color::Indexed(0)));
        assert_eq!(try_parse_color("255"), Ok(Color::Indexed(255)));
    }

    #[test]
    fn parse_invalid_color_returns_err() {
        assert!(try_parse_color("not_a_color").is_err());
        assert!(try_parse_color("redd").is_err());
        assert!(try_parse_color("").is_err());
    }

    #[test]
    fn color_string_try_from_valid() {
        let cs = ColorString::try_from("cyan".to_owned());
        assert!(cs.is_ok());
        assert_eq!(cs.unwrap().color(), Color::Cyan);
    }

    #[test]
    fn color_string_try_from_invalid() {
        let cs = ColorString::try_from("garbage".to_owned());
        assert!(cs.is_err());
    }

    #[test]
    fn default_theme_resolves_without_panic() {
        let _resolved = ResolvedTheme::default();
    }
}
