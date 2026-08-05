# Arrow: persistence

Persistence turns the deterministic sim into a game that survives closing the terminal: versioned human-readable saves, atomic writes, and offline catch-up that replays the *same* `tick()` the live ga

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 17 of 17 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/persistence/persistence-design.md`

### EARS
- `docs/intent/persistence/persistence-specs.md` — 17 specs, prefix `SAVE`

### Tests
- `crates/clowder-core/tests/economy_depth.rs`
- `crates/clowder-core/tests/frontier.rs`
- `crates/clowder-core/tests/legacy.rs`
- `crates/clowder-core/tests/persistence.rs`
- `crates/clowder-core/tests/society.rs`
- `crates/clowder-core/tests/wonder.rs`
- `crates/clowder-tui/tests/persist.rs`

### Code
- `crates/clowder-core/src/journal.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-tui/src/runtime/mod.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SAVE-001 – SAVE-017 | 17 | 0 | 0 |

**Summary:** 17 of 17 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
