# TUI-NAV-039

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Extraction:** 17 lines

## Spec as written

> The system shall make every player-facing Command variant that belongs to a section reachable as a list row in that section — including the stand-down choice on a declare list and the unassign action on a count row — so that no sectioned command is reachable only through a mnemonic, the palette, or an accelerator key; the two commands belonging to no section (SetSpeed, ObserveSky) are reachable by their global keys instead.

## What the blind runs found

The readers described the exact inverse of the spec. This spec is about *completeness* — every sectioned command reachable as a row. All three described *omission*: rows dropped when their command's gate is closed.

## Recommendation

The citation sits on the filter that removes rows. A spec asserting nothing is missing, cited on the function whose job is to remove things. The reachability claim is made by the builders that emit rows, and by the test census that enumerates every command variant — the latter is where it belongs, since completeness is a property of the whole list rather than of any one function.
