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
