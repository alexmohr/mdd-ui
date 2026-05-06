---
marp: true
theme: uncover
paginate: true
backgroundColor: #1a1a2e
color: #eaeaea
style: |
  section {
    font-family: 'Inter', 'SF Pro Display', -apple-system, sans-serif;
    font-size: 28px;
  }
  h1 {
    color: #00d4aa;
    font-size: 1.8em;
    margin-bottom: 0.3em;
  }
  h2 {
    color: #00d4aa;
    font-size: 1.4em;
  }
  h3 {
    color: #7ec8e3;
    font-size: 1.1em;
  }
  code {
    background: #2d2d44;
    color: #ff6b6b;
    padding: 2px 6px;
    border-radius: 4px;
  }
  pre {
    background: #16213e;
    border-radius: 8px;
    padding: 16px;
    font-size: 0.7em;
  }
  pre code {
    background: transparent;
    color: #eaeaea;
  }
  blockquote {
    border-left: 4px solid #00d4aa;
    padding-left: 16px;
    font-style: italic;
    color: #b8b8cc;
  }
  table {
    font-size: 0.75em;
    margin: 0 auto;
  }
  th {
    background: #00d4aa;
    color: #1a1a2e;
  }
  td {
    background: #16213e;
  }
  strong {
    color: #00d4aa;
  }
  em {
    color: #7ec8e3;
  }
  .placeholder {
    background: #2d2d44;
    border: 2px dashed #00d4aa;
    border-radius: 8px;
    padding: 40px;
    text-align: center;
    color: #7ec8e3;
    font-style: italic;
    margin: 16px 0;
  }
  section.title {
    text-align: center;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }
  section.title h1 {
    font-size: 2.2em;
  }
  footer {
    color: #666;
    font-size: 0.5em;
  }
---

<!-- _class: title -->

# Shipping a Fully AI-Coded Project
## What Actually Works

<br>

**Alexander Mohr**

*No pitch. No hype. Just what worked, what didn't, and how to replicate it.*

<!--
Speaker notes:
- Don't oversell. This audience uses AI daily.
- Set expectations: instructive, not impressive.
-->

---

# What We Built

<div class="placeholder">
📸 SCREENSHOT: mdd-ui desktop app — full GUI with tree pane, detail pane, search visible
<br><code>git checkout 3b43c35</code>
</div>

- Cross-platform desktop app for automotive diagnostic databases
- Rust backend · Tauri v2 · Vue 3 + TypeScript frontend
- Diff engine · MCP server · LLM chat · Auto-updater
- Ships on macOS, Linux, Windows

<!--
Speaker notes:
- Let the screenshot speak. Point at the tree, the detail pane, the search.
- Pause after "zero lines of code" — let that register.
-->

---

# What We Built

**I provided the domain knowledge and the architecture.**

**I wrote zero lines of code.**

Today: exactly how that happened — not the impressive parts, the *instructive* parts.

- What setup was required
- What the AI did well
- Where it consistently went wrong
- What you actually need to bring yourself

---

# Timeline: 11 Weeks, Zero to Shipped

```
Feb 18  ─── Initial TUI, 4,700 lines           (AI cold-start)
Feb 19–24 ─ Feature sprint: mouse, search, CI   (AI + human review)
Mar 01  ─── 84-commit refactor day              (AI code review loop)
Mar 03  ─── Diff engine                         (AI multi-layer feature)
Apr 24  ─── MCP server: AI-accessible API
Apr 25  ─── ⚠ Index-based navigation            (engineering intervention)
Apr 25  ─── Workspace split: mdd-core           (architecture decision)
Apr 25  ─── Tauri + Vue GUI, 10,500 lines       (AI full-stack drop)
Apr 26  ─── GUI polish, LLM chat, auto-updater
Apr 27  ─── Cross-platform release CI
May 03  ─── UDS byte protocol                   (domain-expertise-driven)
```

<!--
Speaker notes:
- Point out the ⚠ — that's where engineering mattered most.
- "The AI did the volume. I did the decisions."
-->

---

# The Most Important File in the Project

<div class="placeholder">
📸 SCREENSHOT: .github/copilot-instructions.md — full file in editor, scroll visible
<br><code>git checkout 75707ff</code>
</div>

