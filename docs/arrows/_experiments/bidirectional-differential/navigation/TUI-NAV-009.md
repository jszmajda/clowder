# TUI-NAV-009

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall render each actionable item as a row carrying a glyph-first availability marker (● affordable, ◐ partial, ○ revealed-unaffordable, ✓ owned/done), the flavor name, owned count, and next cost; Teased rows render dimmed with their marker and Hidden content is not rendered.

## Annotated code

- `crates/clowder-tui/src/nav/rows.rs`
- `crates/clowder-tui/src/render/mod.rs`

## What the blind runs found

All three A-runs built the full row anatomy — marker, name, owned count, next cost. All three B-runs recovered only the marker grading. The code does implement the rest; the blind readers could not tell it was required.

## Recommendation

The spec bundles five requirements in one sentence: marker set, flavour name, owned count, next cost, dim-when-teased, and absent-when-hidden. LID forbids compound "shall X and Y" for exactly this reason — the clauses cannot be individually cited, tested, or falsified. Split it.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
Agreement *within* a direction and disagreement *between* them is the signal.
