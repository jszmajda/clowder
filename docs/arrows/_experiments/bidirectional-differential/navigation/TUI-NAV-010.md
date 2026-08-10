# TUI-NAV-010

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> While Enter is held on a row that is not a heavy action, the system shall repeat the focused row's action at no more than ten times per second regardless of the terminal's key-repeat rate, and the cursor shall not move on action.

## Annotated code

- `crates/clowder-tui/src/nav/update.rs`
- `crates/clowder-tui/src/nav/view.rs`

## What the blind runs found

The rate limit was recovered exactly, to the millisecond. 'The cursor shall not move on action' was recovered by nobody.

## Recommendation

Compound again, and the missing half is an absence — no code exists for a cursor that does not move. Split it, and let the second clause be carried by a test that asserts the cursor is unchanged after firing.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
