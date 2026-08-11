# TUI-RENDER-007 — the list stops growing

**Classification:** `BD-COHERENT` *(re-measured after a citation repair — was `B-ONLY-DRIFT`)*
**Runs:** A=3, B=3 before the repair, B=3 after

## Before

All three readers described a type listing three layout modes. Nothing about a
cap, a breakpoint, or where surplus width goes. Filed as drift.

## The repair

Four breakpoint specs were annotated as a block on the enum those breakpoints
choose between — a clean, self-contained type, which is what the extractor takes
whenever one is offered. Each variant already carried the spec it serves, and
the widths that pick between them are annotated inside `compute`, where the rule
actually lives. The blanket annotation named the shape the rule produces as
though it were the rule.

Removed. Nothing else changed.

## After

Readers now describe the cap breakpoint, the capped width, and the hysteresis
around it. The spec's content is recovered.

This is the second repair in this audit measured rather than assumed, and the
second time moving an annotation off a declaration and onto the behaviour turned
a miss into a recovery.
