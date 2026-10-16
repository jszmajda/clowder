# Arrow: curiosity

Curiosity is the game's research system: kittens investigate the world and their accumulated curiosity is spent on **Ideas** — one-time unlocks that open structures, jobs, and permanent modifiers. The

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 15 of 15 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/curiosity/curiosity-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/curiosity/curiosity-specs.md` — 15 specs, prefix `SIM-CUR`

### Tests
- `crates/clowder-core/tests/curiosity.rs`
- `crates/clowder-core/tests/economy_depth.rs`
- `crates/clowder-core/tests/society.rs`
- `crates/clowder-core/tests/wonder.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-CUR-001 – SIM-CUR-015 | 15 | 0 | 0 |

**Summary:** 15 of 15 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
