# AGENTS.md

Project instructions for coding agents working in this repository. The file is named `AGENTS.md` per the cross-tool convention; Claude Code finds the same content via the `CLAUDE.md` symlink.

## Repository Purpose

**Clowder** is a cozy terminal incremental game about raising a thriving clowder of kittens — Rust + ratatui, played entirely from the keyboard, at home inside a tmux pane. It is loosely inspired by Kittens Game, with every grim mechanic (starvation, sacrifice) redesigned as something playful. Nothing in Clowder ever dies and nothing is ever sacrificed.

## Structure

- `docs/high-level-design.md` — the HLD: game vision, architecture, tenets
- `docs/intent/` — design tree: one folder per component, each holding its `{node}-design.md` and `{node}-specs.md`
- `crates/clowder-core/` — deterministic, headless simulation library (game rules, tick, content tables, save/load)
- `crates/clowder-tui/` — ratatui binary: screens, input mapping, truecolor theme (next segment; not yet built)

The two-crate split is load-bearing: `clowder-core` never touches the terminal, and all game-balance tests run headless via `cargo test -p clowder-core`.

## Spoiler Policy

Jess (the player-owner) wants to **discover the game's content by playing it**. Agents build this project autonomously through the LID workflow and must not reveal game content in conversation: no resource, structure, job, research, or event names; no balance numbers; no flavor text. Status updates stay structural — system counts, test results, file paths. Check in with Jess only on preference-affecting, non-spoiler aspects (pacing, scope, risk). Phase-boundary reviews resolve against the HLD's ratified tenets (cozy > idle-respect > whimsy-with-clarity) rather than user review; this standing override was granted explicitly, drift risk acknowledged. Everything under `docs/intent/` and `crates/` is spoiler territory for Jess until she has played past it.

## Conventions

- Version control is **jj** (Jujutsu), colocated with git. Prefer `jj` commands over bare `git`.
- Content (buildings, jobs, upgrades, resources) lives in typed Rust tables in `clowder-core`, not external data files.
- The theme assumes 24-bit truecolor; no low-color fallback palettes.
- Keybindings must coexist with default tmux bindings: plain printable keys, Tab, and arrows only — no Ctrl chords (except Ctrl-C), no Alt/Meta, no function keys.

## LID
- Mode: Full
- Version: 1.3.0

## Linked-Intent Development (MANDATORY)

**Consult the `linked-intent-dev` skill for ALL code changes.** All changes flow through the arrow of intent in one direction:

```
HLD → LLDs → EARS → Tests → Code
```

- **New features and refactors**: full six-phase workflow (HLD check → LLD check/draft → EARS → intent-narrowing edge audit → tests-first → code).
- **Bug fixes**: walk the arrow like any other change — find where behavior diverged from intent and cascade from there. No short-circuit.
- **If unsure**: use the full workflow.

Stop after each phase for user review. **Docs carry current intent, written to be read cold** — write each doc as if authored fresh today, from current intent alone: no narration of how it changed, no meaning that needs the conversation that produced it, no rebuttals to questions only a past discussion raised. Rationale, considered alternatives, and constraints a fresh author would independently write stay; record rejected alternatives and why in the LLD's Decisions & Alternatives table, not as asides in body prose.

**Memory vs. intent.** Before saving durable project knowledge to agent or tool memory, test whether it is project *intent* — would a fresh agent, in any tool, next session, need it to build this system correctly? If yes, record it in the arrow (HLD / LLD / EARS / decision doc), which travels and cascades — not in private, per-tool memory, where intent escapes the arrow. Knowledge about the user or how they like to work stays in memory.

### Navigation

| What you need | Where to look |
|---|---|
| High-level design | `docs/high-level-design.md` |
| Design tree (sub-HLDs, LLDs, their specs) | `docs/intent/` — one folder per node |
| EARS specs | beside each design doc as `{node}-specs.md` in the node's folder under `docs/intent/` |
| Decision docs | `docs/decisions/` (project-level) and `docs/intent/<segment>/decisions/` |

### Terminology

- **HLD**: High-Level Design — single project-level doc at `docs/high-level-design.md`.
- **LLD**: Low-Level Design — detailed component design doc in `docs/intent/`. The design layer is a recursive tree: the root is the HLD, leaf LLDs own EARS, and a component deep enough to outgrow one doc becomes a sub-HLD (HLD-shaped, owns no EARS) with children beneath it. "HLD" and "LLD" are roles by position; depth-2 (one HLD over flat leaf LLDs) is the default.
- **EARS**: Easy Approach to Requirements Syntax — structured one-line requirements beside each design doc as `{node}-specs.md` in the node's folder under `docs/intent/`. IDs are path-concatenated — the root-to-leaf path of the owning segment plus a number — so a prefix grep gathers a subtree. Markers: `[x]` implemented, `[ ]` active gap, `[D]` deferred.
- **Arrow**: the unidirectional chain from vision to code (HLD → LLDs → EARS → Tests → Code). Strictly a DAG of intent.
- **Arrow segment**: the territory owned by one leaf LLD — the LLD itself plus the specs, tests, and code that cite its EARS IDs. The boundary is the leaf prefix. Within-segment cascade is free; across-segment cascade pauses.
- **Cascade**: propagating a change downstream through the arrow so adjacent levels stay coherent.

### Code annotations

Annotate code and tests with `@spec` comments citing EARS IDs:

```rust
// @spec SIM-RES-001, SIM-RES-002
```

Place the annotation at the *entry point of the behavior's implementation graph* — the topmost function or module owning the specified behavior, not every helper. When a behavior spans multiple subsystems (sim core + TUI, for example), annotate at the entry point in each subsystem. Tests follow the same rule: annotate the test that directly exercises the spec, not every inner assertion.
