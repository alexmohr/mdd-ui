<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
-->

# Shipping a Fully AI-Coded Project: What Actually Works

> 30 minutes of content for a 20-minute slot (quick talker buffer).
> Slide cues in `[SLIDE]`. Screenshot placeholders in `[SCREENSHOT: description | sha]`.

---

## Slide 1 — Title

**Shipping a Fully AI-Coded Project: What Actually Works**

Alexander Mohr

*No pitch. No hype. Just what worked, what didn't, and how to replicate it.*

---

## Slide 2 — What We Built (1.5 min)

[SCREENSHOT: mdd-ui desktop app — full GUI with tree pane, detail pane, and search visible | 3b43c35]

- Cross-platform desktop app for automotive diagnostic databases
- Rust backend, Tauri v2 shell, Vue 3 + TypeScript frontend
- Diff engine, MCP server, embedded LLM chat, auto-updater
- Ships on macOS, Linux, Windows

**I provided the domain knowledge and the architecture.**
**I wrote zero lines of code.**

---

## Slide 3 — Timeline Overview (1.5 min)

[SLIDE: visual timeline]

```
Feb 18  ─── Initial TUI, 4,700 lines (AI cold-start)
Feb 19–24 ─ Feature sprint: mouse, search, DOP, CI
Mar 01  ─── 84-commit refactor day (AI code review loop)
Mar 03  ─── Diff engine lands (multi-layer feature)
Apr 24  ─── MCP server: AI-accessible API
Apr 25  ─── Index-based navigation (⚠ engineering intervention)
Apr 25  ─── Workspace split: mdd-core extraction
Apr 25  ─── Tauri + Vue 3 GUI (10,500 lines in one commit)
Apr 26  ─── GUI polish, LLM chat, auto-updater, branding
Apr 27  ─── Cross-platform release CI
May 03  ─── UDS byte protocol (domain-expertise-driven)
```

11 weeks. Zero lines of code written by hand.

---

## Slide 4 — The Most Important File (3 min)

[SCREENSHOT: .github/copilot-instructions.md — full file scrolled in editor | 75707ff]

`.github/copilot-instructions.md` — added day 5. **This is the most important artifact in the project. Not the code — this.**

A 435-line document covering:

- Project structure ↔ domain model mapping
- What "jump/link" means — existing nav system, no popups, no re-renders
- Clippy warnings are never silenced — they are fixed
- When files should be split into module directories
- Complete Rust style guide with do/don't examples:
  - Iterator chains over for-loops
  - `?` over unwrap
  - `let...else` for early returns
  - Enums over string dispatch
  - Exhaustive match arms — no wildcards, ever
- Ownership conventions: borrow, don't clone
- Newtype pattern for semantic type safety
- When to ask before proceeding vs. when to just do it

---

## Slide 5 — Why the Instructions File Matters (1.5 min)

[SLIDE: two code snippets side by side — "Without instructions" vs "With instructions"]

**Without it:** different style every session. AI defaults are reasonable but inconsistent.

**With it:** consistent code quality across 11 weeks and hundreds of commits.

> "This file is the memory. It's the senior engineer who's always in the room."

Every session starts fresh. The AI has no memory of last time. This file IS the memory.

**Lesson 1: Write this document first. Update it every time you catch a pattern you don't want to see again. It compounds.**

---

## Slide 6 — The Backlog: Your Other Job (2 min)

[SCREENSHOT: todo.md file with specific requirements listed | aabee68]

[SCREENSHOT: git log showing "Implement all todo.md items" commit | c0da734]

The format matters. Each item is specific and unambiguous:

❌ "improve the detail view"

✅ "the Static Fields overview table should have only a short-name column, no category column. The detail view for a Static Field should show byte size and fixed number of items."

The more specific → less back-and-forth → better output.

The AI cleared the entire backlog in one commit: `c0da734 Implement all todo.md items`. One commit, all green.

That only works when items are precise enough to execute without interpretation.

**Lesson 2: Your job is writing the list, not the code. Vague backlog items produce vague code.**

---

## Slide 7 — What the AI Is Good At: Cold Starts (2 min)

[SCREENSHOT: initial commit file structure — src/app/, src/tree/ with line counts | c4d2a96]

**Feb 18. First commit.** 13 files, 4,700 lines of Rust.

