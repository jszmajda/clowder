# TUI-NAV-054

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Extraction:** 6 lines

## Spec as written

> The system shall count in the idle chip only kittens the player can assign right now, excluding any kitten who is away from the meadow, so that no chip asks for an action the player cannot take.

## What the blind runs found

All three readers recovered the rule exactly — no job assigned *and* at home — and two independently reconstructed its purpose, that the chip should not count kittens the player cannot act on.

## Recommendation

No action. Notable: the reason clause was recoverable here where it usually is not, because the condition itself encodes it. A rule that reads as its own justification is the ideal case for this method.
