<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
-->

# Code Review

Review of all source files as of the `feature` branch (PR #11).

---

## Legend

| Severity | Meaning |
|----------|---------|
| **BUG** | Incorrect behaviour or silent data corruption |
| **DESIGN** | Structural decision that harms maintainability or correctness |
| **PERF** | Avoidable performance cost |
| **STYLE** | Violation of the project style guide or minor clean-up |

---

## Bugs / Correctness

### BUG-1 — `handle_tab_click` uses byte length for tab-width calculation
**File:** `src/app/mouse/clicks.rs` — `handle_tab_click`

```rust
let tab_width: usize = tab_str.len().saturating_add(1); // +1 for separator
```

`String::len()` returns the UTF-8 byte count, not the display column width.
Tab titles with multi-byte characters will produce wrong column positions, causing clicks to resolve to the wrong tab.
The same bug was fixed in the breadcrumb path (commit "fix(B-2)") by using `.chars().count()`.

**Fix:** Replace `.len()` with `.chars().count()` in the tab layout and position calculations
inside `handle_tab_click` and `calculate_tab_lines`.

---

### BUG-2 — `sort_diagcomm_nodes_in_place` permanently removes duplicate services
**File:** `src/app/sort.rs` — `sort_diagcomm_nodes_in_place`

```rust
let mut seen_names = std::collections::HashSet::new();
services.retain(|service| {
    let name = extract_service_name(&service.text);
    seen_names.insert(name.to_owned())
});
```

Deduplication mutates `all_nodes` in memory. Toggling sort direction (ID ↔ name) after the
first call re-sorts the already-reduced list. The removed services are gone until the application
is restarted. Additionally the Diag-Comms header text is rewritten to reflect the new count:

```rust
header_node.text = format!("Diag-Comms ({new_count})");
```

This is a destructive side-effect inside a sort function that users cannot undo.

**Fix:** Remove deduplication from sorting.

---

### BUG-3 — Horizontal scroll clips constraints but not data-row cells
**File:** `src/app/render/table.rs` — `apply_horizontal_scroll`

```rust
// Since ratatui Row doesn't let us extract cells after construction,
// we return the original rows and let the Constraint::Length handle clipping.
// Columns outside the viewport simply won't have space allocated.
let data_rows: Vec<Row<'static>> = visible_rows.to_vec();
```

The header row correctly filters to `vis_col_indices`, but data rows are returned unmodified.
Ratatui allocates each cell a zero-width column when the matching constraint is absent, so the
output is visually correct for most terminals — but the cell count mismatch between header and
data rows violates the table invariant and can cause misaligned rendering for some terminal
emulators.

**Fix:** Rebuild data rows keeping only the cells whose column index is in `vis_col_indices`,
matching the approach used for the header row. This requires iterating the `DetailRow` source
data (which is still accessible at that point) rather than converting to `Row<'static>` before
applying the filter.

---

### BUG-4 — `jump_to_matching_tree_node` bypasses `set_tree_cursor`
**File:** `src/app/mod.rs` — `jump_to_matching_tree_node`

```rust
self.tree.cursor = target;
self.reset_detail_state();
self.tree.scroll_offset = self.tree.cursor.saturating_sub(SCROLL_CONTEXT_LINES);
```

Every other tree cursor mutation goes through `set_tree_cursor`, which guards `reset_detail_state`
on an actual change. This path calls `reset_detail_state` unconditionally, clearing jump buffer,
column widths and section state even when the cursor does not move (unlikely here, but the
inconsistency makes future refactors error-prone).

**Fix:** Use `set_tree_cursor(target)` and update `scroll_offset` afterwards, the same pattern
used in `page_down`, `home`, `end`, etc.

---

## Design Issues

### DESIGN-1 — `get_column_widths` duplicates `normalize_column_widths`
**File:** `src/app/render/table.rs` — `get_column_widths` (lines ~620–665)

The ~25-line normalization block in `get_column_widths` is a near-copy of
`column_widths.rs::normalize_column_widths` and `initialize_column_widths`.
Any fix to the rounding logic must be applied in two places.

```rust
// In table.rs — duplicated inline
if total > 0 && total != 100 {
    let scaled_widths = widths.iter().map(|&w| { ... }).collect();
    widths = scaled_widths;
    let new_total: u16 = widths.iter().sum();
    if new_total != 100 && !widths.is_empty() { ... }
}
```

**Fix:** Call `normalize_column_widths` (make it `pub(crate)`) from `get_column_widths` instead
of inlining the logic.

---

### DESIGN-2 — Unused return value in `apply_horizontal_scroll`
**File:** `src/app/render/table.rs` — `render_hscrolled_table`

```rust
let (vis_constraints, vis_header, vis_rows, _first_vis_col) =
    self.apply_horizontal_scroll(...);
```

`first_vis_col` is always discarded. It appears in the return type of `apply_horizontal_scroll`
but is never used.  Dead return values widen the API surface without benefit and confuse readers.

**Fix:** Remove `first_vis_col` from the return tuple of `apply_horizontal_scroll`.

---

### DESIGN-3 — `jump_buffer` is shared across two independent modes
**File:** `src/app/mod.rs` — `TableState`

`TableState::jump_buffer` and `jump_buffer_time` are used for both:
1. Type-to-jump within the detail table (finding a row by prefix), and
2. Type-to-jump within the tree (finding a tree node by text).

These are independent modes — a user switching focus between tree and detail pane finds the
buffer persists from the previous mode, which might jump to an unintended position.

**Fix:** Store `jump_buffer`/`jump_buffer_time` in an independent struct (e.g.,
`JumpState { buffer: String, time: Option<Instant> }`) and keep separate instances for tree and
detail, or at minimum clear the buffer on focus switch in `handle_normal_key` (Tab key handler).

---

### DESIGN-4 — `resolve_path` duplicates `ensure_node_visible` expansion logic
**File:** `src/app/history.rs` — `resolve_path`

```rust
if let Some(node) = self.tree.all_nodes.get_mut(idx)
    && node.has_children
{
    node.expanded = true;
}
```

`navigate_to_previous_in_history` calls `resolve_path`, which manually expands nodes inline.
The fast path then calls `ensure_node_visible` which does the same thing with a backward walk.
The expansion logic therefore lives in two places with subtly different traversal strategies.

**Fix:** `resolve_path` should only locate the node index; call `ensure_node_visible` afterwards
(as the fast path already does) to expand ancestors, then rebuild the visible list once.

---

### DESIGN-5 — `CompositeBlock` variants repeat the same table fields
**File:** `src/app/render/composite.rs`

```rust
pub(super) enum CompositeBlock<'a> {
    TitledTable {
        title: String,
        header: &'a DetailRow,
        rows: &'a [DetailRow],
        constraints: &'a [crate::tree::ColumnConstraint],
    },
    Table {
        header: &'a DetailRow,
        rows: &'a [DetailRow],
        constraints: &'a [crate::tree::ColumnConstraint],
    },
    PlainText { lines: &'a [String] },
}
```

`TitledTable` and `Table` share three identical fields. Adding a new field (e.g.,
`use_row_selection`) requires updating two arms. A shared `TableBlock<'a>` struct would
eliminate the duplication.

**Fix:**
```rust
struct TableBlock<'a> {
    header: &'a DetailRow,
    rows: &'a [DetailRow],
    constraints: &'a [crate::tree::ColumnConstraint],
}

enum CompositeBlock<'a> {
    TitledTable { title: String, table: TableBlock<'a> },
    Table(TableBlock<'a>),
    PlainText { lines: &'a [String] },
}
```

---

### DESIGN-6 — `debug_assert!(false, …)` for impossible-in-production state
**File:** `src/app/render/composite.rs` — `build_composite_blocks`

```rust
DetailContent::Composite(_) => {
    debug_assert!(false, "nested Composite not supported");
}
```

`debug_assert!(false, …)` is silently a no-op in release builds; the arm falls through
to `i = i.saturating_add(1)` and skips the subsection without any visible indication.
`unreachable!` or an explicit log/error message would be more robust.

**Fix:** Replace with:
```rust
DetailContent::Composite(_) => {
    // Nested Composite should never be produced by the builder.
    // Log and skip in release; panic in debug.
    debug_assert!(false, "nested Composite not supported");
    // fallthrough: skip this subsection
}
```
Or simply: `unreachable!("nested Composite")` if the invariant is guaranteed.

---

### DESIGN-7 — `LayoutCache` mixes cached rendering data with persistent settings
**File:** `src/app/mod.rs` — `LayoutCache`

`LayoutCache` holds two conceptually different things:
1. **Persistent layout settings** that the user controls: `tree_width_percentage`.
2. **Per-frame cached data** that is purely derived: `breadcrumb_segments`, `tab_area`,
   `cached_ratatui_constraints`, etc.

Mixing these makes it unclear which fields survive a `LayoutCache::default()` reset and which
are intentionally sticky.

**Fix:** Split into `LayoutSettings { tree_width_percentage }` (persisted, lives on `App`) and
`RenderCache { tab_area, breadcrumb_segments, … }` (derived, reset each frame or on change).

---

### DESIGN-8 — `build_scrolled_header_row` nearly duplicates `build_header_row`
**File:** `src/app/render/table.rs`

Both methods format header cells with sort indicator + `\n{column_name}` and apply the same
bold/themed style.  The only difference is that `build_scrolled_header_row` filters by
`vis_col_indices` while `build_header_row` iterates all columns.

**Fix:** Unify into one method accepting an optional column-index filter:
```rust
fn build_header_row_impl(
    &self,
    header: &DetailRow,
    col_indices: impl Iterator<Item = usize>,
    sort_state: Option<TableSortState>,
) -> Row<'static>
```

---

## Performance Issues

### PERF-1 — `find_in_hierarchy` is O(N) and called on every Enter/navigation
**File:** `src/app/navigation/helpers.rs` — `find_in_hierarchy`

Every cell-level navigation (Enter in detail pane) calls `find_in_hierarchy`, which performs up
to four linear passes over `all_nodes`. For databases with thousands of nodes this is
perceptible.

**Consideration:** For now the app is single-threaded and interactive; a HashMap index from node
text → index built at tree construction would make lookups O(1) but requires keeping it in sync
with any mutations (`sort_diagcomm_nodes_in_place`, `sort_children_by_name`). This is a
performance optimization for future work, not a blocking issue.

---

### PERF-2 — `find_in_parent_ref_containers` is potentially O(N²)
**File:** `src/app/navigation/helpers.rs` — `find_in_parent_ref_containers`

The method iterates all ParentRef entries in a subtree, and for each one performs a full tree
scan to find the ParentRef's container, then searches its subtree. In the worst case this is
O(P × N) where P is the number of parent refs.

**Same mitigation as PERF-1** — a name-to-index map would reduce this to O(P × subtree_size).

---

### PERF-3 — `build_node_path` is O(N) on every navigation push to history
**File:** `src/app/history.rs` — `build_node_path`

The backward scan from `node_idx` to build the ancestor path is O(N) per navigation action.
The deduplication check also compares the entire `node_path` Vec:

```rust
if self.history.entries.back().is_some_and(|e| e.node_path == path)
```

**Consideration:** For typical databases with <10 000 nodes and human navigation speeds this is
imperceptible. A parent-index sidebar on `TreeNode` (currently absent) would reduce path
construction to O(depth) ≈ O(log N).

---

## Style Issues

### STYLE-1 — `sort_diagcomm_nodes_in_place` uses a manual while loop
**File:** `src/app/sort.rs`

```rust
let mut i = 0;
while i < self.tree.all_nodes.len() {
    ...
    i = section_start.saturating_add(inserted_count);
}
```

The style guide prefers iterator chains over for / while loops. Here the manual index
arithmetic is needed because `drain` and `splice` modify the Vec in place, making a normal
iterator unsafe. The correct approach is to collect the ranges of Diag-Comms sections first,
then process them in reverse order (so indices don't shift):

```rust
// Collect (section_start, section_end) pairs for all Diag-Comms sections, reverse order
let sections: Vec<(usize, usize)> = self.tree.all_nodes
    .iter()
    .enumerate()
    .filter(|(_, n)| Self::is_service_list_type(n, ServiceListType::DiagComms))
    .map(|(i, n)| {
        let depth = n.depth;
        let start = i + 1;
        let end = ...; // subtree_range logic
        (start, end)
    })
    .collect();

for (start, end) in sections.into_iter().rev() { ... }
```

---

### STYLE-2 — `sort_children_by_name` toggles sort by checking current order
**File:** `src/app/sort.rs`

```rust
let already_ascending = child_groups.windows(2).all(|w| { ... a <= b });
if already_ascending {
    child_groups.sort_by(|a, b| b_text.cmp(&a_text)); // descending
} else {
    child_groups.sort_by(|a, b| a_text.cmp(&b_text)); // ascending
}
```

Checking the sort order of the children to decide the next sort direction is fragile — it
depends on the children being in one of exactly two states (all ascending or not). Mixed-order
children are always sorted ascending regardless of intent.

**Fix:** Store an explicit `SortDirection` per node (or in a `HashMap<usize, SortDirection>`
keyed by parent index) so the toggle is deterministic.

---

### STYLE-3 — `render/table.rs::get_column_widths` uses `is_none_or` on a method reference
**File:** `src/app/render/table.rs`

```rust
if self.table.column_widths.get(section_idx).is_none_or(Vec::is_empty)
```

`is_none_or` is stable since Rust 1.82 — this is fine. Note this is consistent with the style
guide's `map_or` preference. No change required; flagged for awareness.

---

### STYLE-4 — `render/composite.rs` renders spacing with `y += spacing` after last block
**File:** `src/app/render/composite.rs` — `render_composite_content`

```rust
y = y.saturating_add(render_h).saturating_add(spacing).min(viewport_bottom);
```

Spacing is added after every block including the last, wasting one row of space at the bottom
of the composite view. Minor cosmetic issue.

---

### STYLE-5 — `visibility.rs::apply_search_filter` — `unwrap_or(0)` on `max_depth`
**File:** `src/app/visibility.rs`

```rust
let max_depth = self.tree.all_nodes.iter().map(|n| n.depth).max().unwrap_or(0);
```

`unwrap_or(0)` is correct (empty tree → no nodes → `parent_at_depth` is empty → the subsequent
loop is a no-op). Add a comment to document why 0 is the correct default here, since a reader
might misread it as "the root has depth 0, so ancestors stop at 0" (true, but for a different
reason).

---

### STYLE-6 — `draw_status` builds display text then truncates with `chars().take()`
**File:** `src/app/render/mod.rs` — `draw_status`

```rust
let text = if text.chars().count() > display_width {
    text.chars().take(display_width).collect::<String>()
} else {
    text
};
```

When `text` is already within width (the common case), `chars().count()` still iterates the
entire string. Use `text.char_indices().nth(display_width)` to short-circuit:

```rust
let text = match text.char_indices().nth(display_width) {
    Some((byte_idx, _)) => text[..byte_idx].to_string(),
    None => text,
};
```

---

### STYLE-7 — `config.rs::parse_color` falls through to `Color::White` for unknown strings
**File:** `src/app/config.rs`

```rust
_ => Color::White,
```

An unrecognised color string (e.g., a typo like `"yelow"`) silently falls back to white with no
warning. Users cannot distinguish "I didn't set a color" from "my color string was wrong".

**Fix:** Log a warning to stderr for unrecognised strings:
```rust
_ => {
    eprintln!("Warning: unrecognised color '{s}', using White");
    Color::White
}
```

---

## Positive Observations

- **State decomposition** — `App` is cleanly split into focused sub-structs (`TreeState`,
  `SearchState`, `DetailState`, `TableState`, `MouseState`, `HistoryState`, `LayoutCache`).
  Each struct carries its own capacity helpers (`ensure_section_capacity`, etc.).

- **`SelectedRowContext`** — The borrow-checker-safe pattern for resolving the selected row
  context before any `&mut self` call is idiomatic and effective.

- **`BreadcrumbSegment`** — Converting the breadcrumb from `Vec<(String, usize, u16, u16)>` to
  a named struct was a clear improvement.

- **`first_table` helper** — Delegating the four accessors (`table_rows`, `table_constraints`,
  `table_use_row_selection`, `table_header`) to a single private method is clean.

- **`section_offset()` single source of truth** — The 0-or-1 header offset is now computed in
  one place, preventing the historical divergence between multiple copies.

- **`SearchEntry` struct** — Replacing `Vec<(String, SearchScope)>` with `Vec<SearchEntry>`
  improved readability at all five call sites.

- **`find_in_hierarchy` search order** — The four-priority hierarchy (direct children →
  subtree → enclosing container → parent-refs) correctly mirrors the database structure and
  avoids false cross-section matches.

- **`VecDeque` for history** — O(1) `pop_front` is the correct data structure for a capped
  FIFO history. The previous `Vec::remove(0)` was O(N).

- **`Rect::contains(Position)` for hit-testing** — Unified and clean. The
  `point_in_optional_rect` helper for `Option<Rect>` is a neat extraction.

- **`ecu_name` stored on `App`** — Removing the fragile string-parsing renderer dependency was
  the right call.

- **Error handling** — No `unwrap()` or `expect()` in application logic. All error paths use
  `Result`, `?`, or `map_or`/`unwrap_or_else` with safe defaults.

- **`collapse_all` uses `depth == 0`** — Semantically correct predicate, correctly replacing
  the positional `i == 0` that would have broken for non-root-first layouts.
