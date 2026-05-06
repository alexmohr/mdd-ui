<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
-->

# mdd-ui — Project History

> A talk outline for a ~20-minute presentation on an AI-coded success story,
> including honest engineering caveats.

---

## 1 · The Birth — Day 1 (2026-02-18)

**Commit:** `initial commit`

The project starts as a single Rust TUI (terminal-user-interface) application — no GUI, no
crates, no CI. Everything lives in a handful of files:

```
src/app/input.rs   (~196 lines)
src/app/mod.rs     (~470 lines)
src/app/render.rs  (~634 lines)
src/database.rs
src/tree/layers.rs (~627 lines)
src/tree/mod.rs    (~269 lines)
```

Total: ~4 700 lines of code dropped in one shot by an AI pair-programmer, including a
working `README.md` with screenshots. The data model for MDD automotive diagnostic
databases (variants, services, DOPs, protocols, …) is already present in `tree/layers.rs`.

**What the AI did well:** It produced a compilable, structurally coherent program on the
first commit — not a skeleton, but a navigable tree browser with keyboard input handling and
a split-pane TUI layout.

**What the AI could not do:** It could not know the SOVD/MDD domain. The database schema
and field semantics came from the developer's own automotive expertise.

---

## 2 · Rapid Feature Expansion (2026-02-18 → 2026-02-24)

Over the next six days the AI adds features in quick bursts, often several commits per
session:

| Date | What changed |
|------|-------------|
| Feb 18 | Hide-on-search, README revision |
| Feb 19 | Mouse support, tabs wrapping, state charts, tree simplification |
| Feb 20 | Table sorting, DiagComm overview, module reorganisation |
| Feb 21 | Parent refs, navigation history (backspace), breadcrumb clicks, sorting by double-click, composite rendering skeleton, search-on-navigate |
| Feb 22 | DOP (Data Object Parameter) support, Variants overview, scrollbar, jump-to-job |
| Feb 23 | DTC support, DOP improvements, instructions/todo file |
| Feb 24 | License, CI, lint gates, DOP detail views, Unit Spec, Tables, "Implement all todo.md items" |

The AI tracked work in a `todo.md` file and then committed `"Implement all todo.md items"` —
the AI using its own backlog as a planning artifact.

**Engineering insight:** The developer continuously updated `instructions` and `todo.md`
files to steer the AI. The AI wrote code; the human curated requirements.

---

## 3 · The First Architecture Warning — String-Based Navigation (2026-02-21)

Early navigation was driven by `section.title.contains("…")` and
`section.title.starts_with("Not Inherited")` — raw string comparisons to decide which
code path to execute. This is a classic AI code smell: it works, but it is fragile and
untestable.

The developer noticed and committed:

```
refactor: replace string comparisons with type-based checks and fix detail state persistence
```

This is the first example of **engineering-skill-required intervention**: the AI naturally
reaches for strings when a type-safe enum would be correct. Left unchecked, this pattern
scales into a maintenance nightmare.

---

## 4 · The Big Refactor Weekend (2026-03-01)

A single day produced **~60 commits**, all tagged with structured prefixes
(`design(D-x)`, `fix(B-x)`, `style(S-x)`, `perf(P-x)`). This was a methodical
code-review-driven session where an AI reviewer produced a numbered findings list
(`docs: add code review findings`) and then a second AI session applied every fix.

Key changes:

- **`DetailSectionType` enum** replaces all `section.title` string dispatch
- **`BreadcrumbSegment` struct** replaces anonymous `(String, usize)` tuples
- **`SearchEntry` struct** replaces `Vec<(String, SearchScope)>`
- **`TableBlock` / `CompositeBlock`** introduced to remove duplicate fields
- **`set_tree_cursor` helper** eliminates duplicated cursor-reset pattern everywhere
- **`VecDeque` for `HistoryState`** for O(1) pop_front
- **`Rc<[DetailSectionData]>`** to eliminate per-frame deep clones

> **Presentation point:** The AI is excellent at applying a numbered checklist of
> refactors. Give it a list, get clean code. The human's job is writing the list.

---

## 5 · Diff Mode (2026-03-03 → 2026-03-05)

A full snapshot-diff engine is added: FlatBuffers database extraction, a comparison engine,
annotated tree nodes with `DiffStatus`, colour-coded rendering, and an `export-diff` CLI
subcommand. This required restructuring the CLI into `browse` / `diff` subcommands.

The AI implements the whole feature coherently across multiple layers — Rust backend,
tree model, and TUI renderer — in a single session.

