# TUI-RUN-028 — a failed save ends nothing

**Classification:** `BD-COHERENT`
**Runs:** A=3, B=3

All three B-runs recovered every clause: reported once on the transition into
failure and suppressed until a save succeeds again, the session continuing, and
the retry waiting for the next cadence rather than spinning. One recovered the
timer resetting even on failure, which is what makes "next cadence" true.

Richer than the spec rather than divergent from it. No repair.
