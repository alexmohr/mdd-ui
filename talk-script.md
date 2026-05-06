<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
-->

# Talk Script — "Shipping a Fully AI-Coded Project: What Actually Works"

> 20 minutes. Audience: engineers who already use AI daily.
> No pitch. No hype. Just what worked, what didn't, and how to replicate it.
> Slide cues in `[brackets]`. Pauses marked with `…`

---

## 0 · Setup (1 min)

[Slide: mdd-ui screenshot — polished desktop app]

"This is mdd-ui. Cross-platform desktop app for browsing automotive diagnostic databases.
Rust backend, Tauri shell, Vue 3 frontend. Diff engine, MCP server, embedded LLM chat.
Ships on macOS, Linux, and Windows with auto-updates.

I provided the domain knowledge and the architecture.
I wrote zero lines of code.

Today I want to walk through exactly how that happened — not the impressive parts,
the instructive parts. What setup was required, what the AI did well, where it
consistently went wrong, and what you actually need to bring yourself."

---

## 1 · The Most Important File in the Project (3 min)

[Slide: .github/copilot-instructions.md — show it in full, scroll slowly]

"Before I show you the commit history, I want to show you this.

`.github/copilot-instructions.md`. Added on day five. This is the most important
artifact in the entire project. Not the code — this.

It's a 435-line document covering:

- How the project structure should map to the domain model
- What 'jump/link' means and how it must be implemented — using the existing
  navigation system, not a popup, not a re-render
- That clippy warnings are never silenced, they are fixed
- When a file is too large and should be split into a module directory
- A complete Rust style guide with concrete do/don't examples: iterator chains
  over for-loops, `?` over unwrap, `let...else` for early returns, enums over
  string dispatch, exhaustive match arms — no wildcards, ever
- Ownership conventions: borrow, don't clone
- The newtype pattern for semantic type safety
- When to ask before proceeding versus when to just do it

…

This document is the reason the project has consistent code quality across eleven weeks
and hundreds of commits. Every session starts fresh — the AI has no memory of last time.
This file is the memory. It's the senior engineer who's always in the room.

Without it, you get a different style every session. You get the AI's defaults, which are
reasonable but inconsistent and will drift in exactly the ways you don't want.

**Lesson one: write this document first. Update it every time you catch a pattern you
don't want to see again. It compounds.**"

---

## 2 · The Backlog — Your Other Job (1.5 min)

[Slide: todo.md excerpt alongside commit `"Implement all todo.md items"`]

"The second artifact is a todo.md. Also maintained by me, not the AI.

The format matters. Each item is a specific, unambiguous requirement — not 'improve the
detail view' but 'the Static Fields overview table should have only a short-name column,
no category column. The detail view for a Static Field should show byte size and fixed
number of items.'

The more specific, the less back-and-forth, the better the output.

The AI worked through this list. At one point it cleared the entire backlog and
committed it as 'Implement all todo.md items.' One commit, all green.

That only works when the items are precise enough to execute without interpretation.

**Lesson two: your job is writing the list, not the code. Vague backlog items
produce vague code. Specific backlog items produce specific code.**"

---

## 3 · What the AI Is Actually Good At (3 min)

[Slide: timeline with callouts — first commit, Tauri drop, 114-commit day]

"Let me walk through three moments that show the ceiling of what works well.

**Cold starts.**

First commit: February 18th. Thirteen files, 4,700 lines of Rust. A complete,
keyboard-navigable, split-pane terminal application — on the first try. Not a scaffold.
Working software. The AI bootstrapped a coherent project structure, module layout, and
data model from a description of the problem domain. That's the fast part. Use it.

**Multi-layer features.**

The diff engine landed in a single session — FlatBuffers snapshot extraction,
a comparison engine, a `DiffStatus` enum threaded through every tree node,
colour-coded rendering, a new CLI subcommand structure. Multiple files, multiple
abstraction layers, all consistent with the existing architecture. When the model
is clean and the instructions are clear, the AI extends it correctly across layers.
This is the thing that would have taken a solo developer a week. It took a session.

**The Tauri drop.**

In late April: one commit, 34 files, 10,525 lines of code. Tauri v2 backend, Vue 3
frontend, TypeScript, TailwindCSS, Bun, Vite, Pinia store, five components, full
Rust-to-JS command bridge. `cargo check --workspace` clean, `bun run build` clean.

…