`.github/copilot-instructions.md` — added day 5.

**This is the most important artifact in the entire project.**

Not the code — *this.*

<!--
Speaker notes:
- Scroll through it slowly on screen if presenting live.
- Let people absorb the length and specificity.
-->

---

# What's In the Instructions File

A **435-line** document covering:

- Project structure ↔ domain model mapping
- What "jump/link" means — use existing nav, no popups
- Clippy warnings are **never silenced** — they are fixed
- When files should be split into module directories
- Complete Rust style guide with do/don't examples:
  - Iterator chains over for-loops
  - `?` over `unwrap`
  - `let...else` for early returns
  - Enums over string dispatch
  - Exhaustive match arms — **no wildcards, ever**
- Ownership: borrow, don't clone
- Newtype pattern for type safety
- When to ask vs. when to just do it

---

# Why It Matters

**Without it:** a different style every session. AI defaults drift in exactly the ways you don't want.

**With it:** consistent code quality across 11 weeks, hundreds of commits, zero memory between sessions.

> "This file is the memory. It's the senior engineer who's always in the room."

Every session starts fresh — the AI has no memory of last time.

This file IS the memory.

---

# Lesson 1

<br>
<br>

> ## Write the instructions file first.
> ## Update it every time you catch a pattern you don't want to see again.
> ## It compounds.

<!--
Speaker notes:
- Pause. This is the single most important takeaway.
-->

---

# The Backlog — Your Other Job

<div class="placeholder">
📸 SCREENSHOT: todo.md with specific requirements
<br><code>git checkout aabee68</code>
</div>

The format matters. Each item: **specific, unambiguous**.

❌  *"improve the detail view"*

✅  *"Static Fields overview table: only short-name column, no category. Detail view shows byte size and fixed number of items."*

<!--
Speaker notes:
- This is the difference between useful AI output and "creative interpretation."
-->

---

# The Backlog — Results

<div class="placeholder">
📸 SCREENSHOT: git log showing commit c0da734 "Implement all todo.md items"
<br><code>git log --oneline | grep "Implement all"</code>
</div>

The AI cleared the entire backlog in **one commit**: all green.

`c0da734  Implement all todo.md items`

That only works when items are precise enough to execute without interpretation.

> **Lesson 2:** Your job is writing the list, not the code.
> Vague backlog → vague code.
> Specific backlog → specific code.

---

# What the AI Is Good At: Cold Starts

<div class="placeholder">
📸 SCREENSHOT: initial commit file tree — src/app/, src/tree/ visible with line counts
<br><code>git checkout c4d2a96</code>
</div>

**Feb 18 — First commit.** 13 files, 4,700 lines of Rust.

- Complete keyboard-navigable split-pane TUI
- Coherent project structure and module layout
- Data model from problem domain description
- Not a scaffold — **working software**

On the first try.

<!--
Speaker notes:
- "The AI bootstrapped what would take me a week of boilerplate in an afternoon."
-->

---

# What the AI Is Good At: Multi-Layer Features

<div class="placeholder">
📸 SCREENSHOT: diff mode with color-coded tree nodes (Added/Removed/Modified)
<br><code>git checkout 3d357da</code>
</div>

**The diff engine** — landed in a single session (Mar 03):

| Commit | Layer |
|--------|-------|
| `f29a126` | FlatBuffers snapshot extraction |
| `50cf5a0` | Comparison engine |
| `209d176` | `DiffStatus` enum on every tree node |
| `e8dd7f6` | Colour-coded rendering |
| `2c04e39` | CLI subcommand restructure |
| `18f9859` | Export-diff plaintext output |

Multiple files · multiple layers · consistent architecture.

---

# What the AI Is Good At: The Tauri Drop

<div class="placeholder">
📸 SCREENSHOT: Tauri GUI first render — tree pane, detail tables, badges
<br><code>git checkout 91d647b</code>
</div>

**Apr 25 — one commit:**

- 34 new files
- 10,525 lines of code
- Tauri v2 backend + full command bridge
- Vue 3 + TypeScript + TailwindCSS
- Bun + Vite build
- Pinia store, 5 components
- `cargo check` clean · `bun run build` clean

---

# But Here's the Thing About That Commit

