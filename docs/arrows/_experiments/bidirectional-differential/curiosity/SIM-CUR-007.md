# SIM-CUR-007

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> When an idea's cost includes a gate craft (stories, anthologies, schemes, starshards, echoes), ResearchIdea shall spend curiosity and the gate components together, all-or-nothing, under the same epsilon-tolerant affordability rule.

## Annotated code

- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`

## What the blind runs found

All three readers recovered the rejection precedence and the atomicity of the spend — that the whole transaction fails rather than partially applying.

## Recommendation

No action. The atomicity clause being recovered independently by every reader is a good sign: it is the part most easily lost.
