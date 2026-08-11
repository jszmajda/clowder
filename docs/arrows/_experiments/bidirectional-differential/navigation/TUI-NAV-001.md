# TUI-NAV-001

**Classification:** `INCONSISTENT-BLIND` · **Action:** `method-limit` · **Audited:** 2026-08-10 · **Extraction:** 41 lines

## Spec as written

> The system shall map keys only from the tmux-safe set — plain printable characters, Tab, BackTab, Enter, Esc, Space, Backspace, and the four arrow keys — plus Ctrl-C as emergency quit; no other Ctrl chord, Alt/Meta, or function key shall be bound.

## What the blind runs found

76% word overlap on the strongest run — above the threshold that flags a suspected stripping failure. Confirmed: the implementing function is named `is_tmux_safe`, so the identifier alone hands the reader the spec's central term.

## Recommendation

No project action. The finding is about the method: **the differential is weakest exactly where naming is best.** A function named for its requirement will always let a reader recover that requirement, and the resulting agreement proves nothing. Stripping the name would leave nothing to reason about, so this is a limit rather than a bug. Treat well-named implementations as a fifth blind class.
