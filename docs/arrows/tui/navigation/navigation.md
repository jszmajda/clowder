# Arrow: navigation

This leaf owns how a human drives clowder from a keyboard: the key model, section navigation, the action model (select / act / repeat / bulk / confirm), the inspector, the glanceable chrome, discovera

## Status

**PARTIAL** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 35 of 57 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/navigation/navigation-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/navigation/navigation-specs.md` — 57 specs, prefix `TUI-NAV`

### Tests
- `crates/clowder-tui/tests/chrome.rs`
- `crates/clowder-tui/tests/composed.rs`
- `crates/clowder-tui/tests/inspector.rs`
- `crates/clowder-tui/tests/keys.rs`
- `crates/clowder-tui/tests/overlays.rs`
- `crates/clowder-tui/tests/rows.rs`
- `crates/clowder-tui/tests/update.rs`

### Code
- `crates/clowder-tui/src/nav/action.rs`
- `crates/clowder-tui/src/nav/keys.rs`
- `crates/clowder-tui/src/nav/mod.rs`
- `crates/clowder-tui/src/nav/rows.rs`
- `crates/clowder-tui/src/nav/section.rs`
- `crates/clowder-tui/src/nav/update.rs`
- `crates/clowder-tui/src/nav/view.rs`
- `crates/clowder-tui/src/render/mod.rs`
- `crates/clowder-tui/src/render/widgets/chrome.rs`
- `crates/clowder-tui/src/render/widgets/inspector.rs`
- `crates/clowder-tui/src/render/widgets/overlays.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | TUI-NAV-001 – TUI-NAV-057 | 35 | 0 | 22 |

**Summary:** 35 of 57 active specs implemented; gaps below.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

### Must Fix
1. `TUI-NAV-004` not implemented
2. `TUI-NAV-037` not implemented
3. `TUI-NAV-006` not implemented
4. `TUI-NAV-038` not implemented
5. `TUI-NAV-053` not implemented
6. `TUI-NAV-011` not implemented
7. `TUI-NAV-015` not implemented
8. `TUI-NAV-041` not implemented
9. `TUI-NAV-018` not implemented
10. `TUI-NAV-020` not implemented
11. `TUI-NAV-022` not implemented
12. `TUI-NAV-044` not implemented
13. `TUI-NAV-024` not implemented
14. `TUI-NAV-046` not implemented
15. `TUI-NAV-027` not implemented
16. `TUI-NAV-050` not implemented
17. `TUI-NAV-055` not implemented
18. `TUI-NAV-056` not implemented
19. `TUI-NAV-029` not implemented
20. `TUI-NAV-030` not implemented
21. `TUI-NAV-035` not implemented
22. `TUI-NAV-036` not implemented