A complete, keyboard-navigable, split-pane terminal application — on the first try.

Not a scaffold. Working software. The AI bootstrapped:
- Coherent project structure
- Module layout
- Data model from problem domain description
- Keyboard input handling
- Split-pane TUI layout

That's the fast part. Use it.

---

## Slide 8 — What the AI Is Good At: Multi-Layer Features (2 min)

[SCREENSHOT: diff mode with color-coded tree nodes showing Added/Removed/Modified | f29a126]

**The diff engine** — landed in a single session (Mar 03):

- FlatBuffers snapshot extraction (`f29a126`)
- Comparison engine (`50cf5a0`)
- `DiffStatus` enum threaded through every tree node (`209d176`)
- Colour-coded rendering (`e8dd7f6`)
- CLI subcommand restructure (`2c04e39`)
- Export-diff plaintext output (`18f9859`)

Multiple files, multiple abstraction layers, all consistent with existing architecture.

When the model is clean and instructions are clear → AI extends correctly across layers.

This would have taken a solo dev a week. It took a session.

---

## Slide 9 — What the AI Is Good At: The Tauri Drop (2.5 min)

[SCREENSHOT: the full Tauri GUI on first render — tree pane, detail tables, badges | 91d647b]

**Apr 25: one commit, 34 files, 10,525 lines of code.**

- Tauri v2 backend with full command bridge
- Vue 3 + TypeScript + TailwindCSS frontend
- Bun + Vite build system
- Pinia store, 5 components
- `cargo check --workspace` clean
- `bun run build` clean

But here's the thing about that commit...

---

## Slide 10 — The Prerequisite Nobody Talks About (1.5 min)

[SCREENSHOT: git log showing mdd-core extraction immediately before Tauri commit | 6904ae0]

Before I asked for the GUI, I made one architectural decision:

**Extract all business logic into a shared library crate first.**

```
6904ae0  refactor: extract mdd-core shared library crate
91d647b  feat: add Tauri + Vue 3 desktop GUI (mdd-tauri crate)
```

The AI would have built the GUI and duplicated the logic, or called it from the wrong layer. I made the cut. Then I asked.

The one-shot landing happened *because* the architecture was right before the AI touched it.

**Lesson 3: The AI executes well when the structure is clean. Your architectural decisions are the prerequisite, not an afterthought.**

---

## Slide 11 — The AI Code Review Loop (2.5 min)

[SCREENSHOT: git log showing design(D-1) through style(S-7) commits on Mar 01 | 0442fd0]

**March 1st: 84 commits.** Here's how:

1. Separate AI session reviews the codebase
2. Produces a numbered findings list:
   - DESIGN-1 through DESIGN-8
   - BUG-1 through BUG-4
   - STYLE-1 through STYLE-7
   - PERF-1 through PERF-2
3. Second AI session works through every item
4. Each fix = its own commit, tagged by finding ID

[SCREENSHOT: excerpt of code review findings doc showing specific items | 0442fd0]

The key: the AI reviewer evaluated against the instructions file. That's where it got its definition of what counts as a bug, a style violation, a design smell.

Without that standard → generic suggestions.
With it → findings specific to your codebase and your conventions.

**Lesson 4: Periodic AI-reviews-AI sessions are high-leverage. Write findings as a numbered document, apply in a second session. The instructions file makes the review meaningful.**

---

## Slide 12 — Where It Goes Wrong: Overview (0.5 min)

[SLIDE: table with 5 rows — Pattern | What the AI does | The fix]

| Pattern | What the AI does | The fix |
|---------|-----------------|---------|
| String dispatch | `if title.contains("X")` | Enum + exhaustive match |
| Sentinel values | `usize::MAX`, `"-"` for absent | `Option<T>` |
| Wildcard match | `_ => handle_other()` | Ban wildcards in instructions |
| Scope creep | Refactors surrounding code on bugfix | Explicit "fix only this" |
| Dependency adds | Pulls in a crate for 10 lines of stdlib | Require approval in instructions |

These patterns repeat across every project, every language, every session.

---

## Slide 13 — Failure Pattern: Strings Instead of Types (2 min)

[SCREENSHOT: git diff showing TreeNodeByName removal and TreeNodeByIndex replacement | 116b339]