But here's the thing about that commit. Before I asked for it, I made one architectural
decision myself: extract all the business logic into a shared library crate first.
The AI would have built the GUI and duplicated the logic, or called it from the wrong
layer. I made the cut. Then I asked. The one-shot landing happened *because* the
architecture was right before the AI touched it.

**Lesson three: the AI executes well when the structure is clean. Your architectural
decisions are the prerequisite, not an afterthought.**"

---

## 4 · The AI Code Review Loop (2 min)

[Slide: bar chart — 114 commits on March 1st]

"March 1st: 114 commits. Here's how that happened and why it's replicable.

I asked a separate AI session to review the codebase and produce a numbered findings list.
It returned: DESIGN-1 through DESIGN-8, BUG-1 through BUG-4, STYLE-1 through STYLE-7,
PERF-1 through PERF-2. Twenty-eight specific, actionable findings.

Then I opened another session and worked through every item. Each fix is its own commit,
tagged by finding ID. The log is traceable, surgical, methodical.

The key insight: the AI reviewer was evaluating against the instructions file.
That's where it got its definition of what counted as a bug, a design smell, a style
violation. Without that standard, the review produces generic suggestions.
With it, you get findings specific to your codebase and your conventions.

**Lesson four: periodic AI-reviews-AI sessions are high-leverage. Write the findings
as a numbered document, then apply them in a second session. The instructions file
is what makes the review meaningful.**"

---

## 5 · Where It Consistently Goes Wrong (4 min)

[Slide: three columns — "Pattern", "What the AI does", "What you need"]

"This is the part that will save you the most time.

There are patterns the AI repeats across every project, every language, every session.
Knowing them in advance means you catch them in review instead of in production.

---

**Strings instead of types.**

This is the most common and the most dangerous.

The AI will use string comparisons for control flow. It will store navigation targets
as display names. It will use `"-"` as a sentinel for 'no value.' It works in the demo.
It silently breaks when you rename a label, when names aren't unique, when you sort.

In this project: navigation jump targets were `TreeNodeByName`, `ContainerByName`,
`ServiceOrJobByName` — string-keyed lookups scanning the entire tree on every click.
I had to stop and redesign: one typed enum variant, `TreeNodeByIndex`, with a single
index-resolution pass at build time. O(1), compiler-enforced, sort-stable.

The AI implemented the fix correctly and immediately.
It would never have initiated the refactor. Nothing was broken.

Add this to your instructions file: 'Do not use string comparisons for logic.
Use enums or structs instead.' Then watch for violations in review.

---

**Sentinel values instead of Option.**

`usize::MAX`, `u16::MAX` as stand-ins for 'not set'. The AI reaches for these
naturally, especially in languages where it's common pattern. A senior engineer
replaces them with `Option<T>`. The compiler then forces you to handle the absent case
everywhere. The AI version silently mishandles it at the one callsite you forgot.

This is already in the instructions file here: handle errors with `Result` and `?`,
never `unwrap()` in production code. Extend that principle to all sentinel values.

---

**Wildcard match arms.**

The AI will write `_ => handle_other()`. Every time. This is the string-dispatch
problem in a different form — you opt out of exhaustiveness checking, and the next
enum variant you add is silently unhandled.

The instructions file here bans it explicitly: 'Never use wildcard matches.
If a wildcard makes sense, ask first.' The AI respects this constraint perfectly
once it's written down.

---

**Scope creep on fixes.**

Ask the AI to fix a bug, it will also refactor the surrounding code. Sometimes that's
fine. Sometimes it changes something you didn't intend and the diff is three times
larger than it needed to be. Be explicit: 'fix only this, do not refactor anything else.'
Add it to the instructions if it keeps happening.

---

**Dependencies.**

The AI will add a dependency to solve a problem when the problem could be solved with
ten lines and the standard library. It won't ask. The instructions file here requires
approval before any new dependency. Enforce this. Dependency creep is real and
reviewing it after the fact is painful.

---

**Lesson five: these patterns repeat. Write them into your instructions file as
explicit prohibitions with examples. The AI follows written rules consistently.
It does not self-correct without them.**"

---

## 6 · What You Still Need To Bring (2.5 min)

[Slide: three items only — Domain, Architecture, Consequences]

"Three things the AI genuinely cannot supply.

**Domain knowledge.**

The AI did not know what an MDD file is. It did not know what a DOP is, what a
FlatBuffers schema looks like for automotive data, how UDS service IDs are encoded.
Every meaningful data model decision in this project was downstream of knowledge
I had from working in this domain.

This is true in every specialised field. The AI knows general software. It does not
know your specific problem. The closer your domain is to general software, the more
the AI can lead. The further it is, the more you need to lead and the AI follows.

