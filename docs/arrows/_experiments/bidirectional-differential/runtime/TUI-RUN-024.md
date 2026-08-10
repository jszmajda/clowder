# TUI-RUN-024

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-code` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall cap an in-process time gap at the same limit the load path applies and flag it the same way, so an absence spanning a suspend and an absence spanning a restart are indistinguishable to the player.

## Annotated code

- `crates/clowder-tui/src/runtime/clock.rs`

## What the blind runs found

Readers again described the clock choice, not the cap. Notably none mentioned the flag — 'flag it the same way' — which an earlier code review independently reported as unimplemented: there is no truncation signal on the in-process path.

## Recommendation

Two methods reaching the same absence is corroboration. The cap exists; the flag does not, so an in-process gap cannot be reported to the player the way a between-process one is. Add the signal, then a test.
