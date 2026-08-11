# TUI-NAV-012

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Extraction:** 76 lines

## Spec as written

> When X is pressed on a focused row that is not a heavy action, the system shall apply the row's command repeatedly until the core rejects it or a shell-side iteration cap is reached.

## What the blind runs found

Second run, same result: readers described the reducer's quit-first dispatch rather than the repeat-until-rejected behaviour. The extraction is still 76 lines because the citation sits on the reducer's entry point, not on the arm that implements this spec.

## Recommendation

Not a harness fault after all — the citation is genuinely too high in the call graph. Move it to the max-affordable handler. Recorded as a finding rather than re-run again.
