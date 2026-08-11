# Structural scan — the patterns the differential named, found by grep

The differential is expensive (six blind sessions per spec) and slow. After
eighteen specs it had named four recurring faults. Three of them are detectable
in the source directly, at no subprocess cost. This is that scan across all 293
auditable specs.

## What it found

| Pattern | Count | Where |
|---|---|---|
| Table-definition specs — outside the differential's reach | 12 | spread across ten sim leaves, one in theme |
| Compound specs — two or more testable claims in one line | 88 | navigation 27, engine 10, runtime 10, economy 9, then a long tail |
| Behaviour specs cited **only** on a value, never on code that acts | 2 | render, runtime |

## On that last number, and how it moved

The naive query — "spec cited on a `const`, a struct field, or an attribute" —
returned **72**. Filtering to specs whose text describes something the system
*does* rather than something it *is* cut that to **26**. Requiring that *no*
citation anywhere lands on a function cut it to **2**.

The first two numbers were wrong in the same direction, and for the same reason:
a citation on a constant is exactly right when the spec defines a value, and a
behaviour spec that cites both its constant and its function is annotated
correctly. Only a behaviour spec whose every citation is on inert state is
genuinely unreachable — a reader following the annotation lands on a field and
learns nothing about what the system does with it.

Reporting 72 would have been the same error as marking 44 specs implemented
because a suite was green: a number that is easy to produce, sounds serious, and
does not survive being checked.

## How to use this

- **The 12 table specs** should be excluded from differential scope. They are
  verifiable, but by a test asserting the table against its design doc — a
  reader of consuming code cannot reconstruct what a table must contain.
- **The 88 compound specs** are the large finding. Each bundles clauses that
  cannot be individually cited, tested, or falsified, which is how a suite comes
  to cover one clause while its siblings go unbuilt. Splitting them is
  mechanical work on the EARS files, and it does not need a differential to
  justify it — the differential found the pattern, and this scan sizes it.
- **The 2 orphaned behaviour specs** want their citations moved to the code that
  acts, not the state it acts on.

## What the scan cannot see

Misplaced citations *between functions* — a spec cited on a function whose
conspicuous behaviour belongs to a different requirement. That is the fault the
differential found in four specs so far, and it is a judgment about what code
appears to do, which no grep makes. Those still need blind readers.

## Addendum — over-subscribed annotations

Batch five found a sharper version of the placement fault: not a citation in the
wrong place, but **one place carrying too many citations**. Readers of the clock's
observation function produced near-identical reconstructions for two different
specs, because that one function body is cited by five.

Counting them needed two attempts. Keying on "the annotation and the line after
it" collapsed every annotated struct field onto the `#[serde(default)]` beneath
it and reported one site carrying 45 specs. Keying on the annotation's own line
gives **61 single annotations citing three or more specs at once**.

Of those, the module-header ones are legitimate — a file's opening `@spec` line
listing what the module covers is an index, not a claim about one function. The
genuine cases are function-level, and the largest are a content accessor (7), and
five functions carrying five apiece, including the clock observation and the
loop's step.

That is the third time in this audit a first count has overstated: 72 misplaced
citations became 2, and 64 over-subscribed sites became 61 of which most are
benign. The pattern is consistent enough to be worth stating as a rule — a number
produced by a heuristic scan is a hypothesis, and reporting it before checking is
how the 44 false markers happened in the first place.

## Addendum 2 — the harness can manufacture a finding

Batch six produced a result that looked like the over-subscription pattern and
was not. `TUI-RENDER-005` is annotated in five places in its file, one of them a
module header. The extractor walks from an annotation to the end of its enclosing
item; a module header has no enclosing item, so the walk ran to the end of the
file and handed the blind readers 252 lines. They reconstructed the module. Of
course they did.

The extractor now refuses any block over ~90 lines rather than passing it on.
Past that size the reader is reconstructing a module, not a requirement, and the
classification means nothing.

`TUI-RENDER-005` is recorded as `INCONSISTENT-BLIND` and left *out* of the audited
count, rather than dropped quietly — the count should say what has actually been
examined.

The same check vindicated the batch-five finding it was meant to test:
`clock.rs:67` really does carry five specs on one annotation line, confirmed by
reading the source. So the over-subscription pattern is real; one of its two
claimed instances was mine.

The standing rule from Addendum 1 holds and now has a sixth instance: a signal is
a hypothesis until something independent confirms it. Here the independent thing
was reading the annotation directly, which took under a minute and changed one
classification out of two.

## Addendum 3 — the re-run, and what it settles

`TUI-RENDER-005` was re-run under the capped extractor: 10 lines instead of 252.
**The finding survived.** Readers still described the three-way layout
classification rather than the width band this spec governs.

That settles the question the cap raised. Six of the shell findings came from
extractions over 90 lines, and the worry was that over-extraction had been
inflating them generally. At least for the one case tested, it was not — the
citation really does sit on a classification covering all three bands rather than
on the branch this spec owns.

So the earlier result was right, and the correction was still worth making. Those
are compatible: the instrument was faulty *and* the finding was real, and there
was no way to know which without checking. The audited count stays at what has
actually been examined rather than being retroactively inflated by a result that
happened to survive.

## Addendum 4 — the mirror fault, and a property worth keeping

Batch six's fault was extraction too large. Batch eight found its mirror: a
module-header annotation whose walk landed on the *import block*, handing the
readers three lines of `use` declarations.

