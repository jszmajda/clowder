# TUI-NAV-043

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> Arming a heavy row shall reserve no resources and shall disarm any previously armed row; the committing Enter shall re-run the real command so the core re-validates it against current state, and a commit that is no longer valid shall fail to the footer like any other rejection.

## Annotated code

- `crates/clowder-tui/src/nav/update.rs`
- `crates/clowder-tui/src/nav/view.rs`

## What the blind runs found

All three readers reconstructed a *timing* requirement — 'apply the armed command only if the press came at least the repeat-gate interval after the previous one'. The spec is about *state*: arming reserves nothing, disarms its predecessor, and the commit re-runs the real command so the core re-validates.

## Recommendation

Not one of the spec's four clauses was recovered. The annotation sits on the commit gate, whose observable behaviour is timing. Same shape as TUI-NAV-002: the citation points at code that testifies to a different requirement.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
