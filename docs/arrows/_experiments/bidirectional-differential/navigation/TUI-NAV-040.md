# TUI-NAV-040

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall bind ObserveSky to o from every screen; while no sky-event window is open the key shall be a no-op with a footer note.

## Annotated code

- `crates/clowder-tui/src/nav/update.rs`

## What the blind runs found

Readers recovered the conditional dispatch and its footer note. None recovered 'from every screen', which is the clause that makes it a global key.

## Recommendation

Split across layers: the reducer decides, the key map binds. Only the reducer is cited.