One reader refused: *"This snippet can't support a reconstruction: it contains
only two `use` declarations."* It did not confabulate a requirement from
imports. That is worth recording as a property of the method rather than a
curiosity — a blind reader given nothing will sometimes say so, and that refusal
is a signal the harness is broken, not the project.

The extractor now skips blocks with fewer than four lines of actual logic, as
well as those over ninety. Both faults share a cause: annotations that sit at
module level have no enclosing item, and walking from them finds either
everything or nothing.

## A fourth unannotatable class

`TUI-NAV-021` requires every UI symbol to be single-width, with no emoji or
ambiguous-width glyphs. That is a constraint on *what characters the source
contains*, not on what any function does at runtime. No code can testify to it,
and the readers described the width-fitting helper the citation happens to sit
on instead.

The classes now known to be outside the differential's reach:

1. Negative requirements — "shall not open a dialog", "shall not pin headers"
2. Content tables — the table is data; consuming code cannot reconstruct it
3. Rationale clauses — "never from a separate formula that could diverge"
4. Source-level constraints — properties of the code's text, not its behaviour

All four are testable; none by this method. Each wants a test that asserts the
negation or the property directly, and the citation belongs on that test.

## Addendum 5 — partitioning what is left, and a correction to my own argument

Applying the four blind classes to the 247 unaudited specs:

| Class | Count | of which shell |
|---|---|---|
| Content tables | 11 | 1 |
| True negatives — the requirement *is* that something does not happen | 10 | 4 |
| Source-level constraints | 0 | 0 |
| Differential-able | 226 | 48 |

The first cut of this table said **70** negatives. That was wrong: the query
lumped *rationale* clauses ("so that X") in with true negatives, and a spec
carrying a rationale clause is perfectly auditable — SIM-ENG-030 proved it,
coming back coherent on its mechanism with only its "never from a separate
formula" clause unrecovered. Restricting to specs whose requirement genuinely is
an absence gives 10.

**This corrects an argument I had been making.** I had been recommending the
sweep be bounded partly on the grounds that much of what remained could not be
evaluated by this method. That premise was weak: only 21 of 247 are unreachable.
The honest case for bounding rests on the hit rate alone — the shell finds
something roughly two times in three, the sim roughly one in three, and the sim
is 178 of the 226 remaining differential-able specs.

That is still a case, but it is a different and smaller one than I was making,
and it is worth saying so rather than letting a convenient number stand.

## Addendum 6 — the method is weakest where the naming is best

`TUI-NAV-001` returned 76% word overlap, above the threshold the skill sets for a
suspected stripping failure. The guard was right. The implementing function is
named for the requirement it satisfies, so its identifier alone hands the reader
the spec's central term.

That is not a code smell — it is good naming, and stripping it would leave
nothing to reason about. It is a limit of the method: **a function named after
its requirement will always let a blind reader recover that requirement, and the
resulting agreement proves nothing.** Coherence and self-documentation are
indistinguishable from outside.

Fifth blind class, and the most awkward, because it selects against exactly the
code you want.

## Uncited specs — the count was 10, the answer is 1

**This section originally reported ten specs with nothing pointing at them, and
called it the most actionable finding of the audit. That was wrong.**

Nine of the ten are cited on *tests*. They are cited there because every one of
them is a negative requirement or a property — "shall never remove", "no mechanic
shall reduce", "shall render with no colour", a performance bound, a round-trip
identity. None has a production sink, so the citation belongs on the test that
asserts it, which is exactly the convention this audit had already recommended
two addenda earlier.

So the coverage mechanism was working, and my query was looking in one place
(`src/`) for citations that correctly live in another (`tests/`).

One spec is genuinely uncited: the engine's milestone-bounding rule. It is
implemented — the first-copy rule and the demoted calendar turn are both
instances of it — but only its sibling spec was cited at that site. Now cited at
both.

That is the seventh time in this audit a first count has overstated, and the
first time the overstatement had already been reported as a finding rather than
caught before reporting. The rule from Addendum 1 needs one more clause: check
where the evidence would live *if the thing were fine*, not only where it would
live if it were broken.

---

### Original section, retained



A sweep for specs marked `[x]` with no `@spec` citation anywhere in `src/`:

| Segment | Count |
|---|---|
| curiosity, engine, persistence | 2 each |
| ceremonies, dreamtime, kittens, theme | 1 each |

These are invisible to the differential, and to any coverage check built on
grepping `@spec`. A tool that counts citations will report them as fine by
omission — which is the precise shape of the failure that prompted this audit:
something claimed done, with no mechanism that could contradict the claim.

Two of them are worth naming: the engine's event-classification bounding rule and
its speed-journaling exclusion. Both were added late, both are real behaviour,
and neither is cited anywhere.

## Addendum 7 — cited, but not on anything that behaves

The fixed import filter surfaced a small class worth naming: specs whose *only*
source citation sits on a module header or import block. Four of them — two in
navigation, two in theme.

They are not uncited; a grep for the spec id finds them. But the citation points
at a file's opening doc comment rather than at code that does anything, so
following it tells a reader which module is involved and nothing more. A coverage
check counting citations scores them as covered.

One of the four had already been audited from the other direction: its readers
described the module's role table rather than the requirement, which is exactly
what a module-header citation would produce. The two methods agree.

This is the smallest finding recorded here, and it was checked before it was
written down — the count was four when I looked, and four after.
