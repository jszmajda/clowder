# TUI-RUN-017 — render-independence

**Classification:** `B-ONLY-DRIFT`
**Runs:** A=3, B=3

## Spec

The system advances the simulation regardless of render state.

## What the blind readers saw

All three B-runs reconstructed the tick arithmetic and the per-frame cap — TUI-RUN-019 and TUI-RUN-025 — and none recovered that the advance is ungated. The requirement is an absence

## Repair

 that the tick call sits under no render conditional. An extracted region shows what code does, never what encloses it.:Covered by two tests in loop_body.rs. No repair needed; recorded as a blind-class instance.
