# TUI-RUN-019 — ticks owed are counted as integers

**Classification:** `BD-COHERENT`
**Runs:** A=3, B=3

All three B-runs recovered the substance: whole pending ticks held as a
backlog, the sub-tick remainder carried forward across updates, and the
rationale that no portion of elapsed time is lost. One added the unrun-seconds
accounting, which belongs to a neighbouring spec sharing the site — richer than
the spec rather than divergent from it.

The mechanism is legible from the code because it *is* the code: an integer
count and a float remainder are visible as types. No repair.
