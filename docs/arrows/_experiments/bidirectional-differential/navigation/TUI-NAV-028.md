# TUI-NAV-028

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall change game speed one notch along 0.25→0.5→1→2→4→8 on < and >, applying SetSpeed and always showing the current speed in the header; from a loaded speed that is not on the ladder, the first press shall move to the nearest notch in the pressed direction, and at either end of the ladder a further press shall be a no-op.

## Annotated code

- `crates/clowder-tui/src/nav/update.rs`

## What the blind runs found

Readers recovered the ladder, the direction, and the no-op at either end exactly. None recovered "always showing the current speed in the header" — that half is drawn elsewhere and carries no citation for this spec.

## Recommendation

Another compound spec: the ladder mechanics and the header display are two requirements sharing a sentence. The header half is currently unfalsifiable — no annotation, no test. Split, and cite the chrome that draws the speed.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
Agreement *within* a direction and disagreement *between* them is the signal.
