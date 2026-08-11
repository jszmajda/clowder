# TUI-NAV-007

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Extraction:** 67 lines

## Spec as written

> Tab/BackTab, l/h, and →/← shall cycle only revealed sections; ↓/↑ and j/k shall move within the focused list; [ and ] shall jump between the list's group headers.

## What the blind runs found

The spec governs three key families — section cycling, cursor movement, and group jumps. All three readers recovered only the group jumps.

## Recommendation

Compound across three behaviours that live in three different functions. Split into three specs, each citable and testable on its own.
