# TUI-NAV-023

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 43 lines

## Spec as written

> The system shall display a persistent hint bar whose keys adapt to the focused context, ordered most-relevant-first and always ending with the help key, and shall surface the count key the first time a count-taking row is focused.

## Annotated code

- `crates/clowder-tui/src/render/widgets/chrome.rs`

## What the blind runs found

Readers recovered the context-matching. None recovered 'ordered most-relevant-first' or 'always ending with the help key' — though the code does both.

## Recommendation

The ordering is implicit in how the hint lists are written and the help key is appended by the overflow builder, so neither is legible as a requirement. Both are real guarantees worth their own clause and their own test.
