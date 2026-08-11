# TUI-RUN-031 — shutdown does not drain the backlog first

**Classification:** `B-ONLY-DRIFT`
**Runs:** A=3, B=3

## What the blind readers saw

Readers recovered the save format — timestamp plus the still-owed count, so quitting mid-catch-up carries forward — and not the clause that gives the spec its name. That a shutdown does not drain first is satisfied by the absence of draining code in the shutdown path, which is nothing a reader can see.

## Repair

Citation added at the shutdown save. The behaviour was already right; nothing pointed at it.
