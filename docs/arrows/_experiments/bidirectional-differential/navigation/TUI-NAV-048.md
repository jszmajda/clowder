# TUI-NAV-048

**Classification:** `INCONSISTENT-BLIND` · **Action:** `re-run` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 3 lines

## Spec as written

> Where the terminal is about 120 columns or wider, the system shall lay out three columns — sidebar, list capped for scannability, and inspector taking the remainder — giving extra width to the inspector and the ticker.

## What the blind runs found

Invalid — harness fault, the mirror of batch six's. The extractor landed on a module's import block and handed the readers three lines of `use` declarations. One reader said so outright rather than confabulating: 'This snippet can't support a reconstruction.'

## Recommendation

That refusal is worth recording as a property of the method: a blind reader given nothing will sometimes say so. The extractor now skips blocks with fewer than four lines of actual logic. Re-run. Left out of the audited count.