The most common and most dangerous pattern.

The AI stored navigation targets as display names:
- `TreeNodeByName` — string-keyed lookup scanning entire tree
- `ContainerByName` — O(n) on every click
- `ServiceOrJobByName` — silently breaks on sort

The fix (developer-initiated):
```
116b339  refactor: replace name-based navigation with index-based tree indices
```

One typed enum variant: `TreeNodeByIndex { index, short_name }`
Single `resolve_all_indices()` pass at build time. O(1), compiler-enforced, sort-stable.

The AI implemented the fix correctly and immediately.
**It would never have initiated the refactor. Nothing was "broken."**

---

## Slide 14 — Failure Pattern: Sentinel Values (1 min)

[SCREENSHOT: code showing usize::MAX replaced with Option<usize> | 371db5b]

`usize::MAX` and `u16::MAX` as stand-ins for "not set."

The AI reaches for these naturally. A senior engineer replaces them with `Option<T>`.

The compiler then forces you to handle the absent case everywhere. The AI version silently mishandles it at the one callsite you forgot.

Already in the instructions file: *"handle errors with Result and ?, never unwrap() in production code."* Extend that principle to all sentinel values.

---

## Slide 15 — Failure Pattern: Wildcard Match Arms (1 min)

[SLIDE: code example showing `_ => handle_other()` vs exhaustive match]

```rust
// What the AI writes — EVERY TIME
match status {
    Status::Pending => handle_pending(),
    _ => handle_other(),   // ← opts out of exhaustiveness
}

// What the instructions enforce
match status {
    Status::Pending => handle_pending(),
    Status::Active => handle_active(),
    Status::Completed => handle_completed(),
}
```

The instructions ban it explicitly: *"Never use wildcard matches. If a wildcard makes sense, ask first."*

The AI respects this constraint perfectly once it's written down.

---

## Slide 16 — Failure Pattern: Scope Creep & Dependencies (1 min)

[SLIDE: two bullet points with examples]

**Scope creep:** Ask the AI to fix a bug → it refactors surrounding code. Diff is 3x larger than needed.

Fix: *"fix only this, do not refactor anything else."*

**Dependencies:** The AI adds a crate to solve what ten lines of stdlib would handle. Won't ask.

Fix: *"Never modify dependencies in Cargo.toml without approval."*

Both are already in the instructions file. The AI follows written rules. It does not self-correct without them.

**Lesson 5: Write failure patterns into your instructions file as explicit prohibitions with examples. The AI follows written rules consistently. It does not self-correct without them.**

---

## Slide 17 — What You Still Need: Domain Knowledge (1.5 min)

[SLIDE: three items — Domain, Architecture, Consequences]

**The AI did not know:**
- What an MDD file is
- What a DOP is
- What a FlatBuffers schema looks like for automotive data
- How UDS service IDs are encoded

[SCREENSHOT: UDS byte grid feature showing domain-specific encoding | 3b43c35]

Every meaningful data model decision was downstream of domain expertise.

The closer your domain is to general software → the more AI can lead.
The further away → the more you lead, AI follows.

---

## Slide 18 — What You Still Need: Architecture Decisions (1.5 min)

[SLIDE: diagram showing mdd-core extraction enabling GUI without duplication]

When to extract a shared library. When to split a module into a directory. When to kill a subsystem. When a working implementation is the wrong abstraction.

The `mdd-core` extraction:
```
6904ae0  refactor: extract mdd-core shared library crate
91d647b  feat: add Tauri + Vue 3 desktop GUI
```

That decision determined whether the GUI was a clean extension or a mess of duplicated logic.

The AI is an excellent implementer of a shape you've decided on. It is a poor designer of the shape itself.

---

## Slide 19 — What You Still Need: Consequence Thinking (1.5 min)

[SLIDE: "works today" vs "works at scale" comparison]

The AI solves the problem in front of it:
- String navigation works today ← breaks when you sort
- O(n) scan is fast enough today ← breaks at scale
- 1,000-line file is manageable today ← unmaintainable in 3 months

Engineering = thinking about what "today" becomes under maintenance, at scale, when someone else reads it.

The AI has no model of "six months from now." You do.

That's why the instructions file matters. It encodes your consequence thinking into a form the AI can act on today.

---

