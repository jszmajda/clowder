# TUI-THEME-009 — the seasonal accent never carries state

**Classification:** `B-ONLY-DRIFT`
**Runs:** A=3, B=3

## What the blind readers saw

B recovered the season-to-colour mapping exhaustively and never the requirement, which is about where the accent may be *used* — no element that carries state, so no colour is ambiguous between a state and a season. The citation sits on the mapping; the requirement constrains its call sites, which the extraction cannot see.

## Repair

No repair. Absence-at-the-call-site, the sharpest recurring blind class.
