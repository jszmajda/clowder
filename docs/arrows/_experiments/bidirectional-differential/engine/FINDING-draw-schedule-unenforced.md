# "Never reorder" had nothing enforcing it

**Found by:** the same question as the Move fields — does a spec that quantifies
get tested that way
**Status:** enforcement added, deliberately ignored by default

## The claim

`SIM-ENG-018` fixes the order a day boundary consumes random draws, lists them,
and ends: *new subsystems append to (never reorder) this schedule.*

That last clause is the one that matters to anyone already playing. The order
decides what a given seed produces, so inserting a draw in the middle rewrites
the future of every save already on disk — the same meadow, from the same
moment, plays out differently.

## What was checking it

Nothing. The test the spec cites checks that *events* come out in the right
order — returns before arrivals, daily processing before the random-events
phase. That is a different claim and a true one.

Every determinism test compares two runs of **the same build**. Two runs of one
build share whatever schedule that build has, so they agree whether or not the
schedule moved since yesterday. The property they prove — that the generator is
deterministic — is not the property at risk.

## What now checks it

A golden: the generator's position after twenty thousand ticks from two fixed
seeds, pinned to committed literals. An inserted draw moves it. A reordered draw
moves it. A changed probability that gates a later draw moves it.

**Ignored by default, on purpose.** That last case means it fails for good
reasons as well as bad ones — any balance change altering how often a roll fires
moves the number too, and during development that is frequent. As a gate on
every commit it would be noise nobody reads; run deliberately before shipping,
it is exactly the notice that matters.

A failure is not a bug. It is notice that saves already written will diverge
from where they were heading — a thing to decide rather than to discover.

## The shape, for the third time

An enumeration in the spec, a hand-written list in the code, and no check that
the two still agree. It found a missing migration fixture, two fields a Move
keeps by omission, and now this. In each the code was defensible and the
enforcement was absent; in each, the absence was invisible to a green suite.
