# A behaviour with no requirement, wearing another spec's id

**Found by:** the stratified sim sample — all three blind readers reconstructed
`SIM-SKY-005` as something it says nothing about
**Status:** reported, not repaired — the repair adds an EARS

## What happened

`SIM-SKY-005` is about skymap and hushfelt production and storage from named
outpost structures. Its six source citations are almost all about a different
mechanic: one outpost's extension of the Long Sun cap. Handed those, three
independent readers reconstructed the Long Sun bonus, unanimously and
confidently. They were right about the code they were shown.

One of the six is correct — the header over the outpost table, which is exactly
what the spec describes. The rest sit on the cap-extension effect key, its
per-structure value, its accumulation, and an adventure reward that raises it.

## Why the id landed there

Because there was nowhere else to put it. The cosmic design names the
behaviour plainly — that outpost grants +2 Long Sun cap each — and no EARS
states it. So the annotation went to the nearest plausible neighbour in the
same leaf.

That is the finding, and it is not really about citations. **An implemented
behaviour has no requirement.** Nothing states the intent, so nothing can
cascade to a test that asserts it, and the tests that touch it cite a spec
about skymaps. The misattribution is the symptom that made it visible.

## Why the structural scans missed it

Last cycle's sweep checked whether every spec could be *prepared* — whether an
annotation existed and yielded a region of workable size. `SIM-SKY-005` passes
that easily: six citations, plenty of code. What no structural check asks is
whether the cited code has anything to do with the spec. That question needs
someone who has read both, which is precisely what the blind runs are.

So the earlier conclusion — that the sim partition has no citation defects — was
too strong. It had none that preparability can see.

## The repair

An EARS for the cap extension, in the leaf whose design already describes it,
then a test, then the citations move off `SIM-SKY-005` and onto it. That adds a
requirement rather than editing one, so it waits for review.
