# TUI-NAV-016

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> When q is pressed, the system shall save and quit immediately with no confirmation; Ctrl-C shall also save and quit.

## Annotated code

- `crates/clowder-tui/src/nav/update.rs`

## What the blind runs found

Readers recovered that quitting is answered immediately from any context, before other handling, without mutating view state. None recovered that it SAVES, and none recovered the Ctrl-C half.

## Recommendation

The requirement is spread across three layers — the reducer answers, the loop saves, the key map binds Ctrl-C — and only the reducer carries a citation. A reader sees a third of the guarantee. Cite all three.
