# TUI-NAV-002

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> While a screen is focused with no prompt or overlay open, the system shall move the list cursor on ↑/↓ (and j/k), and perform the focused row's primary action once on Enter.

## Annotated code

- `crates/clowder-tui/src/nav/action.rs`
- `crates/clowder-tui/src/nav/keys.rs`
- `crates/clowder-tui/src/nav/rows.rs`
- `crates/clowder-tui/src/nav/update.rs`

## What the blind runs found

All three blind readers reconstructed *the key map* — "resolve a key event to exactly one action according to the current view context" — not cursor movement and Enter. They agree with each other and disagree with the spec.

## Recommendation

The annotation sits on the key-resolution function, whose behaviour is a different requirement (the tmux-safe key set and the prompt/overlay suspension rules). A reader of the annotated code cannot recover this spec because this code is not where the spec lives. Move the citation to the reducer arms that actually move the cursor and fire the row's action.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
Agreement *within* a direction and disagreement *between* them is the signal.
