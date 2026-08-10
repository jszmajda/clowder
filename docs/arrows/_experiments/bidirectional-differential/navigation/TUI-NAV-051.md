# TUI-NAV-051

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall commit a heavy action only on a distinct, deliberate second Enter — a held or auto-repeating Enter shall arm the row and go no further, and X shall never commit a heavy action — so no heavy action can be reached by holding a key.

## Annotated code

- `crates/clowder-tui/src/nav/rows.rs`
- `crates/clowder-tui/src/nav/update.rs`

## What the blind runs found

Readers recovered the X-refusal — heavy rows reject the max-affordable shortcut with a footer — but not the held-Enter protection, which is the clause that actually stops a keyboard repeating an irreversible action.

## Recommendation

The guarantee is split across annotations: this spec cites the shortcut refusal, while the auto-repeat gate lives in code cited by TUI-NAV-010 and TUI-NAV-043. A reader of this spec's code sees half the safety property. Cite the repeat gate here too.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