<div class="placeholder">
📸 SCREENSHOT: git log showing mdd-core extraction immediately before Tauri
<br><code>git log --oneline 6904ae0..91d647b</code>
</div>

```
6904ae0  refactor: extract mdd-core shared library crate
91d647b  feat: add Tauri + Vue 3 desktop GUI (mdd-tauri crate)
```

Before I asked for the GUI, I made **one architectural decision**:

*Extract all business logic into a shared library crate first.*

The AI would have duplicated the logic or called it from the wrong layer.

I made the cut. Then I asked. The one-shot landing happened **because the architecture was right** before the AI touched it.

---

# Lesson 3

<br>
<br>

> ## The AI executes well when the structure is clean.
> ## Your architectural decisions are the prerequisite, not an afterthought.

---

# The AI Code Review Loop

<div class="placeholder">
📸 SCREENSHOT: git log of tagged refactor commits — design(D-1)..style(S-7)
<br><code>git log --oneline --after=2026-02-28 --before=2026-03-02</code>
</div>

**March 1st: 84 commits.** How:

1. Separate AI session → reviews codebase
2. Produces numbered findings:
   - DESIGN-1 through DESIGN-8
   - BUG-1 through BUG-4
   - STYLE-1 through STYLE-7
   - PERF-1 through PERF-2
3. New session → applies every item
4. Each fix = own commit, tagged by finding ID

---

# Why the Review Loop Works

The AI reviewer evaluated against **the instructions file**.

That's where it got its definition of what counts as a bug, a design smell, a style violation.

**Without that standard** → generic suggestions
**With it** → findings specific to your codebase and your conventions

> **Lesson 4:** Periodic AI-reviews-AI sessions are high-leverage.
> Write findings as a numbered list. Apply in a separate session.
> The instructions file makes the review meaningful.

---

# Where It Consistently Goes Wrong

Five patterns the AI repeats across every project, every language, every session.

| Pattern | What the AI does | The fix |
|---------|-----------------|---------|
| String dispatch | `if title.contains("X")` | Enum + exhaustive match |
| Sentinel values | `usize::MAX`, `"-"` | `Option<T>` |
| Wildcard match | `_ => handle_other()` | Ban in instructions |
| Scope creep | Refactors on bugfix | "fix only this" |
| Dependency adds | Crate for 10 lines of stdlib | Require approval |

Knowing them in advance = catch in review, not production.

---

# Failure: Strings Instead of Types

<div class="placeholder">
📸 SCREENSHOT: git diff showing *ByName removal → TreeNodeByIndex
<br><code>git diff 116b339~1..116b339 -- src/</code>
</div>

The AI stored navigation targets as display names:
- `TreeNodeByName` → string-keyed lookup scanning entire tree
- `ContainerByName` → O(n) on every click
- Silently breaks when you sort or rename

**The fix** (`116b339`):

One typed enum: `TreeNodeByIndex { index, short_name }`
One `resolve_all_indices()` pass at build time.
O(1) · compiler-enforced · sort-stable.

---

# Strings: The Key Insight

The AI implemented the fix **correctly and immediately**.

It would **never** have initiated the refactor. Nothing was "broken."

String-based navigation *works in the demo*. It silently breaks when:
- You rename a label
- Names aren't unique
- You sort

**Add to instructions file:** *"Do not use string comparisons for logic. Use enums or structs instead."*

Then watch for violations in review.

---

# Failure: Sentinel Values

<div class="placeholder">
📸 SCREENSHOT: code showing usize::MAX replaced with Option&lt;usize&gt;
<br><code>git show 371db5b</code>
</div>

`usize::MAX` and `u16::MAX` as stand-ins for "not set."

The AI reaches for these naturally.

A senior engineer replaces them with `Option<T>` — the compiler then **forces** you to handle the absent case everywhere.

The AI version silently mishandles it at the one callsite you forgot.

---

# Failure: Wildcard Match Arms

```rust
// What the AI writes — EVERY. TIME.
match status {
    Status::Pending => handle_pending(),
    _ => handle_other(),
}
```

```rust
// What the instructions enforce
match status {
    Status::Pending => handle_pending(),
    Status::Active => handle_active(),
    Status::Completed => handle_completed(),
}
```

