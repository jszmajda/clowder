# TUI-RENDER-010

**Classification:** `UNANNOTATABLE` · **Action:** `signpost` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 29 lines

## Spec as written

> The system shall scroll group headers with the list rather than pinning them.

## Annotated code

- `crates/clowder-tui/src/render/mod.rs`

## What the blind runs found

Readers described the scroll window keeping the cursor visible — which is the sibling spec. This spec's content is that group headers are *not* pinned, and nothing in code says what was not done.

## Recommendation

A negative requirement with no sink. Pair it with a test asserting a header scrolls out of view when the cursor moves past it; the test then carries the citation.
