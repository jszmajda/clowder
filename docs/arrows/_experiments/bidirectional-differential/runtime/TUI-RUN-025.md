# TUI-RUN-025 — a catch-up is paced, never dropped

**Classification:** `A-ONLY-DRIFT`
**Runs:** A=3, B=3

## The divergence

The spec said "a bounded slice of wall time per frame". All three A-runs built
exactly that: `SLICE: Duration = 5ms`, `TICK_BUDGET: 8ms`,
`TICK_SLICE_BUDGET: 4ms`, each with an `Instant::now()` loop checking elapsed
time between ticks.

The code bounds a **count** — `MAX_TICKS_PER_FRAME` — and its comments have
always said so, and why: the accumulator cannot know what a tick costs, so the
caller chooses the number. The B-runs read the count back accurately.

Neither direction was confused. The spec simply described a different mechanism
from the one that was built.

## Which is right

The code, for a reason worth keeping in the spec rather than only in a comment.
A wall-time budget has to read a clock inside the drain loop, and it buys a
different number of ticks on a fast machine than on a slow one — which makes
how far the meadow advances during a catch-up a property of the hardware. The
purpose clause the spec already carries, that a catch-up never blocks the loop
and never discards owed ticks, is satisfied by a count.

## Repair

EARS reconciled to state the bound as a count and to give the reason, so the
next reader does not restore the time budget from the words. Code unchanged;
tests already assert the count behaviour.
