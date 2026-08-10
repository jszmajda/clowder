# TUI-NAV-045

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Corrected after verification** · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> When the hint bar's context keys exceed the available width, the system shall drop the least-relevant keys first and always retain the help key last, the complete key map remaining available through the help overlay.

## Annotated code

- `crates/clowder-tui/src/render/widgets/chrome.rs`

## What the blind runs found

All three readers described choosing hints appropriate to the current context. None described the overflow behaviour — dropping least-relevant keys first while retaining the help key — which is the entire subject of this spec.

## Recommendation

**This record originally said the overflow handling appeared unimplemented. That was wrong, and checking the code took under a minute.**

It is implemented: the hint builder walks candidate lengths downward, dropping
trailing entries — the list is ordered most-relevant-first, so trailing *is*
least-relevant — and appends the help key unconditionally to every candidate.
The spec is satisfied.

What the audit actually found is that readers did not recover it, which is a
statement about the code's legibility, not its correctness. The overflow loop
sits at the end of a function whose bulk is context selection, so it reads as an
afterthought rather than as the requirement. The honest finding is annotation
and emphasis, not a missing behaviour.

Recorded rather than quietly amended: inferring a gap from a reconstruction's
silence is exactly the reasoning that produced 44 false markers earlier in this
project, and it nearly produced a false one in the opposite direction here. A
reader not mentioning something is evidence about the reader.
