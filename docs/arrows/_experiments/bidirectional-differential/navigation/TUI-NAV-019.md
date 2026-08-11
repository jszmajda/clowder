# TUI-NAV-019

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Extraction:** 36 lines

## Spec as written

> The system shall keep persistent chrome on screen at every supported size: a header (clowder name/generation, date, kitten count, mood as a word, current speed, and the attention chips), the tab bar, a resource sidebar, an event ticker, and the hint bar.

## What the blind runs found

Readers recovered the header and its fields — generation, season, date, kitten count, mood word, speed — at up to 56%. None recovered the other four pieces of chrome the spec lists.

## Recommendation

The spec names five regions in one sentence and the citation sits on one of them. Five requirements, five homes; split so each region's spec can be cited and tested where it is drawn.
