<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
-->

# Systems Engineers Can Have Nice Things
## Shipping an AI-Coded Desktop App Without Writing Code

> 30 minutes of content for a 20-minute slot (quick talker buffer).
> Slide cues in `[SLIDE]`. Screenshot placeholders in `[SCREENSHOT: description | sha]`.

---

## Slide 1 — Title

**Systems Engineers Can Have Nice Things**
*Shipping an AI-Coded Desktop App Without Writing Code*

Alexander Mohr

*An experience report. No pitch, no hype.*

> Set the tone: peer-to-peer. "I'm going to tell you what worked and what didn't."

---

## Slide 2 — A Quick Confession (0.5 min)

I'm an onboard diagnostics engineer. ECUs, UDS protocols, CAN buses.

I don't build UIs. I build the thing the UI talks to.

I had a tooling problem: automotive diagnostic databases are complex, deeply nested, and the existing tools weren't great. I needed a visual solution. So I let AI build one.

> Keep it brief. 30 seconds max. The audience should think: "OK this person isn't a frontend dev — interesting."

---

## Slide 3 — What We Built (1.5 min)

[SCREENSHOT: mdd-ui desktop app — full GUI with tree pane, detail pane, search visible | 3b43c35]

- Cross-platform desktop app for automotive diagnostic databases
- Rust backend · Tauri v2 · Vue 3 + TypeScript frontend
- Diff engine · MCP server · LLM chat · Auto-updater

> Let the screenshot speak. Point at tree, detail pane, search. "This looks like it was built by a frontend team. It wasn't."

---

## Slide 4 — What We Built (cont.)

**I provided the domain knowledge and the architecture.**

**I wrote zero lines of code.**

Every line of implementation — AI-generated. Every architectural decision — mine.

Today: not the impressive parts, the *instructive* parts.

> Don't linger. Set up the structure of the talk: setup, strengths, failures, what you still need.

---

## Slide 5 — Timeline: 11 Weeks, Zero to Shipped (1.5 min)

[SLIDE: visual timeline]

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

> Point out the ⚠ — that's where engineering mattered most. "The AI did the volume. I did the decisions."

---

## Slide 6 — The Most Important File in the Project (2.5 min)

[SCREENSHOT: .github/copilot-instructions.md — full file in editor, scroll visible | 75707ff]

`.github/copilot-instructions.md` — added day 5.

This is the most important artifact in the entire project. Not the code — this.

> Scroll through it slowly on screen if presenting live. Let people absorb the length and specificity.

---

## Slide 7 — What's In the Instructions File

A 435-line document covering:

- Project structure ↔ domain model mapping
- What "jump/link" means — use existing nav, no popups
- Clippy warnings are never silenced — they are fixed
- When files should be split into module directories
- Complete Rust style guide with do/don't examples
- Ownership conventions, newtype pattern
- When to ask vs. when to just do it

> Read selected items. Don't rush through the whole list. Pick the 3 that resonate.

---

## Slide 8 — Why It Matters

Without it: a different style every session. AI defaults drift.

With it: consistent code quality across 11 weeks, hundreds of commits, zero memory between sessions.

"This file is the memory. It's the senior engineer who's always in the room."

Every session starts fresh — the AI has no memory of last time. This file IS the memory.

> Hit the quote hard. This is the anchor of the whole talk.

---

## Slide 9 — Lesson 1

> Write the instructions file first. Update it every time you catch a pattern you don't want to see again. It compounds.

> Pause. This is the single most important takeaway. Let it breathe.

---

## Slide 10 — The Backlog: Your Other Job (1.5 min)

[SCREENSHOT: todo.md with specific requirements | aabee68]

The format matters. Each item: specific, unambiguous.

❌ "improve the detail view"

✅ "Static Fields overview table: only short-name column, no category. Detail view shows byte size and fixed number of items."

> Show the screenshot, then the contrast. Let them feel the difference.

---

## Slide 11 — The Backlog: Results

[SCREENSHOT: git log showing commit c0da734 "Implement all todo.md items" | c0da734]

The AI cleared the entire backlog in one commit: all green.

`c0da734  Implement all todo.md items`

That only works when items are precise enough to execute without interpretation.

Lesson 2: Your job is writing the list, not the code. Vague backlog → vague code.

---

## Slide 12 — Section Break: What the AI Is Good At (0.5 min)

Three patterns where AI exceeded expectations.

> Quick transition. Don't elaborate — the slides do the work.

---

## Slide 13 — Cold Starts (1.5 min)

[SCREENSHOT: initial commit file tree — src/app/, src/tree/ visible with line counts | c4d2a96]

Feb 18 — First commit. 13 files, 4,700 lines of Rust.

Complete keyboard-navigable split-pane TUI. Not a scaffold — working software. On the first try.