---

## 6 · MCP Server — AI Talks to AI (2026-04-24)

```
feat: add optional MCP server for AI-assisted MDD browsing and diffing
```

An [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server is added behind
a Cargo feature flag. This exposes `load_mdd`, `browse_tree`, `get_node_details`,
`search_nodes`, `diff_mdd`, and `export_diff` as tools for AI assistants.

The MDD tool is now a building block for AI-assisted automotive diagnostics workflows.
This commit was co-authored by a Mercedes-Benz engineer (`Florian Roks`), showing real
collaborative multi-developer AI-assisted development.

---

## 7 · The Architectural Turning Point — Index-Based Navigation (2026-04-25)

This is the most important engineering intervention in the project.

**The problem:** Navigation jump targets were stored as strings:
`TreeNodeByName`, `ContainerByName`, `ServiceOrJobByName`. Finding a node meant scanning
the entire tree by display text — O(n), fragile, wrong.

**The fix (developer-initiated refactor):**

```
refactor: replace name-based navigation with index-based tree indices
```

- All `*ByName` variants are deleted from the jump-target enum
- A single `TreeNodeByIndex { index, short_name }` replaces them all
- `TreeBuilder::finish()` gains a `resolve_all_indices()` pass that converts
  sentinel `usize::MAX` placeholders into real indices before the tree is handed off
- Sort operations re-run `resolve_all_indices()` to keep indices stable

**Why this matters for the talk:** This is the clearest example of where engineering
discipline is non-negotiable. The AI will use strings for navigation because strings are
easy and "work in the demo." A senior engineer recognises the O(n) lookup, the fragile
display-text coupling, and the silent breakage on sort — and rewrites it with proper typed
indices before the codebase grows further.

> _"Preventing string-based navigation from surviving code review is not an AI limitation —
> it's an engineering standard."_

The same session also eliminated `u16::MAX` and `usize::MAX` sentinel values, replacing
them with typed `Option` fields.

---

## 8 · Core / GUI Split — Workspace Architecture (2026-04-25)

```
refactor: extract mdd-core shared library crate
```

The business logic (database loading, tree building, diff engine) is extracted into
`crates/mdd-core/`, converting the repo into a Cargo workspace. The TUI becomes a consumer.
All 53 tests pass with no new clippy warnings.

This is a deliberate architectural decision to enable a second frontend — the GUI — without
duplicating logic.

---

## 9 · Tauri + Vue 3 GUI Added (2026-04-25)

```
feat: add Tauri + Vue 3 desktop GUI (mdd-tauri crate)
```

In a single commit, 34 new files and ~10 500 lines of code land:

- **Rust backend**: Tauri v2 commands (`load_mdd`, `get_visible_nodes`, `get_node_detail`,
  `toggle_expand`, `search`, …)
- **Vue 3 + TypeScript + TailwindCSS frontend**: `TreePane`, `DetailPane`, `SearchBar`,
  `StatusBar`
- **Bun** as the JS runtime/bundler

The Tauri crate reuses `mdd-core` for all logic. The TUI and GUI share the same data
model. The AI generated a coherent full-stack desktop application in one pass.

---

## 10 · GUI Polish Sprint (2026-04-25 → 2026-04-26)

Seventeen commits in two days turn the raw GUI into a polished application:

| Commit | What changed |
|--------|-------------|
| `fix(tauri): double-click expand, colored node types` | UX feel |
| `feat(tauri): sort button, color legend, remove type badges` | Visual clarity |
| `feat(tauri): replace color-coded text with badges, neutral theme` | Design overhaul |
| `feat(tauri): fix sorting, add font size control, table column resize & sort` | Power-user features |
| `feat(tauri): INH+type dual badges, keyboard nav, persistent sort` | Keyboard-first UX |
| `feat: 4-way diagcomm sort, Tauri commands, UI prefs, and clippy fixes` | Configuration persistence |
| `refactor: merge mdd-tauri into root mdd-ui crate` | Workspace simplification |
| `feat: remove TUI, keep export-diff and mcp mode` | Strategic decision: GUI wins |

The TUI is retired. The desktop GUI becomes the primary product.

---

## 11 · LLM Chat Panel (2026-04-26)

```
feat: add GitHub Copilot GHE auth and LLM chat with navigation
```

An AI chat panel is embedded directly in the GUI:

- GitHub Copilot (GHE) OAuth Device Flow authentication
- Markdown rendering in chat responses
- `[[NodeName]]` syntax in AI responses navigates the tree live
- Later extended to Azure OpenAI, OpenAI, and AWS Bedrock (`feat(llm): support Azure OpenAI, OpenAI, Bedrock providers`)

The tool that was built with AI can now use AI as a feature.

---

## 12 · Auto-Updater, CLI File Open, App Branding (2026-04-26)

```
feat: add auto-updater via tauri-plugin-updater
feat: open MDD file from CLI argument
chore: rename productName to 'MDD UI'
```

The project becomes a shippable product: self-updating, launchable from the terminal with
a file argument, properly branded.

---

## 13 · Release Pipeline & Cross-Platform CI (2026-04-26 → 2026-04-27)

GitHub Actions workflows are wired for macOS, Linux, and Windows release builds. Several
fixing commits deal with real-world CI pain:

- DMG bundling on macOS requiring `CI=true` in cargo config
- Linux system deps for Tauri build
- Updater JSON job and non-draft release lifecycle
- macOS Gatekeeper workaround documented for unsigned builds
- Linux DMA-BUF workaround, dark theme, taskbar icon fixes

> **Presentation point:** CI/CD is where the AI needs the most human guidance. Platform
> quirks, signing requirements, and build toolchain subtleties are not in training data.
> Each platform fix is a human-spotted issue, AI-applied fix.

---

## 14 · UDS / Byte-Level Protocol Work (2026-05-03 → present)

```
feat: add UDS byte encoding, service lookup, and interactive byte grid
refactor: replace SOVD translation with direct UDS encoding
```

Domain-specific automotive protocol knowledge (UDS byte encoding, service ID lookup) is
added. This is squarely in the territory where the developer's automotive expertise drives
the feature — the AI implements, the engineer specifies.

---

## Summary Timeline

```
Feb 18  ──► Initial TUI, tree browser (AI cold-start)
Feb 19-24 ► Feature sprint: mouse, search, DOP, CI (AI + human review)
Mar 01  ──► 60-commit refactor day: types, enums, structs (AI code review loop)
Mar 03  ──► Diff engine (AI multi-layer feature)
Apr 24  ──► MCP server: AI-accessible API
Apr 25  ──► Index-based navigation (⚠ engineering intervention)
Apr 25  ──► Workspace split: mdd-core (architecture decision)
Apr 25  ──► Tauri + Vue GUI (AI full-stack drop)
Apr 26  ──► GUI polish, LLM chat, auto-updater, branding
Apr 27  ──► Cross-platform release CI
May 03  ──► UDS byte protocol (domain-expertise-driven)
```

---

## Engineering Caveats — Where Human Skill Is Still Required

### 1. Domain Knowledge
The AI cannot know what an MDD file is, what a DOP is, or how UDS service IDs are
encoded. Every domain-specific data model decision required automotive engineering
expertise.

### 2. Type Safety vs. Strings
The AI's default is to use strings for dispatch and navigation. Every `section.title.contains()`
pattern, every `TreeNodeByName` variant, was introduced by the AI and needed a human to
say _"no — use an enum."_ **This is the single most important lesson: review for string
abuse.**

### 3. Sentinel Values
`usize::MAX`, `u16::MAX`, `"-"` as sentinel strings — the AI uses them naturally. A
senior engineer replaces them with `Option<T>`. Budget time for this in every review.

### 4. CI/CD Platform Quirks
macOS code signing, Linux DMA-BUF, Windows strip settings — these require humans who
have actually shipped software on those platforms.

### 5. Architecture Decisions
The extraction of `mdd-core` as a shared crate enabling both TUI and GUI was a deliberate
human decision. The AI would have continued adding to a monolithic binary. The workspace
split required a human to see the future use case.

### 6. Security & Auth
The LLM chat panel borrowed opencode's OAuth Client ID (noted in a disclaimer in the
README). That kind of pragmatic decision — and its risks — requires human judgement.

---

## The AI Success Story

| Metric | Value |
|--------|-------|
| First working prototype | **Day 1** |
| Lines of code at GUI launch | ~15 000+ |
| Time from zero to shippable desktop app | **~11 weeks** |
| Number of AI-generated features | Essentially all of them |
| Number of architectural rewrites required | 2–3 targeted interventions |

The project demonstrates that an AI pair-programmer can carry a non-trivial, multi-layer
desktop application from zero to a shippable product. The human's role shifts from
_writing code_ to _directing architecture, reviewing for type safety, and supplying domain
knowledge_.

> _"The boulder rolls uphill faster with AI. The engineer's job is to make sure it stays on
> the path."_
