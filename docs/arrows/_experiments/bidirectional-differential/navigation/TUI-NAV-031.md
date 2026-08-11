# TUI-NAV-031

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Extraction:** 68 lines

## Spec as written

> The system shall tag each actionable row's command as count-taking (the count is a command argument — Pounce, Trade, DeepNap) or repeat-N (the shell calls the command N times — Build, Craft, buys); x passes N to a count-taking command and repeat-calls a repeat-N one, an empty or zero count cancels, and repeat-N calls run until one is rejected (partial success intended).

## What the blind runs found

All three readers recovered the count-semantics dispatch — that an entered count is either passed as an argument or used to repeat the call, according to how the row's command is tagged.

## Recommendation

No action. The tagging itself is the mechanism the spec describes, and it was legible.
