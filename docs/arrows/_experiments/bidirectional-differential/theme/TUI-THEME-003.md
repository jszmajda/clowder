# TUI-THEME-003

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall use colour only to convey state, never identity — a resource, structure, or idea shall take its colour from its current state and never from which item it is.

## Annotated code

- `crates/clowder-tui/src/theme/mod.rs`

## What the blind runs found

Readers recovered the closed set of six semantic roles. None recovered the rule the spec is actually about — that colour conveys state and never identity.

## Recommendation

The prohibition is the requirement; the role set is how it is kept. Code that maps roles to colours cannot testify that no colour was ever assigned per-item, so this needs a test asserting the negation — render two different items in the same state and require the same colour.
