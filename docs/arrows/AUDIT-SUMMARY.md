# What the audit found — and what needs your decision

Safe to read: this page names no resource, structure, job, idea, event or
balance number. Everything is described by its shape.

Standing since 2026-08-10. 127 of 293 specs audited; all four shell segments
complete, every sim segment sampled. 628 tests pass.

---

## Needs your decision

Six items. Each is written up in full under
`docs/arrows/_experiments/bidirectional-differential/`.

### 1. The save grows without limit — *the one with a deadline*

Two specs disagree. One forbids retaining anything permanently unless its count
is bounded by construction, precisely because the save would grow forever. Its
sibling exempts absence-reports from eviction anyway, and the code implements
the exemption.

Measured: **1,337 bytes per report**, against a base save of 10,682. Opening the
game twice a day for a year retains ~730 of them — about 950 KB, at which point
they are ~99% of the save, parsed on every load.

The exemption is deliberate and its reason is good — an unread report must
survive being closed on. The question is what happens to one already read, or
one from eleven months ago. Three options are laid out in the write-up. The
tenets do not settle it: cozy argues for keeping what you might look back on,
idle-respect for a save that still loads after a year of respecting absence.

### 2. Five gates live in a query rather than in the core

Fifteen actions have no content id and answer through one gate query. Ten agree
with what the core would actually do. Five do not: four are hidden by the query
while the core would refuse them for cost, and one is hidden while the core
**accepts** it — returning success, changing nothing, and reporting a turning
point that did not turn. That last also breaks a general engine rule allowing a
command exactly two outcomes.

Nothing reaches them today, because the query hides the rows. Per action: is the
gate a real precondition (belongs in the core) or a display choice (belongs in
the shell, and the spec should say so)? Pinned by an ignored test.

### 3. Layout breakpoints are specified in two leaves at once

Two leaves state the same three rules about where panes appear and when the
layout may change. One hedges ("about 100 columns") where the other commits
(100). Editing either leaves the other reading as current intent, because it is
— of a different leaf. Which leaf owns them?

### 4. A behaviour has no requirement

One mechanic is implemented, works, and is named plainly in a design doc — and
no EARS states it. Its annotation went to the nearest plausible neighbour, a
spec about something else, which is how it surfaced: three blind readers
reconstructed the wrong mechanic and were right about the code they were shown.
Needs an EARS written, then a test, then the citation moved.

### 5. An ambiguity two specs cannot settle between them

Two specs attach the same rate and the same cap to *different quantities*.
Either one describes both loosely, or there are two mechanics sharing constants.
Only the design intent separates them.

### 6. Three older questions, still open

- Should a stock spent to exactly zero disappear from its list, or stay?
- Was one panel meant to be per-row, or is "for that row" loose phrasing?
- Does an absent truncation signal get built, or does the clause get dropped?

---

## Fixed along the way

Nineteen specs and their code were reconciled. In brief:

- Two specs were marked implemented while **both files naming them explained why
  they could not be covered** — notes whose stated blockers had since been
  removed and which nobody revisited. Tests added; the code was already right.
- Four more had **no test at all**. Writing them found item 2 above.
- A spec **pinned a save version that had moved**, and the fixture for the
  version players actually have was missing, so the migration a real save takes
  was untested. It works; now it is asserted.
- A guard against a non-finite input was **necessary, correct, and unstated** —
  it could have been simplified away with the suite staying green.
- Two specs described mechanisms the code never had: a time budget where the
  code bounds a count, and a tolerance the code does not apply. Both reconciled
  toward the code, with the reasons recorded.
- Twelve spec references used a short form invisible to the prefix grep the
  whole id scheme exists to serve.
- A spec cited *nowhere* was invisible to the coverage check, because that check
  looked for a citation before looking for a test.

## What now runs on every commit

`scripts/spec-test-coverage.py` — every spec marked implemented must have
something asserting it. Fails on a new gap **and** on a stale exemption, so its
own exemption list cannot rot the way the prose notes did. Currently clean, with
two documented exemptions that genuinely need a real terminal.

## What was tried and did not work

Four attempts to find a cheap substitute for the expensive method, recorded so
they are not tried again:

| Idea | Result |
|---|---|
| Citation crowding predicts drift | No discrimination |
| Inline vs item annotation predicts drift | Backwards, and noise |
| Compound specs drift more | 70% vs 63% — noise |
| Shared implementation predicts drift | 66% vs 64% — noise |
| Mine reconstructions for unstated behaviour | One ambiguity from 113 specs |

Four independent ways of cutting these specs, each chosen because it looked like
it should matter, and **every partition drifts at about 65%**. The recovery rate
is a property of the method, not of the specs: the blind runs recover roughly a
third of what they are shown, and which third is decided by whether a
requirement happens to be locally expressible in code. Negatives, clauses whose
other half lives elsewhere, rationale, and constraints on a table's values are
not, and they are scattered evenly through every leaf.

This is now predictable without running it. The remaining specs will yield about
ninety drift verdicts and fifty coherent ones, at roughly one real finding per
thirteen specs — one per eighty blind sessions.

## The remaining scope

166 specs unaudited. Recent yield is about one real finding per thirteen, at six
blind sessions each — roughly 1,000 sessions for perhaps a dozen findings, of
which the last several came from *chasing what the harness could not process*
rather than from the runs themselves.

My recommendation: bound the blind runs here and spend the remainder on the
compound specs — around 75 carry several requirements under one id, which cannot
be marked implemented independently and which a prefix grep cannot separate.
That is a traceability argument, and it was always the stronger one.

Your call; the loop continues either way.