## Slide 20 — The Practical Checklist (2 min)

[SLIDE: checklist — copy this]

### Before you write any code:
- [ ] Write your instructions file — domain conventions, structural rules, style guide with do/don't examples, explicit prohibitions
- [ ] Establish lint/format gates early — nightly rustfmt, clippy all-warnings-as-errors, pre-commit hooks
- [ ] Be specific. Generic instructions produce generic code.

### During development:
- [ ] Maintain a specific backlog — not "improve X" but "X should show columns A and B, detail view includes D and E"
- [ ] Review every session for known failure patterns: strings, sentinels, wildcards, deps, scope creep
- [ ] Update instructions file when you catch something new — one catch, one rule, never see it again

### Periodically:
- [ ] Run AI code review → numbered findings list → apply in separate session
- [ ] Evaluate findings yourself — some will be wrong or not worth fixing

### Architecture calls:
- [ ] Make them yourself, before asking the AI to implement
- [ ] The AI cannot see around corners. You can.

---

## Slide 21 — The Numbers (1 min)

[SLIDE: metrics table]

| Metric | Value |
|--------|-------|
| First working prototype | Day 1 |
| Lines at GUI launch | ~15,000+ |
| Time to shippable desktop app | ~11 weeks |
| AI-generated features | Essentially all |
| Architectural rewrites required | 2–3 targeted interventions |
| Instructions file updates | Continuous |

---

## Slide 22 — Close (1 min)

[SLIDE: timeline — Feb 18 to May 3]

February 18th: first commit, 4,700 lines, working terminal app.
May 3rd: cross-platform desktop GUI, diff engine, MCP server, LLM chat, auto-updater.

11 weeks. Zero lines of code written by hand.

The AI is fast, capable, and will follow good engineering standards — if you write them down.

The parts that require you:
- Domain knowledge
- Architecture decisions
- The instructions file
- Judgment to catch what the AI consistently gets wrong

---

## Slide 23 — The Punchline

[SLIDE: the "it never was" meme]

> "The code was never the job."

…

Thanks.

---

## Appendix — Slide Reference with SHAs

| Slide | Screenshot Needed | Commit SHA |
|-------|-------------------|------------|
| 2 | mdd-ui desktop app — polished GUI | `3b43c35` (latest feature state) |
| 4 | .github/copilot-instructions.md in editor | `75707ff` (instructions added) |
| 6a | todo.md with specific requirements | `aabee68` (todo + instructions added) |
| 6b | "Implement all todo.md items" in git log | `c0da734` |
| 7 | Initial commit file structure | `c4d2a96` |
| 8 | Diff mode with color-coded tree | `f29a126` or `3d357da` |
| 9 | Tauri GUI first render | `91d647b` |
| 10 | git log showing mdd-core → Tauri sequence | `6904ae0` + `91d647b` |
| 11a | git log of tagged refactor commits | `0442fd0` (findings doc) |
| 11b | code review findings doc content | `0442fd0` |
| 13 | Diff: TreeNodeByName → TreeNodeByIndex | `116b339` |
| 14 | Code: usize::MAX → Option | `371db5b` |
| 17 | UDS byte grid (domain-specific) | `3b43c35` |

---

## Timing Budget (30 min content)

| Slide(s) | Section | Time |
|-----------|---------|------|
| 1 | Title | 0.5 min |
| 2–3 | What we built + timeline | 3 min |
| 4–5 | Instructions file | 4.5 min |
| 6 | Backlog | 2 min |
| 7–10 | What AI is good at (3 examples) | 8 min |
| 11 | Code review loop | 2.5 min |
| 12–16 | Where it goes wrong (5 patterns) | 5.5 min |
| 17–19 | What you still need | 4.5 min |
| 20–21 | Checklist + numbers | 3 min |
| 22–23 | Close | 1.5 min |
| **Total** | | **~35 min raw / ~25 min delivered** |

---

## Speaker Notes — Pacing

- Slides 4–5 (instructions file) are the anchor — take your time here, this is the key takeaway
- Slides 12–16 (failure patterns) can be compressed by skipping 14/15 if running long
- Slide 20 (checklist) can be shown without reading aloud — "this is in the slides, copy it later"
- The meme at the end needs a 2-second pause before "Thanks" — let it land
