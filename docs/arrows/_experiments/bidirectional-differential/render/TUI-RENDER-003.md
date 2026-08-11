# TUI-RENDER-003

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Extraction:** 24 lines

## Spec as written

> When an offline banner is present, the system shall take its row from the body rather than adding a row to the frame.

## What the blind runs found

Readers described breakpoints and the dead-band, not the banner taking its row from the body. The citation now resolves to the layout computation, which serves four specs at once.

## Recommendation

Previously recorded as one of two behaviour specs cited only on inert state. It is cited in both places, and neither isolates it. Cite the line that subtracts the banner's row from the body height — that single subtraction *is* the requirement.