---

**Architecture decisions.**

When to extract a shared library. When to split a module into a directory.
When to kill a subsystem and replace it. When a working implementation is the wrong
abstraction.

The shared library extraction before Tauri — that decision determined whether the
GUI was a clean extension or a mess of duplicated logic. The AI would have built
both and left you to untangle it. I made the cut first.

These are not code changes. They are decisions about the shape of the system.
The AI is an excellent implementer of a shape you've decided on. It is a poor
designer of the shape itself.

---

**Consequence thinking.**

The AI solves the problem in front of it. String-based navigation works today.
O(n) scan is fast enough today. A 1,000-line file is manageable today.

Engineering is the discipline of thinking about what 'today' becomes at scale,
under maintenance, when someone else is reading it six months from now.
The AI has no model of 'six months from now.' You do.

This is why the instructions file matters so much. It's how you encode your
consequence thinking into a form the AI can act on today."

---

## 7 · The Practical Checklist (2 min)

[Slide: checklist — copy this]

"If you want to replicate this, here's the concrete setup:

**Before you write any code:**
- Write your instructions file. Domain conventions, structural rules, style guide
  with do/don't examples, explicit prohibitions. Be specific. Generic instructions
  produce generic code.
- Establish your lint/format gates early. This project: nightly rustfmt with strict
  config, clippy with all warnings as errors, pre-commit hooks. The AI respects these
  gates and fixes violations rather than suppressing them — because the instructions
  say so.

**During development:**
- Maintain a specific backlog. Not 'improve X' — 'X should show columns A and B,
  not C, and the detail view should include fields D and E.'
- Review every session's output for the known failure patterns: strings, sentinels,
  wildcards, unnecessary dependencies, scope creep.
- Update the instructions file when you catch something new. One catch, one rule,
  never see it again.

**Periodically:**
- Run an AI code review session. Ask it to produce a numbered findings list.
  Evaluate the findings yourself — some will be wrong or not worth fixing.
  Apply the good ones in a separate session with explicit commit per finding.

**Architecture calls:**
- Make them yourself, before asking the AI to implement.
- The AI cannot see around corners. You can. Use that."

---

## 8 · Close (1 min)

[Slide: timeline — Feb 18 to May 3, eleven weeks]

"February 18th: first commit, 4,700 lines, working terminal app.
May 3rd: cross-platform desktop GUI, diff engine, MCP server, LLM chat, auto-updater.
Eleven weeks. Zero lines of code written by hand.

That is achievable. The AI is fast, capable, and will follow good engineering standards
if you write them down.

The parts that require you: domain knowledge, architecture decisions, the instructions
file, and the judgment to catch what the AI consistently gets wrong.

Those parts were always the job."

[Slide: the "it never was" meme]

"…

Thanks."

---

## Appendix — Slide Deck Structure

| # | Title | Time |
|---|-------|------|
| 0 | What we built — screenshot, zero lines of code | 1 min |
| 1 | The instructions file — show it in full | 3 min |
| 2 | The backlog | 1.5 min |
| 3 | What the AI is good at — 3 concrete examples | 3 min |
| 4 | The AI code review loop | 2 min |
| 5 | Where it consistently goes wrong — 5 patterns | 4 min |
| 6 | What you still need to bring | 2.5 min |
| 7 | The practical checklist | 2 min |
| 8 | Close | 1 min |
| **Total** | | **~20 min** |

---

## The Five Failure Patterns (summary card for slide)

| Pattern | What the AI does | The fix |
|---------|-----------------|---------|
| String dispatch | `if title.contains("X")` | Enum + exhaustive match |
| Sentinel values | `usize::MAX`, `"-"` for absent | `Option<T>` |
| Wildcard match | `_ => handle_other()` | Ban wildcards in instructions |
| Scope creep | Refactors surrounding code on a bugfix | Explicit "fix only this" instruction |
| Dependency adds | Pulls in a crate for 10 lines of stdlib | Require approval in instructions |

---

## Key Lines

> "This file is the memory. It's the senior engineer who's always in the room."

> "Your job is writing the list, not the code."

> "The one-shot Tauri landing happened because the architecture was right before the AI touched it."

> "The AI implements a shape you've decided on. It is a poor designer of the shape itself."

> "Write them into your instructions file as explicit prohibitions with examples. The AI follows written rules consistently. It does not self-correct without them."

> "The code was never the job. We just didn't have anything to prove it with — until now."