You opt out of exhaustiveness checking → next enum variant is silently unhandled.

The instructions ban it: *"Never use wildcard matches. If a wildcard makes sense, ask first."*

The AI respects this perfectly once written down.

---

# Failure: Scope Creep & Dependencies

### Scope creep
Ask the AI to fix a bug → it refactors surrounding code.
Diff is 3x larger than needed.

Fix: *"Fix only this, do not refactor anything else."*

### Dependency creep
AI adds a crate for what 10 lines of stdlib would solve.
It won't ask.

Fix: *"Never modify Cargo.toml without approval."*

---

# Lesson 5

<br>
<br>

> ## These patterns repeat.
> ## Write them into your instructions file as explicit prohibitions with examples.
> ## The AI follows written rules consistently.
> ## It does not self-correct without them.

---

# What You Still Need: Domain Knowledge

<div class="placeholder">
📸 SCREENSHOT: UDS byte grid — domain-specific encoding feature
<br><code>git checkout 3b43c35</code>
</div>

The AI did not know:
- What an MDD file is
- What a DOP (Data Object Parameter) is
- How UDS service IDs are encoded
- What FlatBuffers schemas look like for automotive data

Every meaningful data model decision → downstream of domain expertise.

**The closer your domain is to general software** → more AI can lead.
**The further away** → more you lead, AI follows.

---

# What You Still Need: Architecture Decisions

When to extract a shared library. When to split a module. When to kill a subsystem. When a working implementation is the wrong abstraction.

```
6904ae0  refactor: extract mdd-core shared library crate    ← my decision
91d647b  feat: add Tauri + Vue 3 desktop GUI                ← AI execution
```

That decision determined whether the GUI was a clean extension or a mess of duplicated logic.

> The AI is an excellent **implementer** of a shape you've decided on.
> It is a poor **designer** of the shape itself.

---

# What You Still Need: Consequence Thinking

The AI solves the problem in front of it:

| Today | Six months from now |
|-------|---------------------|
| String navigation works | Breaks when you sort |
| O(n) scan is fast enough | Breaks at scale |
| 1,000-line file is manageable | Unmaintainable |
| `usize::MAX` sentinel works | Silently mishandled |

Engineering = thinking about what "today" becomes under maintenance, at scale, when someone else reads it.

**The AI has no model of "six months from now." You do.**

---

# The Practical Checklist

### Before you write any code:
- ✎ Write instructions file — domain, structure, style, prohibitions
- ✎ Establish lint/format gates — rustfmt, clippy, pre-commit
- ✎ Be specific. Generic instructions → generic code.

### During development:
- ✎ Maintain specific backlog items
- ✎ Review for: strings, sentinels, wildcards, deps, scope creep
- ✎ Update instructions on every new catch

### Periodically:
- ✎ AI code review → numbered findings → apply in second session

### Architecture:
- ✎ Make decisions yourself, before asking the AI
- ✎ The AI cannot see around corners. You can.

---

# The Numbers

| Metric | Value |
|--------|-------|
| First working prototype | **Day 1** |
| Lines at GUI launch | **~15,000+** |
| Time to shippable app | **~11 weeks** |
| AI-generated features | **All of them** |
| Architectural rewrites | **2–3 targeted** |
| Instructions file updates | **Continuous** |
| Lines of code written by hand | **Zero** |

---

# Close

**February 18th:** first commit, 4,700 lines, working terminal app.

**May 3rd:** cross-platform desktop GUI, diff engine, MCP server, LLM chat, auto-updater.

11 weeks. Zero lines written by hand.

The AI is fast, capable, and follows good engineering standards — **if you write them down.**

The parts that require you:
- Domain knowledge
- Architecture decisions
- The instructions file
- Judgment to catch what the AI consistently gets wrong

---

<!-- _class: title -->

<br>
<br>

# Those parts were always the job.

<br>

![width:400px](https://i.imgflip.com/placeholder-it-never-was-meme.jpg)

<!--
Speaker notes:
- Show "it never was" meme.
- 2-second pause. Let it land.
- "Thanks."
-->

---

<!-- _class: title -->

# Thank you.

<br>

**github.com/alexmohr/mdd-ui**

<br>

*Slides, talk script, and the instructions file — all in the repo.*
