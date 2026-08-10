# TUI-NAV-049

**Classification:** `BIDIRECTIONAL-DRIFT` · **Action:** `reconcile-EARS-or-code` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall list every revealed resource as an inspectable row in the home section's stores group, keeping the resource sidebar a curated display-only glance rather than the complete table.

## Annotated code

- `crates/clowder-tui/src/nav/rows.rs`
- `crates/clowder-tui/src/render/widgets/chrome.rs`

## What the blind runs found

The spec says every *revealed* resource is listed. All three readers described a different rule: resources are hidden while their stock is not positive.

## Recommendation

These are not the same thing, and the difference is visible in play — a resource spent back to exactly zero disappears from the list, taking its cursor memory with it, despite having been met. The shell uses stock-above-zero as a proxy because resources have no reveal concept in the core. Either the core grows a sticky resource-reveal notion the shell can ask for, or the spec should say 'held or once held'. An earlier code review flagged this same spot from the other direction, which is corroboration rather than coincidence.
