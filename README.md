<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
-->

# mdd-ui

A terminal-based (TUI) browser for MDD diagnostic databases, built with [Ratatui](https://ratatui.rs). It renders the full ECU diagnostic tree — variants, functional groups, shared data, protocols, services, parameters, and more — in an interactive, navigable interface directly in your terminal.

![demo](img/demo.gif)

## Features

- **Hierarchical tree view** — browse ECU variants, functional groups, ECU shared data, protocols, layers, services, requests, responses, DOPs, SDGs, state charts, communication parameters, and functional classes.
  - **Grey elements in browse mode** — inherited services (from parent variants or functional groups) appear grey. They show no detail pane because they're references to services defined elsewhere; navigate to the actual service definition to see details.
- **Detail pane** — tabbed tables showing overview data, parameter lists, inherited references, and related items for the selected node.
- **Per-cell jump targets** — cells highlighted in blue are clickable links that navigate to the referenced element in the tree (e.g., jumping from a service's request to the request node itself).
- **Stacked search** — incremental, stackable search filters with configurable scope (All, Variants, Services, Diag-Comms, Requests, Responses, or a user-defined subtree).
- **Sorting** — toggle alphabetical/ID sorting for DiagComm lists, and column-level sorting in detail tables.
- **Navigation history** — breadcrumb trail with back-navigation so you never lose your place.
- **Mouse support** — click to select, drag the pane divider to resize, scroll with the mouse wheel, and click breadcrumbs to jump back. Toggle mouse mode with `m` to regain terminal text selection.
- **Fully configurable colour theme** — customise every colour via a TOML config file (named colours, hex, or ANSI-256 indices).
- **Diff mode** — compare two MDD files with colour-coded additions (green), removals (red/strikethrough), modifications (yellow), and unchanged elements (grey). The detail pane shows:
  - **Changes section** — A table comparing old vs. new values for modified properties
  - **Full element details** — Complete information about the element (like browse mode), showing the current state from the appropriate file
  - **Status notes** — Clear indicators for added/removed/unchanged elements

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

### Browse Mode (default)

```sh
mdd-ui <MDD_FILE> [--theme <THEME_FILE>]
```

| Argument | Description |
|---|---|
| `<MDD_FILE>` | Path to the MDD file to browse (required). |
| `--theme <THEME_FILE>` | Path to a TOML colour-theme configuration file (optional). |

#### Example

```sh
mdd-ui my_ecu.mdd
mdd-ui my_ecu.mdd --theme ~/.config/mdd-ui/config.toml
```

### Diff Mode

Compare two MDD files side by side in the TUI. Modified elements are shown in the detail pane with a table comparing old and new values.

```sh
mdd-ui diff <OLD_FILE> <NEW_FILE> [--theme <THEME_FILE>]
```

| Argument | Description |
|---|---|
| `<OLD_FILE>` | Path to the reference/old MDD file. |
| `<NEW_FILE>` | Path to the new MDD file to compare. |
| `--theme <THEME_FILE>` | Path to a TOML colour-theme configuration file (optional). |

#### Example

```sh
mdd-ui diff old_ecu.mdd new_ecu.mdd
```

#### Diff Mode Keybindings

| Key | Action |
|---|---|
| `u` | Toggle show/hide unchanged elements |
| `n` / `N` | Jump to next/previous change (when no search is active) |

### Export Diff (plain text)

Export a text-based diff report to a file or stdout:

```sh
mdd-ui export-diff <OLD_FILE> <NEW_FILE> [-o <OUTPUT_FILE>]
```

#### Example

```sh
mdd-ui export-diff old_ecu.mdd new_ecu.mdd -o diff_report.txt
mdd-ui export-diff old_ecu.mdd new_ecu.mdd  # prints to stdout
```

### MCP Server Mode

mdd-ui can run as an [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server over stdio, allowing AI assistants to browse, search, and diff MDD databases programmatically.

This requires building with the `mcp` feature:

```sh
cargo build --release --features mcp
```

Then start the server:

```sh
mdd-ui mcp
```

#### Available MCP Tools

| Tool | Description |
|---|---|
| `load_mdd` | Load an MDD file and return an ECU summary (name, variant count, etc.). Must be called before other read tools. |
| `browse_tree` | Navigate the tree hierarchy with optional depth limit and start index. |
| `get_node_details` | Get detailed information (overview tables, parameters, etc.) for a node by index. |
| `search_nodes` | Case-insensitive text search across all tree nodes. |
| `diff_mdd` | Compare two MDD files and return an annotated diff tree. |
| `export_diff` | Generate a full text diff report with property-level changes. |

#### OpenCode Configuration

To use the MCP server with [OpenCode](https://opencode.ai), add the following to your `opencode.json` (either in your project root or `~/.config/opencode/opencode.json`):

```json
{
  "mcp": {
    "mdd-ui": {
      "type": "local",
      "command": ["path/to/mdd-ui", "mcp"]
    }
  }
}
```

Replace `path/to/mdd-ui` with the actual path to your built binary (e.g., `target/release/mdd-ui`).

### Tauri Desktop UI

mdd-ui also includes a desktop application built with [Tauri](https://tauri.app/) and [Vue.js](https://vuejs.org/). This provides a graphical interface for browsing MDD databases.

#### Prerequisites

- Rust **2024 edition** (1.85+)
- [Bun](https://bun.sh/) (for the frontend build)
- [Tauri CLI](https://tauri.app/start/create-project/#manual-setup-tauri-cli) (`cargo install tauri-cli --locked`)

#### Running the Tauri UI (Development Mode)

```sh
cargo tauri dev
```

This will automatically install frontend dependencies, start the Vite dev server, and launch the Tauri window with hot-reloading enabled.

#### Building the Tauri UI (Release Build)

To build a release binary for your current platform:

```sh
cargo tauri build
```

The built application will be in `src/mdd-tauri/target/release/bundle/` with platform-specific installers:
- **Linux**: `.deb` and `.AppImage` files
- **macOS**: `.dmg` file and `.app` bundle
- **Windows**: `.msi` installer and `.exe` file

To build for a specific target platform, use the `--target` flag:

```sh
cargo tauri build --target x86_64-pc-windows-msvc
```

#### Automated Release Builds

The project includes a GitHub Actions workflow that automatically builds Tauri binaries for all platforms when a release is created. The binaries are uploaded as artifacts to the release page.

See [`.github/workflows/tauri-release.yml`](.github/workflows/tauri-release.yml) for the workflow configuration.

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
| `Shift+C` | Copy table to clipboard as markdown |
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

This is a Cargo workspace with three crates:

```
src/
├── main.rs                  # CLI parsing, database loading, TUI bootstrap
├── app/                     # TUI application state and logic
│   ├── mod.rs               #   Core App struct, event loop, state types
│   ├── clipboard.rs         #   Clipboard integration
│   ├── column_widths.rs     #   Per-section column width management
│   ├── config.rs            #   Theme/colour configuration loading
│   ├── cursor.rs            #   Cursor and scroll management
│   ├── history.rs           #   Navigation history / breadcrumbs
│   ├── input.rs             #   Keyboard input handling
│   ├── mouse/               #   Mouse event handling (areas, clicks, drag)
│   ├── navigation/          #   Jump-target resolution and tree navigation
│   ├── render/              #   Drawing: tree pane, detail pane, popups, tables
│   ├── search.rs            #   Search / filter stack logic
│   ├── sort.rs              #   Sorting (DiagComms, table columns)
│   └── visibility.rs        #   Visible-node calculation after search/collapse
├── mcp/                     # MCP server (optional, behind "mcp" feature)
│   └── mod.rs
├── mdd-core/                # Shared library crate
│   └── src/
│       ├── lib.rs
│       ├── database/        #   MDD file loading and data extraction
│       ├── diff/            #   Diff functionality (snapshot, compare, export)
│       └── tree/            #   Tree model, builder, types, and element modules
└── mdd-tauri/               # Tauri desktop application crate
    ├── src/
    │   ├── main.rs          #   Tauri entry point
    │   └── commands.rs      #   Tauri IPC commands
    ├── frontend/            #   Vue.js + Vite frontend
    │   ├── src/
    │   ├── package.json
    │   └── vite.config.ts
    └── tauri.conf.json
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
| [rmcp](https://github.com/modelcontextprotocol/rust-sdk) | MCP server SDK (optional, `mcp` feature) |
| [tokio](https://tokio.rs) | Async runtime for MCP server (optional, `mcp` feature) |
| [tauri](https://tauri.app) | Desktop application framework (mdd-tauri crate) |

## License

Licensed under [Apache-2.0](LICENSE).

```
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
```
