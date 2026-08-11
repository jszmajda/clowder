# TUI-RENDER-002

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Extraction:** 84 lines

## Spec as written

> The system shall lay the frame out as a header row, a tab-bar row, a body, a ticker of one or more rows, and a hint-bar row, with the body taking whatever height the chrome leaves so the regions always sum to the terminal's height.

## What the blind runs found

All three readers recovered every region of the chrome in order, including that the banner appears only while a report is undismissed — a conditional the spec states elsewhere.

## Recommendation

No action. The cleanest render result of the audit.
