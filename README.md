<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
-->

# MDD-UI - Diagnostic Database Viewer

A terminal-based user interface for browsing and exploring MDD (Marvelous Diagnostic Data) diagnostic databases. Built with Rust and [ratatui](https://ratatui.rs/), this tool provides an efficient way to navigate ECU diagnostic services, parameters, and metadata.

**DISCLAIMER**
This tool is 99% vibe coded, so manually editing stuff _might_ be hard.
This project is in early development and may contain bugs or incomplete features.
Use at your own risk.
Contributions _are_ welcome but things still moved around _a lot_ so I would recommend opening an issue before starting work on something.

## Features

<img src="img/demo.png">

### 🎯 Core Functionality
- **Tree Navigation**: Browse ECU structure, variants, and diagnostic services with collapsible/expandable nodes
- **Multi-Tab Detail Panes**: View comprehensive information with table-based formatting across multiple tabs
- **Multi-Section Views**: Separate tabs for Request, Positive Response, and Negative Response with independent scrolling
- **Advanced Search**: Hierarchical search stack system with scope filtering (All/Variants/FunctionalGroups/EcuSharedData/Services/DiagComms/Requests/Responses)
- **Keyboard-First**: Vim-style navigation (h/j/k/l in tree, Shift+H/J/K/L in detail) with full keyboard control
- **Type-to-Jump**: Quickly jump to tree nodes by typing matching text
- **Mouse Support**: Full mouse interaction including click-to-select, scrolling, and double-click to expand/show details
- **Configurable Colors**: Customize the entire color scheme via a TOML configuration file

### 📁 Database Structure Support
The tool provides full navigation and detailed views for:

#### **Variants**
- Base variants and specific variant configurations
- Variant summary information (short name, long name, base status)
- Parent references to other variants, ECU shared data, protocols, and functional groups
- Visual distinction for base variants (`[base]` suffix)
- Inherited service display from parent references (shown in gray)

#### **Functional Groups**
- Functional group definitions with diagnostic layers
- Cross-variant service collection for each functional class
- Aggregated view of services and jobs belonging to each class

#### **ECU Shared Data**
- Shared diagnostic data across multiple functional groups
- Deduplication of shared data references
- Full diagnostic layer hierarchy

#### **Protocols**
- Protocol-level diagnostic layers
- Protocol specifications and configurations
- Parent protocol references

### 📊 Diagnostic Elements Display

#### **Diagnostic Services (DiagComms)**
- Service ID (SID) and Sub-function ID with hex formatting (0x01-0xFFFF)
- Service sorting: by ID (default) or by name (toggle with `s`)
- Inherited services from parent references (displayed with source layer)
- Multiple detail views per service:
  - **Overview**: Service metadata, semantic information, audience flags
  - **Request**: Request parameters with types, positions, coded values, and DOP references
  - **Positive Responses**: One or more positive response parameter tables
  - **Negative Responses**: NRC (Negative Response Code) definitions
  - **ComParam-Refs**: Communication parameter references
  - **Audience**: Manufacturing, development, aftersales, and aftermarket flags

#### **Single ECU Jobs**
- Job definitions and parameters
- Job-level diagnostic operations

#### **Functional Classes**
- Functional class definitions from base variants
- Services and jobs grouped by functional class
- Cross-variant aggregation of class members

#### **State Charts**
- State machine definitions
- State transitions with source and target states
- Individual state definitions

#### **Requests Section**
- All services with request definitions
- Same detailed view as DiagComms (filtered to request-enabled services)
- Parameter details: name, byte position, bit position, semantic, coded value, DOP reference

#### **Positive Responses Section**
- All services with positive response definitions
- Multiple response definitions per service
- Full parameter breakdown with encoding details

#### **Negative Responses Section**
- All services with negative response definitions
- NRC (Negative Response Code) parameters

#### **Parent References**
- Parent reference type and short name
- Not-inherited elements across multiple tabs:
  - Not-Inherited Diag-Comms (clickable service references)
  - Not-Inherited Diag-Variables
  - Not-Inherited DOPs (Data Object Properties)
  - Not-Inherited Tables

### 🔍 Parameter Details
For each parameter, the tool displays:
- **Name**: Parameter short name
- **Byte Position**: Starting byte in message
- **Bit Position**: Starting bit within byte (if applicable)
- **Semantic**: Parameter meaning/description
- **Coded Value**: Hex-formatted constant values (0x01, 0x10, etc.)
- **DOP (Data Object Property)**: Reference to data operation definitions
- **Type**: Parameter type (CodedConst, Value, etc.)

### 🔍 Search Features
- **Hierarchical Search Stack**: Build complex filtered views by chaining multiple searches
- **Search Scopes**:
  - All: Search everywhere
  - Variants: Filter variant names only
  - Functional Groups: Filter functional group names
  - ECU Shared Data: Filter ECU shared data names
  - Services: Filter service names only
  - DiagComms: Filter diagnostic communication sections
  - Requests: Filter request sections
  - Responses: Filter response sections (positive and negative)
- **Search Navigation**: Jump between matches with `n`/`N`
- **Search Stack Management**: Clear entire search stack with `x`, pop last search with Backspace
- **Scope Cycling**: Switch search scope with Shift+S
- **Scope Indicators**: Visual indicators showing active scope (e.g., `[V]` for Variants, `[S]` for Services)

### 🎨 Visual Features
- **Row-Level Navigation**: Select individual table rows with j/k
- **Highlighted Selection**: Dark gray background for selected rows
- **Parameter Headers**: Yellow + bold styling for easy identification
- **Visual Separators**: Horizontal lines between parameter groups
- **Adjustable Column Widths**: Use `[` and `]` to resize columns, `,` and `.` to switch focused column
- **Resizable Panes**: Adjust tree/detail pane split with `+`/`-` keys (20-80% range)
- **Scrollbars**: Indicators when content exceeds viewport
- **Tab Navigation**: Switch between detail tabs with h/l when detail pane is focused
- **DOP Popup**: Press Enter on DOP rows to view detailed Data Object Property information
- **Help Popup**: Press `?` to view all keyboard shortcuts and controls
- **Breadcrumb Navigation**: Shows current location in tree hierarchy at the top
- **Clickable Breadcrumbs**: Click breadcrumb segments to jump to parent nodes
- **Service Reference Links**: Click service references in not-inherited sections to navigate directly to services

### 🔗 Cross-Reference Navigation
Blue-colored cells indicate navigable references. Press Enter or double-click to jump:
- **Parameter Names**: Navigate to the parameter's tree node (by ID)
- **DOP References**: Navigate to the Data Object Property in the tree
- **Service Names**: Navigate to the service or job node in the tree
- **Container References**: Navigate to variants, layers, or functional groups
- **Parent References**: Navigate to the referenced parent container
- **Functional Class References**: Navigate to the functional class node
- **Navigation History**: Press Backspace to go back to previously visited nodes

### 🎯 Interactive Features
- **Sortable Tables**:
  - Click column headers to sort (ascending/descending)
  - Press `s` when detail pane is focused to sort by current column
  - Sort indicators show current sort column and direction (▲/▼)
  - DiagComm sorting: toggle between ID-based and name-based with `s`
- **Double-Click Actions**:
  - Tree nodes: expand/collapse
  - Blue cells: navigate to referenced element (DOP, parameter, service, etc.)
  - Service references: navigate to service
- **Mouse Scroll**: Scroll wheel support for both tree and detail panes
- **Toggle Mouse Support**: Press `m` to enable/disable mouse input

## Configuration

The color scheme is fully configurable via a TOML file. On startup, `mdd-ui` looks for a config file at the platform-specific location (or use `--theme` to specify a custom path):

| Platform | Path |
|----------|------|
| Linux | `$XDG_CONFIG_HOME/mdd-ui/config.toml` (usually `~/.config/mdd-ui/config.toml`) |
| macOS | `~/Library/Application Support/mdd-ui/config.toml` |

If the file does not exist, built-in defaults are used. All fields are optional — you only need to specify the colors you want to change.

See [`config.example.toml`](config.example.toml) for the full list of available settings with their default values.

Colors can be specified as:
- **Named colors**: `red`, `blue`, `cyan`, `darkgray`, `lightgreen`, etc.
- **Hex RGB**: `#ff00ff` or `ff00ff`
- **ANSI 256-color index**: `123` (0–255)

## Installation

### Prerequisites
- Rust 1.88 or later (edition 2024)

### Build from Source
```bash
git clone https://github.com/alexmohr/mdd-ui.git
cd mdd-ui
cargo build --release
```

The binary will be available at `target/release/mdd-ui`.

## Usage

```bash
mdd-ui <path-to-mdd-database>
```

### Options

| Option | Description |
|--------|-------------|
| `--theme <path>` | Path to a custom theme configuration file (TOML format) |

### Example
```bash
# Load and browse an MDD database
mdd-ui /path/to/diagnostic.mdd

# Load with a custom theme
mdd-ui --theme ~/my-theme.toml /path/to/diagnostic.mdd

# Navigate with keyboard or mouse
# Press ? for help popup with all controls
```

### Keyboard Controls

#### Tree Navigation (Default Focus)
| Key | Action |
|-----|--------|
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `h` / `Left` | Collapse node or go to parent |
| `l` / `Right` | Expand node |
| `Space` | Toggle expand/collapse |
| `Enter` | Navigate to parent ref target or expand node |
| `e` | Expand all nodes |
| `c` | Collapse all nodes |
| `s` | Toggle DiagComm sorting (by ID or by name) |
| `Home` | Jump to first node |
| `End` | Jump to last node |
| `PageUp` / `PageDown` | Page through nodes |
| `a-z`, `0-9` | Type-to-jump: jump to next tree node matching typed text |

#### Detail Pane Navigation (When Focused)
| Key | Action |
|-----|--------|
| `Shift+J` / `Down` | Move to next row |
| `Shift+K` / `Up` | Move to previous row |
| `Shift+H` / `Left` | Switch to previous tab |
| `Shift+L` / `Right` | Switch to next tab |
| `Shift+S` | Sort by focused column (toggle direction) |
| `Enter` | Navigate to referenced element (blue cells: DOP, parameter, service, etc.) |
| `Home` | Jump to first row in section |
| `End` | Jump to last row in section |
| `PageUp` / `PageDown` | Page through rows |
| `[` / `]` | Resize focused column (decrease/increase width) |
| `,` / `.` | Switch focused column (left/right) |

#### Search
| Key | Action |
|-----|--------|
| `/` | Start new search (adds to search stack) |
| `Enter` | Finalize search (while in search mode) |
| `n` | Next search match |
| `N` | Previous search match |
| `Shift+S` | Cycle search scope (All → Variants → Services → DiagComms) |
| `x` | Clear entire search stack |
| `Backspace` | Pop last search from stack (when search input is empty) |

#### General
| Key | Action |
|-----|--------|
| `Tab` | Toggle focus between tree and detail pane |
| `Backspace` | Navigate back in history |
| `+` / `=` | Increase tree pane width (max 80%) |
| `-` / `_` | Decrease tree pane width (min 20%) |
| `m` | Toggle mouse support on/off |
| `?` | Show help popup with keyboard shortcuts |
| `Q` / `Esc` | Quit (or close popup/search mode if open) |
| `Ctrl+C` | Force quit |

#### Mouse Controls (when enabled)
| Action | Effect |
|--------|--------|
| Click tree node | Select and focus tree node |
| Double-click tree node | Toggle expand/collapse |
| Click tab | Switch to that tab |
| Click table header | Sort by that column (toggle ascending/descending) |
| Click detail row | Select that row |
| Double-click detail row | Show DOP popup (if applicable) |
| Scroll wheel | Scroll tree or detail pane |

## Structure

The UI is organized into three main areas with hierarchical navigation:

### Tree View Hierarchy
The tree view provides a complete hierarchical representation of the MDD database:

#### **Top-Level Sections**
- **Variants**: ECU variants (base and specific configurations)
  - Each variant contains:
    - Summary tab with variant metadata
    - Parent Refs (if present) with not-inherited element tabs
    - Functional Classes (with cross-variant service aggregation)
    - Diag-Comms (diagnostic services and single ECU jobs)
    - Requests (services with request parameters)
    - Pos-Responses (services with positive responses)
    - Neg-Responses (services with negative responses)
    - State Charts (state machines and transitions)
    - ComParam Refs (communication parameters)

- **Functional Groups**: Diagnostic functional groupings
  - Each group contains similar structure to variants

- **ECU Shared Data**: Shared diagnostic data across functional groups
  - Deduplicated references to shared layers

- **Protocols**: Communication protocol definitions
  - Protocol-level diagnostic layers

#### **Service Display**
- **DiagComms**: Main diagnostic services section
  - Services sorted by ID (default) or name (toggle with `s`)
  - Shows: Service ID (0xXX), Sub-function ID (if present), Service name
  - Inherited services shown with source layer name and gray styling
  - Single ECU Jobs listed after services

- **Requests/Responses Sections**: Filtered views
  - Same services as DiagComms but filtered by capability
  - Maintains ID-based or name-based sorting

### Detail Panes
When a node is selected, the detail pane displays comprehensive information in a tabbed interface:

#### **Service Detail Tabs**
1. **Overview**: Service-level information
   - Service ID and sub-function ID
   - Semantic description
   - Diagnostic layer reference

2. **Request**: Request parameters (if present)
   - Parameter name, byte position, bit position
   - Semantic description
   - Coded values (hex formatted)
   - DOP references (clickable for details)

3. **Positive Response(s)**: One or more tabs for positive responses
   - Multiple response definitions displayed as separate tabs
   - Same parameter detail format as Request

4. **Negative Response(s)**: NRC definitions
   - Negative Response Code parameters
   - Error condition definitions

5. **ComParam-Refs**: Communication parameter references
   - Parameter values and complex values
   - Protocol and protocol stack information

6. **Audience**: Service audience flags
   - Manufacturing, Development, Aftersales, Aftermarket indicators

#### **Variant Detail Tabs**
1. **Summary**: Basic variant information
   - Variant name, base variant status
   - Short name and long name

2. **Parent Refs** (if present): Parent reference information
   - Reference type (Variant, ECU Shared Data, Protocol, Functional Group)
   - Reference short name

#### **Parent Refs Detail Tabs**
1. **Parent Refs List**: Basic reference information
2. **Not Inherited DiagComms**: Services not inherited from parent (clickable references)
3. **Not Inherited DiagVariables**: Variables excluded from inheritance
4. **Not Inherited Dops**: Data Object Properties not inherited
5. **Not Inherited Tables**: Tables excluded from inheritance

#### **State Chart Detail**
- **State Transitions**: Transition name, source state, target state
- **States**: List of all states in the chart

#### **Functional Class Detail**
- **Class Definition**: Functional class metadata
- **Services/Jobs**: All services and jobs belonging to this class (aggregated across variants)

### Navigation Features
- **Independent Tab Navigation**: Each tab maintains its own scroll position and cursor
- **Row Selection**: Navigate within tables using j/k keys
- **Column Focus**: Switch focused column with `,` and `.` for resizing
- **Breadcrumb Trail**: Shows current location in hierarchy (clickable)
- **Service References**: Click references to jump directly to service definitions
- **History Support**: Navigate back through visited nodes (internal tracking)



## Technical Details

### Dependencies
- **ratatui**: Terminal UI framework (v0.30.0)
- **crossterm**: Terminal manipulation (v0.29.0)
- **cda-database**: MDD database parsing and access (from eclipse-opensovd)
- **cda-interfaces**: MDD interface definitions (from eclipse-opensovd)
- **clap**: Command-line argument parsing
- **anyhow**: Error handling
- **serde**: Configuration deserialization
- **toml**: TOML configuration parsing
- **dirs**: Platform-specific configuration directory lookup

### Architecture
- **Event-driven**: Responsive keyboard and mouse input handling with 10ms polling
- **Stateful**: Maintains tree expansion, cursor positions, scroll states, search stack, and sort preferences
- **Efficient**: Only renders visible content with viewport clipping
- **Flexible**: Table-based layout adapts to terminal size with dynamic column constraints
- **Interactive**: Full mouse support with click, double-click, and scroll detection
- **Hierarchical Search**: Stack-based search system for complex filtering with scope awareness
- **Customizable**: Resizable columns (per-section), adjustable pane widths (20-80%), sortable tables, and configurable color theme via TOML
- **Navigation History**: Internal tracking of visited nodes with expand/collapse state restoration for back navigation
- **Cross-Reference Metadata**: Per-cell jump targets encode navigation intent (parameter ID, DOP name, tree node name, container name)
- **Multi-Tab State**: Independent scroll and cursor state per detail tab/section

## Development

### Building
```bash
cargo build
```

### Running in Development
```bash
cargo run -- <path-to-mdd-database>
```

### Key Features to Test
- **Mouse support**: Click nodes, tabs, rows, and table headers; scroll with mouse wheel; double-click for expand/navigate
- **Table sorting**: Click column headers to sort, or press `s` when column is focused; toggle ascending/descending
- **Search stack**: Use `/` to add searches, `x` to clear all, Backspace to pop last search
- **Search scopes**: Press Shift+S to cycle through All/Variants/FunctionalGroups/EcuSharedData/Services/DiagComms/Requests/Responses
- **Column resizing**: Use `[`/`]` to resize columns in detail pane; `,`/`.` to switch focused column
- **Pane resizing**: Use `+`/`-` to adjust tree/detail split (20-80% range)
- **Help popup**: Press `?` to view all keyboard controls and shortcuts
- **DOP popup**: Press Enter on rows with DOP references to view detailed data operation information
- **Breadcrumb navigation**: Click breadcrumb segments at the top to jump to parent nodes
- **Service references**: Click service references in Parent Refs not-inherited sections
- **DiagComm sorting**: Press `s` in DiagComm section header to toggle ID/name sorting
- **Tab navigation**: Use `h`/`l` to switch between detail tabs when detail pane is focused
- **Inherited services**: Services from parent refs shown in gray with source layer name


## License

MIT

## Credits

Built with:
- [ratatui](https://ratatui.rs/) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation
- [cda-database](https://github.com/eclipse-opensovd/classic-diagnostic-adapter) - MDD database library