> "The AI bootstrapped what would take me a week of boilerplate in an afternoon." Emphasize: as an ECU engineer, I didn't know how TUI frameworks work. The AI did.

---

## Slide 14 — Multi-Layer Features (1.5 min)

[SCREENSHOT: diff mode with color-coded tree nodes | 3d357da]

The diff engine — landed in a single session (Mar 03). FlatBuffers extraction, comparison engine, DiffStatus enum threaded through every tree node, colour-coded rendering.

Multiple files, multiple layers, consistent architecture.

> "When the model is clean and instructions are clear, the AI extends correctly across layers."

---

## Slide 15 — The Tauri Drop: TUI → Desktop GUI (2 min)

[SCREENSHOT: TUI on left, Tauri GUI on right — before/after | c4d2a96 → 91d647b]

Apr 25 — one commit: 34 new files, 10,525 lines of code. Tauri v2, Vue 3 + TypeScript, TailwindCSS, Pinia, Bun + Vite.

cargo check clean. bun run build clean.

> Let the before/after speak. "A systems engineer's terminal app became a desktop GUI in one commit."

---

## Slide 16 — But Here's the Thing About That Commit (1.5 min)

[SCREENSHOT: git log showing mdd-core extraction immediately before Tauri | 6904ae0..91d647b]

Before I asked for the GUI, I made one architectural decision: extract all business logic into a shared library crate first.

The AI would have duplicated the logic or called it from the wrong layer. I made the cut. Then I asked.

The one-shot landing happened because the architecture was right before the AI touched it.

> This is the punchline of the "good at" section. Architecture-first, AI-second.

---

## Slide 17 — Lesson 3

> The AI executes well when the structure is clean. Your architectural decisions are the prerequisite, not an afterthought.

---

## Slide 18 — The AI Code Review Loop (2 min)

[SCREENSHOT: git log of tagged refactor commits — design(D-1)..style(S-7) | 0442fd0]

March 1st: 84 commits. How:

1. Separate AI session → reviews codebase
2. Produces numbered findings: DESIGN-1..8, BUG-1..4, STYLE-1..7, PERF-1..2
3. New session → applies every item
4. Each fix = own commit, tagged by finding ID

> Explain the workflow. The key insight: two sessions, not one. Reviewer doesn't fix. Fixer doesn't review.

---

## Slide 19 — Why the Review Loop Works

The AI reviewer evaluated against the instructions file. That's where it got its definition of what counts as a bug, a design smell, a style violation.

Without that standard → generic suggestions. With it → findings specific to your codebase.

Lesson 4: Periodic AI-reviews-AI sessions are high-leverage.

> Connect back to Lesson 1. The instructions file enables this.

---

## Slide 20 — Section Break: Where It Goes Wrong (0.5 min)

Five patterns the AI repeats across every project, every language, every session.

> Shift tone. This is the "honest" section. Lean in.

---

## Slide 21 — The Five Failure Patterns (1.5 min)

| Pattern | What the AI does | The fix |
|---------|-----------------|---------|
| String dispatch | `if title.contains("X")` | Enum + exhaustive match |
| Sentinel values | `usize::MAX`, `"-"` | `Option<T>` |
| Wildcard match | `_ => handle_other()` | Ban in instructions |
| Scope creep | Refactors on bugfix | "fix only this" |
| Dependency adds | Crate for 10 lines of stdlib | Require approval |

> Don't read every row. Highlight strings and wildcards, skip the rest — audience can read.

---

## Slides 22–26 — Individual Failure Deep-Dives (4 min total)

**Strings Instead of Types** (116b339): TreeNodeByName → TreeNodeByIndex. Works in demo, breaks on sort. The AI implemented the fix immediately but would never have initiated it.

**Sentinel Values** (371db5b): usize::MAX → Option<T>. The compiler forces you to handle the absent case. The AI doesn't think about the callsite you forgot.

**Wildcard Match Arms**: The AI writes `_ => handle_other()` every time. Instructions ban it. The AI respects the ban perfectly.

**Scope Creep & Dependencies**: Bug fix becomes refactor. 10 lines of stdlib become a new crate. Both solved by explicit instructions.

> Pick 1-2 to go deep on. Strings is the best story. Skip sentinel/wildcard if running long.

---

## Slide 27 — Lesson 5

> These patterns repeat. Write them into your instructions file as explicit prohibitions with examples. The AI follows written rules consistently. It does not self-correct without them.

---

## Slide 28 — What You Still Need: Domain Knowledge (1.5 min)

[SCREENSHOT: UDS byte grid — domain-specific encoding feature | 3b43c35]

The AI did not know: what an MDD file is, what a DOP is, how UDS service IDs are encoded.

Every meaningful data model decision → downstream of domain expertise.

The closer your domain is to general software → more AI can lead. The further away → more you lead.

