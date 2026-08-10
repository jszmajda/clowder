# TUI-RENDER-005

**Classification:** `INCONSISTENT-BLIND` · **Action:** `re-run` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> Between 80 and 99 columns the system shall draw a sidebar and a list, the inspector appearing only in place of the list when the player calls for it.

## Annotated code

- `crates/clowder-tui/src/render/layout.rs`

## What the blind runs found

Invalid — a harness fault, not a project finding. This spec is annotated in five places in its file, one of them a module header, and the extractor walked from that header to the end of the file: readers were handed 252 lines and reconstructed the module rather than the requirement.

## Recommendation

Re-run once the extractor caps block size. Recorded rather than dropped so the count of audited specs stays honest — this one has not really been audited.
