# Two specs disagree, and the save grows without limit

**Found by:** following up an unactioned note from an earlier review, not by the
differential — this is a contradiction between two specs, and the blind runs
compare one spec against code
**Status:** reported, needs a decision

## The contradiction

`SIM-ENG-042` says an occurrence may be classed a milestone *only* where the
number of such occurrences is bounded over a playthrough by construction, and
gives the reason: anything retained permanently and recurring grows the save
without limit.

`SIM-ENG-029` says the rolling window evicts its oldest non-milestone entry, so
growth is bounded "without ever discarding a milestone **or a retained
away-report**".

An away-report is written every time the player returns from an absence longer
than the minimum interval. Its count is bounded by how often they close and
reopen the game, which is not bounded by construction — it is bounded by how
much they play. `SIM-ENG-042`'s rationale applies to it exactly; `SIM-ENG-029`
exempts it from eviction anyway.

The code implements the exemption.

## Measured

| | |
|---|---|
| A save of a played meadow, no away-reports | 10,682 bytes |
| The same save after one retained away-report | 12,019 bytes |
| Cost per report | **1,337 bytes** |

Opening the game twice a day for a year retains about 730 of them: roughly
**950 KB**, at which point the reports are some ninety-nine per cent of the
save. Nothing trims them, and every load parses all of them.

This is slow rather than sudden, which is why nothing has noticed. It is also
exactly the failure `SIM-ENG-042` was written to prevent, arriving through the
one door that spec's sibling holds open.

## What the decision is

Not a bug to fix quietly — the exemption is deliberate and the reason for it is
good: a report the player has not read yet must survive being closed on. The
question is what happens to one they *have* read, or one from eleven months ago.

Options, in the order they seem worth considering:

1. **Bound the retained reports** — keep the most recent N, evict older ones as
   the rolling window already does for everything else. Simple, and matches what
   `SIM-ENG-042` asks of everything else retained.
2. **Evict on read** — an unread report is protected, a read one becomes
   ordinary and ages out of the window. Matches the stated reason for the
   exemption most precisely; costs a read-flag in the save.
3. **Fold old reports** — collapse anything older than some horizon into a
   single summary. Keeps the history, loses the detail.

Whichever way it goes, the two specs need reconciling so they stop disagreeing,
and the tenets do not settle it: cozy argues for keeping what the player might
want to look back on, and idle-respect argues for a save that still loads
quickly after a year of respecting their absence.
