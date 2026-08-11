# TUI-NAV-021

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 40 lines

## Spec as written

> The system shall render every UI symbol as a single-width character and shall use no emoji or ambiguous-width glyphs.

## What the blind runs found

Readers described width-fitting and truncation, which belongs to a render spec. None described the glyph constraint this spec is about.

## Recommendation

The citation sits on the fitting helper because that is where widths are handled, but the requirement is a constraint on *what characters the source contains*, not on any function's behaviour. No runtime code can testify to it. The existing cell-level test that asserts every drawn cell is single-width is the right home for the citation.
