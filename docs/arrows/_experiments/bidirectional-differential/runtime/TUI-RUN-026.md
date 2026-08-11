# TUI-RUN-026 — autosave on a cadence and on every clean shutdown

**Classification:** `B-ONLY-DRIFT`
**Runs:** A=3, B=3

## What the blind readers saw

Readers recovered the cadence half in full and neither of the others. The shutdown save and the core's atomic write live elsewhere than the cited region — and the shutdown site marked itself with a plain comment naming the spec rather than an annotation, so the extractor never offered it.

## Repair

Citation added at the shutdown save, which now carries both this spec and TUI-RUN-031. The prose marker that stood in for it is gone.
