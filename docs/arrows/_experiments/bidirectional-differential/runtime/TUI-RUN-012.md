# TUI-RUN-012

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> When the save file is present but unparseable, the system shall move it aside under a suffixed name, tell the player where it went, and then start a new game; it shall never overwrite the unparseable file in place.

## Annotated code

- `crates/clowder-tui/src/runtime/persist.rs`

## What the blind runs found

Readers recovered the renaming mechanism exactly, including the numbered suffix that avoids destroying an earlier rescue. None recovered 'tell the player where it went' or 'start a new game'.

## Recommendation

The annotation is on the rename helper, which is only the first third of the spec. The telling and the fresh start happen in the caller. Cite both, or split the spec so each clause has a home.
