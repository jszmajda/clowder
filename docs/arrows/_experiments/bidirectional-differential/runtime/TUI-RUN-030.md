# TUI-RUN-030 — the fractional remainder and the whole-tick backlog

**Classification:** `B-ONLY-DRIFT`
**Runs:** A=3, B=3

## What the blind readers saw

B recovered the fractional carry and the catch-up cap, and not the clause the spec turns on — that every interval is converted at the speed in force when it elapsed, so a later speed change re-rates neither quantity. What is visible in code is that two numbers are kept; why they are kept already-converted is a claim about a change that has not happened yet.

## Repair

No repair. The behaviour is asserted; the rationale is not recoverable from code by construction.
