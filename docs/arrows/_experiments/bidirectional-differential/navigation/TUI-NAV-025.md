# TUI-NAV-025

**Classification:** `INCONSISTENT-BLIND` · **Action:** `method-limit` · **Audited:** 2026-08-10 · **Extraction:** 50 lines

## Spec as written

> When ? is pressed, the system shall show a help overlay listing the full key map and the glyph legend.

## What the blind runs found

73-82% overlap, above the leak threshold on all three runs. The reason is structural rather than a stripping miss: this spec requires a help overlay listing the key map and glyph legend, and the implementation *contains that listing as data*. The code is the requirement written out.

## Recommendation

No project action. Same class as the tmux-safe key set: where the implementation is self-documenting, the method cannot distinguish coherence from the code simply restating the spec. Worth noting that this is the second instance and both are in the shell — help text and predicate names are exactly the places where good code reads like its own requirement.
