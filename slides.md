---
marp: true
theme: uncover
paginate: true
style: |
  /* ═══════════════════════════════════════
     PALETTE — "Engineering Dark"
     bg:      #0f1117  (deep charcoal)
     surface: #1a1d27  (card bg)
     text:    #e2e8f0  (light slate)
     muted:   #94a3b8  (slate-400)
     dim:     #64748b  (slate-500)
     blue:    #3b82f6  (primary)
     amber:   #f59e0b  (lessons)
     green:   #22c55e  (success)
     red:     #ef4444  (failure)
     ═══════════════════════════════════════ */
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800;900&display=swap');

  :root {
    --bg:      #0f1117;
    --surface: #1a1d27;
    --surface2:#252831;
    --text:    #e2e8f0;
    --muted:   #94a3b8;
    --dim:     #64748b;
    --blue:    #3b82f6;
    --blue-d:  #2563eb;
    --amber:   #f59e0b;
    --green:   #22c55e;
    --red:     #ef4444;
    --code:    #fbbf24;
  }

  section {
    font-family: 'Inter', -apple-system, 'Segoe UI', sans-serif;
    font-size: 24px;
    padding: 56px 72px 48px;
    line-height: 1.5;
    color: var(--text);
    background: var(--bg);
    letter-spacing: -0.01em;
  }

  /* ─── Top accent bar ─── */
  section::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: linear-gradient(90deg, var(--blue) 0%, var(--blue-d) 50%, var(--dim) 100%);
    z-index: 10;
  }

  /* ─── Pagination — clean, no block ─── */
  section::after {
    background: transparent !important;
    height: auto !important;
    padding: 0 !important;
    font-size: 0.5em;
    color: var(--dim);
    font-weight: 500;
    pointer-events: none;
  }

  /* ═══════════════════════════════════════
     HEADINGS
     ═══════════════════════════════════════ */
  h1 {
    color: var(--blue);
    font-size: 1.75em;
    font-weight: 800;
    margin-bottom: 0.35em;
    letter-spacing: -0.03em;
  }
  h2 {
    color: var(--blue);
    font-size: 1.15em;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  h3 {
    color: var(--dim);
    font-size: 0.78em;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 0.5em;
  }

  /* ═══════════════════════════════════════
     INLINE CODE
     ═══════════════════════════════════════ */
  code {
    background: rgba(251,191,36,0.1);
    color: var(--code);
    padding: 2px 8px;
    border-radius: 5px;
    font-size: 0.88em;
    border: 1px solid rgba(251,191,36,0.12);
  }

  /* ═══════════════════════════════════════
     CODE BLOCKS
     ═══════════════════════════════════════ */
  pre {
    background: var(--surface);
    border-radius: 10px;
    border: 1px solid var(--surface2);
    border-left: 3px solid var(--blue);
    padding: 16px 22px;
    font-size: 0.6em;
    line-height: 1.6;
    box-shadow: 0 4px 20px rgba(0,0,0,0.3);
  }
  pre code {
    background: transparent;
    color: #e4e4e7;
    padding: 0;
    border: none;
  }

  /* ═══════════════════════════════════════
     BLOCKQUOTE
     ═══════════════════════════════════════ */
  blockquote {
    border-left: 3px solid var(--amber);
    padding: 12px 24px;
    font-style: italic;
    color: #cbd5e1;
    margin: 16px 0;
    background: rgba(245,158,11,0.04);
    border-radius: 0 8px 8px 0;
  }

  /* ═══════════════════════════════════════
     LISTS
     ═══════════════════════════════════════ */
  ul { list-style: none; padding-left: 0; }
  ul > li {
    padding-left: 1.4em;
    position: relative;
    margin-bottom: 0.35em;
  }
  ul > li::before {
    content: '▸';
    position: absolute;
    left: 0;
    color: var(--blue);
    font-weight: bold;
  }
  ol > li {
    margin-bottom: 0.35em;
  }
  ol > li::marker {
    color: var(--blue);
    font-weight: 700;
  }

  /* ═══════════════════════════════════════
     TABLES
     ═══════════════════════════════════════ */
  table {
    font-size: 0.72em;
    margin: 0 auto;
    border-collapse: separate;
    border-spacing: 0;
    width: auto;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 2px 12px rgba(0,0,0,0.25);
  }
  th {
    background: var(--blue);
    color: var(--bg);
    font-weight: 700;
    padding: 10px 20px;
    text-transform: uppercase;
    font-size: 0.82em;
    letter-spacing: 0.04em;
  }
  td {
    background: var(--surface);
    padding: 8px 20px;
    border-bottom: 1px solid rgba(255,255,255,0.04);
  }
  tr:nth-child(even) td {
    background: #1f2230;
  }

  /* ═══════════════════════════════════════
     EMPHASIS
     ═══════════════════════════════════════ */
  strong {
    color: var(--text);
    font-weight: 700;
  }
  em {
    color: #cbd5e1;
  }

  /* ═══════════════════════════════════════
     SCREENSHOT PLACEHOLDERS
     ═══════════════════════════════════════ */
  .placeholder {
    background: rgba(59,130,246,0.04);
    border: 2px dashed rgba(59,130,246,0.3);
    border-radius: 12px;
    padding: 28px 24px;
    text-align: center;
    color: rgba(59,130,246,0.55);
    font-style: italic;
    font-size: 0.72em;
    margin: 12px 0;
    position: relative;
    min-height: 80px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 6px;
  }
  .placeholder::before {
    content: '📷';
    font-size: 1.8em;
    font-style: normal;
    display: block;
    margin-bottom: 4px;
    opacity: 0.6;
  }
  .placeholder code {
    background: rgba(59,130,246,0.08);
    color: rgba(59,130,246,0.6);
    font-size: 0.85em;
    border: 1px solid rgba(59,130,246,0.12);
  }

  /* Two-side placeholder for before/after */
  .placeholder-pair {
    display: flex;
    gap: 16px;
    margin: 12px 0;
  }
  .placeholder-pair .placeholder {
    flex: 1;
    min-height: 160px;
  }

  /* ═══════════════════════════════════════
     BIG NUMBER
     ═══════════════════════════════════════ */
  .big-number {
    font-size: 3.5em;
    font-weight: 900;
    color: var(--blue);
    line-height: 1;
    letter-spacing: -0.04em;
  }

  /* ═══════════════════════════════════════
     PILLS
     ═══════════════════════════════════════ */
  .pill {
    display: inline-block;
    background: rgba(59,130,246,0.12);
    color: var(--blue);
    padding: 3px 14px;
    border-radius: 999px;
    font-size: 0.72em;
    font-weight: 600;
    border: 1px solid rgba(59,130,246,0.18);
  }
  .pill-amber {
    display: inline-block;
    background: rgba(245,158,11,0.12);
    color: var(--amber);
    padding: 3px 14px;
    border-radius: 999px;
    font-size: 0.72em;
    font-weight: 600;
    border: 1px solid rgba(245,158,11,0.18);
  }
  .pill-red {
    display: inline-block;
    background: rgba(239,68,68,0.12);
    color: var(--red);
    padding: 3px 14px;
    border-radius: 999px;
    font-size: 0.72em;
    font-weight: 600;
    border: 1px solid rgba(239,68,68,0.18);
  }

  /* ═══════════════════════════════════════
     SLIDE CLASSES
     ═══════════════════════════════════════ */

  /* ─── Title slide ─── */
  section.title {
    text-align: center;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    background:
      radial-gradient(ellipse at 30% 70%, rgba(59,130,246,0.07) 0%, transparent 50%),
      var(--bg);
  }
  section.title::before {
    height: 0;
  }
  section.title h1 {
    font-size: 2.3em;
    margin-bottom: 0.15em;
  }
  section.title h2 {
    color: var(--muted);
    font-weight: 400;
    font-size: 1.05em;
    margin-bottom: 0.8em;
  }

  /* ─── Lesson slides — amber accent ─── */
  section.lesson {
    display: flex;
    flex-direction: column;
    justify-content: center;
    background:
      radial-gradient(ellipse at 50% 50%, rgba(245,158,11,0.05) 0%, transparent 60%),
      var(--bg);
    text-align: center;
    padding: 56px 88px;
  }
  section.lesson::before {
    background: linear-gradient(90deg, var(--amber) 0%, #d97706 50%, var(--amber) 100%);
  }
  section.lesson h1 {
    color: var(--amber);
    font-size: 1.6em;
    margin-bottom: 0.6em;
  }
  section.lesson blockquote {
    border-left: 3px solid var(--amber);
    text-align: left;
    font-size: 1.1em;
    color: var(--text);
    max-width: 82%;
    margin: 0 auto;
    background: rgba(245,158,11,0.04);
  }

  /* ─── Lead / section break ─── */
  section.lead {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    background:
      radial-gradient(ellipse at 50% 50%, rgba(59,130,246,0.06) 0%, transparent 60%),
      var(--bg);
  }
  section.lead::before {
    background: linear-gradient(90deg, var(--blue) 0%, var(--dim) 100%);
  }
  section.lead h1 {
    font-size: 2.1em;
  }

  /* ─── Danger section break ─── */
  section.danger {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    background:
      radial-gradient(ellipse at 50% 50%, rgba(239,68,68,0.05) 0%, transparent 60%),
      var(--bg);
  }
  section.danger::before {
    background: linear-gradient(90deg, var(--red) 0%, #dc2626 50%, var(--red) 100%);
  }
  section.danger h1 {
    color: var(--red);
    font-size: 2.1em;
  }

  /* ─── Success section ─── */
  section.success {
    background:
      radial-gradient(ellipse at 100% 0%, rgba(34,197,94,0.04) 0%, transparent 50%),
      var(--bg);
  }
  section.success::before {
    background: linear-gradient(90deg, var(--green) 0%, var(--blue) 100%);
  }
  section.success h1 {
    color: var(--green);
  }

  /* ─── Failure slides ─── */
  section.failure {
    background:
      radial-gradient(ellipse at 100% 0%, rgba(239,68,68,0.04) 0%, transparent 50%),
      var(--bg);
  }
  section.failure::before {
    background: linear-gradient(90deg, var(--red) 0%, var(--dim) 100%);
  }
  section.failure h1 {
    color: var(--red);
  }

  /* ─── Showcase slides ─── */
  section.showcase {
    padding: 44px 56px;
  }
  section.showcase::before {
    background: linear-gradient(90deg, var(--blue) 0%, var(--green) 100%);
  }
  section.showcase .placeholder {
    min-height: 240px;
    font-size: 0.8em;
  }

  /* ─── Footer ─── */
  footer {
    color: rgba(255,255,255,0.12);
    font-size: 0.45em;
  }

---

<!-- _class: title -->
<!-- _paginate: skip -->

# Systems Engineers Can Have Nice Things

## Shipping an AI-Coded Desktop App Without Writing Code

<br>

**Alexander Mohr**

*An experience report. No pitch, no hype.*

<!--
Speaker notes:
- Set the tone: peer-to-peer, not a keynote.
- "I'm going to tell you what worked and what didn't."
-->

---

# A Quick Confession

I'm an onboard diagnostics engineer. ECUs, UDS protocols, CAN buses.

**I don't build UIs.** I build the thing the UI talks to.

I had a tooling problem: automotive diagnostic databases are complex, deeply nested, and the existing tools... weren't great.

I needed a visual solution. So I let AI build one.

<!--
Speaker notes:
- Keep it brief. 30 seconds max.
- The audience should think: "OK this person isn't a frontend dev — interesting."
-->

---

<!-- _class: showcase -->

# What We Built

<div class="placeholder">
SCREENSHOT: mdd-ui desktop app — full GUI with tree pane, detail pane, search visible
<br><code>git checkout 3b43c35</code>
</div>

- Cross-platform desktop app for automotive diagnostic databases
- Rust backend · Tauri v2 · Vue 3 + TypeScript frontend
- Diff engine · MCP server · LLM chat · Auto-updater

<!--
Speaker notes:
- Let the screenshot speak. Point at tree, detail pane, search.
- "This looks like it was built by a frontend team. It wasn't."
-->

---

# What We Built

**I provided the domain knowledge and the architecture.**

**I wrote zero lines of code.**

Every line of implementation — AI-generated.
Every architectural decision — mine.

Today: not the impressive parts, the *instructive* parts.

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

<!-- _class: showcase -->

# The Most Important File in the Project

<div class="placeholder">
SCREENSHOT: .github/copilot-instructions.md — full file in editor, scroll visible
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

A <span class="big-number" style="font-size:1.6em">435</span>-line document covering:

- Project structure ↔ domain model mapping
- What "jump/link" means — use existing nav, no popups
- Clippy warnings are **never silenced** — they are fixed
- When files should be split into module directories
- Complete Rust style guide with do/don't examples:
  - Iterator chains over for-loops · `?` over unwrap
  - `let...else` for early returns · Enums over string dispatch
  - Exhaustive match arms — **no wildcards, ever**
- Ownership: borrow, don't clone · Newtype pattern for type safety
- When to ask vs. when to just do it

---

# Why It Matters

**Without it:** a different style every session. AI defaults drift in exactly the ways you don't want.

**With it:** consistent code quality across 11 weeks, hundreds of commits, zero memory between sessions.

> "This file is the memory. It's the senior engineer who's always in the room."

Every session starts fresh — the AI has no memory of last time.
This file IS the memory.

---

<!-- _class: lesson -->

# Lesson 1

> Write the instructions file first.
> Update it every time you catch a pattern you don't want to see again.
> It compounds.

<!--
Speaker notes:
- Pause. This is the single most important takeaway.
-->

---

<!-- _class: showcase -->

# The Backlog — Your Other Job

<div class="placeholder">
SCREENSHOT: todo.md with specific requirements
<br><code>git checkout aabee68</code>
</div>

The format matters. Each item: **specific, unambiguous**.

&#10060;  *"improve the detail view"*

&#9989;  *"Static Fields overview table: only short-name column, no category. Detail view shows byte size and fixed number of items."*

---

<!-- _class: showcase -->

# The Backlog — Results

<div class="placeholder">
SCREENSHOT: git log showing commit c0da734 "Implement all todo.md items"
<br><code>git log --oneline | grep "Implement all"</code>
</div>

The AI cleared the entire backlog in **one commit**: all green.

`c0da734  Implement all todo.md items`

That only works when items are precise enough to execute without interpretation.

> **Lesson 2:** Your job is writing the list, not the code.
> Vague backlog → vague code. Specific backlog → specific code.

---

<!-- _class: lead -->

# What the AI Is Good At

Three patterns where AI exceeded expectations.

---

<!-- _class: success -->
<!-- _class: showcase -->

# Cold Starts

<div class="placeholder">
SCREENSHOT: initial commit file tree — src/app/, src/tree/ visible with line counts
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

<!-- _class: showcase -->

# Multi-Layer Features

<div class="placeholder">
SCREENSHOT: diff mode with color-coded tree nodes (Added/Removed/Modified)
<br><code>git checkout 3d357da</code>
</div>

**The diff engine** — landed in a single session (Mar 03):

| Commit | Layer |
|--------|-------|
| `f29a126` | FlatBuffers snapshot extraction |
| `50cf5a0` | Comparison engine |
| `209d176` | `DiffStatus` enum on every tree node |
| `e8dd7f6` | Colour-coded rendering |

Multiple files · multiple layers · consistent architecture.

---

<!-- _class: showcase -->

# The Tauri Drop: TUI → Desktop GUI

<div class="placeholder-pair">
<div class="placeholder">
SCREENSHOT: TUI terminal app — split pane tree + detail view
<br><code>git checkout c4d2a96</code>
<br><em>Before: Terminal UI</em>
</div>
<div class="placeholder">
SCREENSHOT: Tauri GUI first render — tree pane, detail tables, badges
<br><code>git checkout 91d647b</code>
<br><em>After: Tauri v2 + Vue 3</em>
</div>
</div>

**Apr 25 — one commit:** 34 new files · 10,525 lines of code

<span class="pill">Tauri v2</span> <span class="pill">Vue 3 + TypeScript</span> <span class="pill">TailwindCSS</span> <span class="pill-amber">Pinia</span> <span class="pill-amber">Bun + Vite</span>

`cargo check` clean · `bun run build` clean

---

<!-- _class: showcase -->

# But Here's the Thing About That Commit

<div class="placeholder">
SCREENSHOT: git log showing mdd-core extraction immediately before Tauri
<br><code>git log --oneline 6904ae0..91d647b</code>
</div>

```
6904ae0  refactor: extract mdd-core shared library crate
91d647b  feat: add Tauri + Vue 3 desktop GUI (mdd-tauri crate)
```

Before I asked for the GUI, I made **one architectural decision**:

*Extract all business logic into a shared library crate first.*

The AI would have duplicated the logic or called it from the wrong layer. I made the cut. Then I asked. The one-shot landing happened **because the architecture was right** before the AI touched it.

---

<!-- _class: lesson -->

# Lesson 3

> The AI executes well when the structure is clean.
> Your architectural decisions are the prerequisite, not an afterthought.

---

<!-- _class: showcase -->

# The AI Code Review Loop

<div class="placeholder">
SCREENSHOT: git log of tagged refactor commits — design(D-1)..style(S-7)
<br><code>git log --oneline --after=2026-02-28 --before=2026-03-02</code>
</div>

**March 1st: 84 commits.** How:

1. Separate AI session → reviews codebase
2. Produces numbered findings:
   DESIGN-1..8 · BUG-1..4 · STYLE-1..7 · PERF-1..2
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

<!-- _class: danger -->

# Where It Consistently Goes Wrong

Five patterns the AI repeats across every project, every language, every session.

---

<!-- _class: failure -->

# The Five Failure Patterns

| Pattern | What the AI does | The fix |
|---------|-----------------|---------|
| String dispatch | `if title.contains("X")` | Enum + exhaustive match |
| Sentinel values | `usize::MAX`, `"-"` | `Option<T>` |
| Wildcard match | `_ => handle_other()` | Ban in instructions |
| Scope creep | Refactors on bugfix | "fix only this" |
| Dependency adds | Crate for 10 lines of stdlib | Require approval |

Knowing them in advance = catch in review, not production.

---

<!-- _class: failure -->
<!-- _class: showcase -->

# Failure: Strings Instead of Types

<div class="placeholder">
SCREENSHOT: git diff showing *ByName removal → TreeNodeByIndex
<br><code>git diff 116b339~1..116b339 -- src/</code>
</div>

The AI stored navigation targets as display names:
- `TreeNodeByName` → string-keyed lookup scanning entire tree
- `ContainerByName` → O(n) on every click · silently breaks on sort

**The fix** (`116b339`):
One typed enum: `TreeNodeByIndex { index, short_name }`
One `resolve_all_indices()` pass at build time. O(1) · compiler-enforced · sort-stable.

---

<!-- _class: failure -->

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

<!-- _class: failure -->
<!-- _class: showcase -->

# Failure: Sentinel Values

<div class="placeholder">
SCREENSHOT: code showing usize::MAX replaced with Option&lt;usize&gt;
<br><code>git show 371db5b</code>
</div>

`usize::MAX` and `u16::MAX` as stand-ins for "not set."

The AI reaches for these naturally.

A senior engineer replaces them with `Option<T>` — the compiler then **forces** you to handle the absent case everywhere.

The AI version silently mishandles it at the one callsite you forgot.

---

<!-- _class: failure -->

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

The instructions ban it: *"Never use wildcard matches. If a wildcard makes sense, ask first."*

The AI respects this perfectly once written down.

---

<!-- _class: failure -->

# Failure: Scope Creep & Dependencies

### Scope creep
Ask the AI to fix a bug → it refactors surrounding code. Diff is 3x larger than needed.

Fix: *"Fix only this, do not refactor anything else."*

### Dependency creep
AI adds a crate for what 10 lines of stdlib would solve. It won't ask.

Fix: *"Never modify Cargo.toml without approval."*

---

<!-- _class: lesson -->

# Lesson 5

> These patterns repeat.
> Write them into your instructions file as explicit prohibitions with examples.
> The AI follows written rules consistently.
> It does not self-correct without them.

---

<!-- _class: showcase -->

# What You Still Need: Domain Knowledge

<div class="placeholder">
SCREENSHOT: UDS byte grid — domain-specific encoding feature
<br><code>git checkout 3b43c35</code>
</div>

The AI did not know:
- What an MDD file is · What a DOP is
- How UDS service IDs are encoded
- What FlatBuffers schemas look like for automotive data

Every meaningful data model decision → downstream of domain expertise.

**The closer your domain is to general software** → more AI can lead.
**The further away** → more you lead, AI follows.

This is where being the domain expert — not the UI expert — was the advantage.

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

Systems engineers think about failure modes every day. This is the same skill.

---

<style scoped>
section { font-size: 22px; line-height: 1.4; }
h1 { margin-bottom: 0.15em; }
h3 { margin-top: 0.5em; margin-bottom: 0.25em; }
</style>

# The Practical Checklist

### Before you write any code:
- Write instructions file — domain, structure, style, prohibitions
- Establish lint/format gates — rustfmt, clippy, pre-commit
- Be specific. Generic instructions → generic code.

### During development:
- Maintain specific backlog items
- Review for: strings, sentinels, wildcards, deps, scope creep
- Update instructions on every new catch

### Periodically:
- AI code review → numbered findings → apply in second session

### Architecture:
- Make decisions yourself, before asking the AI
- The AI cannot see around corners. You can.

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

11 weeks. Zero lines written by hand. By someone who doesn't build UIs.

The AI is fast, capable, and follows good engineering standards — **if you write them down.**

The parts that require you:
- Domain knowledge · Architecture decisions
- The instructions file · Judgment

---

<!-- _class: title -->
<!-- _paginate: skip -->

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
<!-- _paginate: skip -->

# Thank you.

<br>

**github.com/alexmohr/mdd-ui**

<br>

*Slides, talk script, and the instructions file — all in the repo.*
