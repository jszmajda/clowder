# TUI-NAV-052

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Extraction:** 28 lines

## Spec as written

> While a full-screen overlay (Journal or help) is open, the system shall give the overlay the cursor, section, and action keys for its own navigation and suspend their global meanings, while keeping the speed keys, q, Ctrl-C, and Esc live — so the player can always retime, leave, or close from anywhere.

## What the blind runs found

All three readers recovered which keys stay live while an overlay is open — quit, the speed pair, cursor movement, group jumps — and that the rest are suspended.

## Recommendation

No action.
