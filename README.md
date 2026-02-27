<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
-->

# mdd-ui

A terminal-based (TUI) browser for MDD diagnostic databases, built with [Ratatui](https://ratatui.rs). It renders the full ECU diagnostic tree — variants, functional groups, shared data, protocols, services, parameters, and more — in an interactive, navigable interface directly in your terminal.

![demo](img/demo.png)

## Features

- **Hierarchical tree view** — browse ECU variants, functional groups, ECU shared data, protocols, layers, services, requests, responses, DOPs, SDGs, state charts, communication parameters, and functional classes.
- **Detail pane** — tabbed tables showing overview data, parameter lists, inherited references, and related items for the selected node.
- **Per-cell jump targets** — cells highlighted in blue are clickable links that navigate to the referenced element in the tree (e.g., jumping from a service's request to the request node itself).
- **Stacked search** — incremental, stackable search filters with configurable scope (All, Variants, Services, Diag-Comms, Requests, Responses, or a user-defined subtree).
- **Sorting** — toggle alphabetical/ID sorting for DiagComm lists, and column-level sorting in detail tables.
- **Navigation history** — breadcrumb trail with back-navigation so you never lose your place.
- **Mouse support** — click to select, drag the pane divider to resize, scroll with the mouse wheel, and click breadcrumbs to jump back. Toggle mouse mode with `m` to regain terminal text selection.
- **Fully configurable colour theme** — customise every colour via a TOML config file (named colours, hex, or ANSI-256 indices).

## Installation

### Prerequisites

- Rust **2024 edition** (1.85+)
- An MDD diagnostic database file to browse

### Build from source

```sh
git clone https://github.com/alexmohr/mdd-ui.git
cd mdd-ui
cargo build --release
```

The binary is placed at `target/release/mdd-ui`.

## Usage

```sh
mdd-ui <MDD_FILE> [--theme <THEME_FILE>]
```

| Argument | Description |
|---|---|
| `<MDD_FILE>` | Path to the MDD file to open (required). |
| `--theme <THEME_FILE>` | Path to a TOML colour-theme configuration file (optional). |

### Example

```sh
mdd-ui my_ecu.mdd
mdd-ui my_ecu.mdd --theme ~/.config/mdd-ui/config.toml
```

## Keybindings

### Navigation

| Key | Action |
|---|---|
| `↑` / `↓` or `k` / `j` | Move cursor up / down |
| `←` / `→` or `h` / `l` | Collapse / expand node (or switch tabs in the detail pane) |
| `PgUp` / `PgDn` | Page up / down |
| `Home` / `End` | Jump to first / last item |
| `Space` | Toggle expand / collapse the current node |
| `Tab` | Switch focus between tree and detail pane |
| `Backspace` | Jump to the last element in navigation history |
| `Enter` | Expand node, or navigate to the referenced element in the detail pane |

### Tree Operations

| Key | Action |
|---|---|
| `e` | Expand all nodes |
| `c` | Collapse all nodes |
| `s` | Toggle sort (by ID or name for services, by name for others) |

### Search & Filter

| Key | Action |
|---|---|
| `/` | Start search (type query, then press `Enter` to add to filter stack) |
| `Shift+S` | Cycle search scope (All → Variants → Services → Diag-Comms → Requests → Responses) |
| `t` | Scope search to the subtree under the cursor |
| `x` | Clear all search filters |
| `Backspace` | Remove the last search from the stack (when search input is empty) |
| `Esc` | Cancel current search input |
| `n` / `N` | Jump to next / previous search match |

### Detail Pane (when focused)

| Key | Action |
|---|---|
| `↑` / `↓` or `Shift+K` / `Shift+J` | Navigate rows |
| `←` / `→` or `Shift+H` / `Shift+L` | Switch tabs |
| `Enter` | Navigate to element or show detail popup |
| `Shift+S` | Toggle sort on focused column |
| `[` / `]` | Decrease / increase column width |
| `,` / `.` | Focus previous / next column |
| `<` / `>` | Scroll table left / right |
| `a-z`, `0-9` | Type-to-jump — jump to the row matching the typed text (resets after 1 s) |

### Type-to-Jump (tree)

| Key | Action |
|---|---|
| `a-z`, `0-9` | Jump to the tree node matching the typed text (resets after 1 s) |

### Window & General

| Key | Action |
|---|---|
| `+` / `-` | Increase / decrease tree pane width |
| Mouse drag | Drag the divider between tree and detail pane to resize |
| `m` | Toggle mouse mode (enable / disable terminal text selection) |
| `?` | Show help popup |
| `Q` or `Esc` | Quit |

## Theme Configuration

Copy `config.example.toml` to one of the following locations:

| OS | Path |
|---|---|
| Linux | `$XDG_CONFIG_HOME/mdd-ui/config.toml` (usually `~/.config/mdd-ui/config.toml`) |
| macOS | `~/Library/Application Support/mdd-ui/config.toml` |
| Windows | `%APPDATA%\mdd-ui\config.toml` |

Or pass any path explicitly with `--theme`.

### Supported colour formats

- **Named** — `"red"`, `"blue"`, `"darkgray"`, `"lightcyan"`, etc.
- **Hex** — `"#ff00ff"` or `"ff00ff"`
- **ANSI index** — `"123"` (0–255)

### Configurable sections

| Section | Controls |
|---|---|
| `[colors.tree]` | Tree node colours (containers, section headers, inherited services, default nodes) |
| `[colors.ui]` | UI chrome (borders, cursor, breadcrumbs, status bar, separators) |
| `[colors.table]` | Table colours (headers, cells, jump cells, active/inactive tabs) |
| `[colors.popup]` | Popup colours (help border/text, detail border/background/text) |

See [config.example.toml](config.example.toml) for the full reference with all defaults.

## Project Structure

```
src/
├── main.rs              # CLI parsing, database loading, TUI bootstrap
├── app/                 # Application state and TUI logic
│   ├── mod.rs           #   Core App struct, event loop, state types
│   ├── config.rs        #   Theme/colour configuration loading
│   ├── cursor.rs        #   Cursor and scroll management
│   ├── history.rs       #   Navigation history / breadcrumbs
│   ├── input.rs         #   Keyboard input handling
│   ├── mouse.rs         #   Mouse event handling
│   ├── navigation.rs    #   Jump-target resolution and tree navigation
│   ├── render.rs        #   Drawing: tree pane, detail pane, popups
│   ├── search.rs        #   Search / filter stack logic
│   ├── sort.rs          #   Sorting (DiagComms, table columns)
│   ├── column_widths.rs #   Per-section column width management
│   └── visibility.rs    #   Visible-node calculation after search/collapse
├── database/            # MDD file loading and data extraction
│   ├── mod.rs
│   └── reader.rs
└── tree/                # Tree model
    ├── mod.rs           #   build_tree entry point
    ├── builder.rs       #   TreeBuilder helper
    ├── types.rs         #   TreeNode, NodeType, DetailSection, etc.
    └── elements/        #   One module per database element type
        ├── dtcs.rs
        ├── layers.rs
        └── variants/
            ├── services.rs
            ├── requests.rs
            ├── responses.rs
            ├── dops/
            ├── ...
            └── mod.rs
```

## Dependencies

| Crate | Purpose |
|---|---|
| [cda-database](https://github.com/eclipse-opensovd/classic-diagnostic-adapter) | MDD/FlatBuffers diagnostic database reader |
| [ratatui](https://ratatui.rs) | Terminal UI framework |
| [crossterm](https://github.com/crossterm-rs/crossterm) | Cross-platform terminal manipulation |
| [clap](https://docs.rs/clap) | Command-line argument parsing |
| [anyhow](https://docs.rs/anyhow) | Ergonomic error handling |
| [serde](https://serde.rs) + [toml](https://docs.rs/toml) | Theme configuration deserialization |

## License

Licensed under [Apache-2.0](LICENSE).

```
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
```
