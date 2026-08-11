# TUI-NAV-008

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 65 lines

## Spec as written

> The system shall remember each section's focused item by content identity (not list index), so returning to a section restores the same item even after intervening reveals inserted rows.

## What the blind runs found

Readers recovered the restore-if-still-present behaviour and the landing rule beneath it. None recovered the spec's actual claim — that focus is remembered by content identity rather than list index.

## Recommendation

The distinguishing clause is a statement about what the code does *not* do. A reader cannot see the index-based implementation that was rejected. This wants a test that inserts a row above the cursor and asserts focus did not move — the behaviour the choice exists to protect.
