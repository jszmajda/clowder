# TUI-NAV-014

**Classification:** `UNANNOTATABLE` · **Action:** `signpost` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> When a command is rejected by the core, the system shall surface the CommandError as a plain-language footer message for a few seconds and shall not open any blocking dialog.

## Annotated code

- `crates/clowder-tui/src/nav/action.rs`
- `crates/clowder-tui/src/nav/mod.rs`
- `crates/clowder-tui/src/nav/update.rs`
- `crates/clowder-tui/src/nav/view.rs`
- `crates/clowder-tui/src/render/mod.rs`

## What the blind runs found

Readers recovered the footer rendering precisely, including its four-second life. None recovered "shall not open any blocking dialog" — a negative requirement with no production sink.

## Recommendation

There is no code that can carry a citation for a dialog never opened. Either pair it with an unwanted-behaviour spec and a test asserting the negation (the test then carries the annotation), or drop the clause and let the absence of any dialog code speak. Also worth noting: the spec says "a few seconds" and the code says four; readers pinned four, so the vagueness is doing no work.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
Agreement *within* a direction and disagreement *between* them is the signal.