This is where being the domain expert — not the UI expert — was the advantage.

> "As a systems engineer, my domain knowledge was the differentiator. Not my inability to write CSS."

---

## Slide 29 — What You Still Need: Architecture + Consequence Thinking (1.5 min)

When to extract a shared library. When to split a module. When a working implementation is the wrong abstraction.

The AI solves the problem in front of it. String navigation works today — breaks when you sort. O(n) scan is fast enough today — breaks at scale.

The AI has no model of "six months from now." You do.

Systems engineers think about failure modes every day. This is the same skill.

> "Consequence thinking isn't a software-specific skill. It's what we do with ECU failure modes, with protocol edge cases. Same muscle, different domain."

---

## Slide 30 — The Practical Checklist (1 min)

[SLIDE: checklist — skim, don't read]

Before: instructions file, lint gates, be specific.
During: specific backlog, review for known patterns, update instructions.
Periodically: AI code review loop.
Architecture: make decisions yourself.

> "This is in the slides. Copy it later. Don't try to write it down now."

---

## Slide 31 — The Numbers (0.5 min)

| Metric | Value |
|--------|-------|
| First working prototype | Day 1 |
| Lines at GUI launch | ~15,000+ |
| Time to shippable app | ~11 weeks |
| AI-generated features | All of them |
| Architectural rewrites | 2–3 targeted |
| Instructions file updates | Continuous |
| Lines of code written by hand | Zero |

> Let the table speak. Don't read every row.

---

## Slide 32 — Close (1 min)

February 18th: first commit, 4,700 lines, working terminal app.

May 3rd: cross-platform desktop GUI, diff engine, MCP server, LLM chat, auto-updater.

11 weeks. Zero lines written by hand. By someone who doesn't build UIs.

The AI is fast, capable, and follows good engineering standards — if you write them down.

The parts that require you: domain knowledge, architecture decisions, the instructions file, judgment.

> Slow down here. This is the summary. Let it land.

---

## Slide 33 — The Punchline

> "Those parts were always the job."

[SLIDE: "it never was" meme]

> 2-second pause. Let it land. Then: "Thanks."

---

## Slide 34 — Thank You

**github.com/alexmohr/mdd-ui**

*Slides, talk script, and the instructions file — all in the repo.*

---

## Appendix — Slide Reference with SHAs

| Slide | Screenshot Needed | Commit SHA |
|-------|-------------------|------------|
| 3 | mdd-ui desktop app — polished GUI | `3b43c35` (latest feature state) |
| 6 | .github/copilot-instructions.md in editor | `75707ff` (instructions added) |
| 10 | todo.md with specific requirements | `aabee68` (todo + instructions added) |
| 11 | "Implement all todo.md items" in git log | `c0da734` |
| 13 | Initial commit file structure | `c4d2a96` |
| 14 | Diff mode with color-coded tree | `f29a126` or `3d357da` |
| 15 | TUI (before) and Tauri GUI (after) | `c4d2a96` → `91d647b` |
| 16 | git log showing mdd-core → Tauri sequence | `6904ae0` + `91d647b` |
| 18 | git log of tagged refactor commits | `0442fd0` (findings doc) |
| 22 | Diff: TreeNodeByName → TreeNodeByIndex | `116b339` |
| 23 | Code: usize::MAX → Option | `371db5b` |
| 28 | UDS byte grid (domain-specific) | `3b43c35` |

---

## Timing Budget (30 min content)

| Slide(s) | Section | Time |
|-----------|---------|------|
| 1 | Title | 0.5 min |
| 2 | Quick confession | 0.5 min |
| 3–5 | What we built + timeline | 3 min |
| 6–9 | Instructions file + Lesson 1 | 4 min |
| 10–11 | Backlog + Lesson 2 | 2 min |
| 12–17 | What AI is good at (3 examples + Lesson 3) | 7 min |
| 18–19 | Code review loop + Lesson 4 | 2.5 min |
| 20–27 | Where it goes wrong (5 patterns + Lesson 5) | 6 min |
| 28–29 | What you still need | 3 min |
| 30–31 | Checklist + numbers | 1.5 min |
| 32–34 | Close + punchline | 1.5 min |
| **Total** | | **~32 min raw / ~22 min delivered** |

---

## Speaker Notes — Pacing

- Slides 6–8 (instructions file) are the anchor — take your time here, this is the key takeaway
- Slides 22–26 (failure deep-dives) can be compressed by skipping sentinel/wildcard if running long
- Slide 30 (checklist) can be shown without reading aloud — "this is in the slides, copy it later"
- The meme at the end needs a 2-second pause before "Thanks" — let it land
- Thread the systems engineer identity throughout: "as someone who works with ECUs, not UIs..."
- The narrative arc: confession → proof it works → how → where it fails → what's still yours → punchline
