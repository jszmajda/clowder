# TUI-RENDER-005

**Classification:** `B-ONLY-DRIFT` *(re-measured after a citation repair — unchanged)*
**Runs:** A=3, B=3 before the repair, B=3 after

The blanket annotation on the layout enum was removed this cycle (see
TUI-RENDER-007). Re-run afterwards, readers moved from describing that enum's
three modes to describing the dead-band and the cap inside `compute` — closer to
the code that decides, and still not this spec.

The reason is instructive. `compute` holds several breakpoint rules, and the
extraction reaches the region around the annotation rather than the whole
function. This spec's rule sits elsewhere in it. Removing a misleading anchor
moved the readers to a better place; it could not move them to the right one,
because the right one is a different part of the same function.

Recorded as unchanged rather than quietly re-filed: a repair that helps one spec
and not its neighbours is the ordinary outcome, and saying so is the point of
measuring.
