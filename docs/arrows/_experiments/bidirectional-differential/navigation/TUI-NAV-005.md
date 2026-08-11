# TUI-NAV-005

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Extraction:** 67 lines

## Spec as written

> The system shall assign sections to the digits 1–0 by a fixed design-time map that never renumbers; pressing a section's digit jumps to it, and pressing the digit of an unrevealed section is a no-op with a footer note.

## What the blind runs found

All three readers recovered the digit map including the tenth section on 0, and the revealed check that makes an empty section's digit a no-op.

## Recommendation

No action. The 'never renumbers' clause is a property of the map's authorship, not of runtime behaviour, and is correctly unrecovered.
