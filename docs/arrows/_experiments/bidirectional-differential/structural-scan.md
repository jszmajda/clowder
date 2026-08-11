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

## Addendum 8 — the sixth blind class, and the list as an artifact

`TUI-RUN-001` requires the save to be resolved and loaded *before* the terminal
is taken over. Readers of the terminal setup described the terminal setup,
correctly. An ordering between two phases is not visible from inside either one,
so no citation on either phase can carry it.

Sixth class: **relational requirements** — a claim about how two components
stand to each other, where neither contains the relation.

The full list, which is the most transferable thing this audit has produced:

| Class | Why the method cannot reach it | Where the citation belongs |
|---|---|---|
| Negative requirements | No code exists for what does not happen | A test asserting the negation |
| Content tables | The table is data; consuming code cannot reconstruct it | A test comparing table to design |
| Rationale clauses | Reasons are not behaviour | The design doc, where they already are |
| Source-level constraints | Properties of the text, not of execution | A test scanning the rendered output |
| Self-documenting implementations | The identifier restates the spec, so agreement proves nothing | Nowhere — a known limit, not a defect |
| Relational requirements | Neither component contains the relation | A test observing both |

Five of the six point at the same remedy: the citation belongs on the test, not
on the code. That is a stronger claim than it looks. It says a meaningful
fraction of an EARS corpus is verified by tests *by nature* rather than by
choice, and that a coverage tool grepping `src/` for citations will under-report
that fraction as a gap — which is exactly the mistake made in this audit's own
uncited-specs finding, before it was corrected.

## Addendum 9 — saturation

Three consecutive batches have produced no new class of finding. Batch fourteen's
four results are one clean, one absence, one over-subscription, one relational —
every category already named and catalogued.

A useful diagnostic appeared in the same batch: three specs whose citations all
resolve to one 47-line function produced three near-identical reconstructions.
That is over-subscription visible without any interpretation — when N specs cite
one body, a blind reader returns the same composite N times, and the specs are
distinguishable only by whatever the reader happened to emphasise.

The method has saturated. It found six blind classes, five fault patterns, two
corroborations against independent review, and one genuine behaviour question, in
roughly its first seventy specs. It has since been re-finding those, at six
sessions each.

What remains genuinely unexamined is the sim's 178 differential-able specs, which
have returned findings at about one in three against the shell's two in three —
and where the findings that did appear were mostly of the classes now known to be
structural rather than local.

## Addendum 10 — a cheap substitute that does not work

Batch fourteen suggested a shortcut: three specs sharing one 47-line function
produced three identical reconstructions, so perhaps *citation crowding* — how
many specs share an annotation site — predicts which specs the differential will
find drift in. If so, a free grep could stand in for six blind sessions.

Tested against the 75 audited specs, both ways of measuring it:

| Metric | Coherent | Drifting | Gap |
|---|---|---|---|
| Specs at the spec's most crowded site | 3.29 | 3.53 | +0.24 |
| Specs at its least crowded site | 1.50 | 1.70 | +0.20 |

**It does not predict.** A gap of two tenths of a spec, on samples of 24 and 43,
is noise. Crowded citations and clean ones drift at indistinguishable rates.

The reason is visible in hindsight: nearly every spec has at least one crowded
site, because module headers list many specs at once. The metric saturates and
stops discriminating. And crowding evidently causes *composite reconstructions*
without causing *drift* — readers hand back the same summary for three specs, yet
that summary may still match all three.

Recorded because it was already recommended before it was tested. The shortcut is
withdrawn; there is no free substitute for the blind runs, and the case for
bounding the sweep rests on hit rate alone.

## Addendum 11 — what the differential cannot see

TUI-RUN-016 was audited on 2026-08-10 and classified `BD-COHERENT`, action
`none`: three blind readers recovered both halves of it from the code, and the
record notes it as a compound spec that reads cleanly anyway. All of that was
true. It also had no test.

The differential compares EARS against code. LID's arrow runs EARS → Tests →
Code. The middle term is not in the comparison, so a spec can pass this audit
at full marks while nothing asserts it — and marked `[x]`, it will read as
finished from every direction anyone normally looks.

Batch fifteen found two such specs by accident: TUI-RUN-015 and TUI-RUN-016,
both marked implemented, both named in test-file prose that explained why they
*could not* be covered — one blocker being a seam that has since been built,
the other a `todo!()` that has since been filled. Neither note was revisited
when its reason expired.

### The scan

For every spec marked `[x]`, find its `@spec` citations and ask whether any of
them attaches to a `#[test]`. Nine do not:

| | Specs | What it means |
|---|---|---|
| No test file mentions them at all | `SIM-ENG-042`, `SIM-ENG-044`, `SIM-ENG-045` | Nothing asserts the spec. Exercised only in passing by tests aimed elsewhere. |
| Named once in a shell fixture comment | `SIM-ENG-046` | Same, plus a citation that points at scaffolding rather than an assertion. |
| Asserted, but cited only at module scope | `TUI-RUN-002`, `-003`, `-010`, `-011`, `-013` | Coverage is real; traceability is not. A prefix grep for the spec finds a header, not the test. |

The first four are the newest queries in the engine — the layer added while the
shell was being built, where the tests-first order was inverted under time
pressure. That the defect concentrates exactly there is the expected shape.

### Why this is worth more than the crowding metric

Addendum 10 proposed citation crowding as a cheap detector and it did not
predict anything. This one is not a predictor of drift; it is a direct check of
an arrow invariant — a spec marked implemented should have something asserting
it. It found two real instances before it was written, which is why it was
written, and its probes agree with hand-checks on four specs.

It is also cheap enough to run on every commit, and belongs there rather than
in an experiment folder.

## Addendum 12 — the five module-scope citations, resolved

Addendum 11 split nine specs into four with nothing asserting them and five
asserted but cited only at module scope. The four became tests and found a core
defect. The five turned out not to be one category but two.

**Three had a stale "no seam" note.** `persist.rs` explained at length that the
load-failure policy — no save, a newer save, an unreadable save — had nowhere
to be observed: `runtime::persist` names the outcomes but no function below
`runtime::run` produces them, and `run` enters raw mode and loops. True when
written. The ordering tests in `loop_order.rs` had since built the seam it said
was missing: a child process running the real binary against a scratch home,
which observes the whole decision from outside, including what it left on disk.

Three tests added there. Two assert what had never been checked at all — that a
refused save is byte-identical afterwards with nothing new beside it, and that
an *unreadable* save is never moved aside the way an unparseable one is. That
second is the distinction the design leans hardest on, and until now nothing
observed it.

The third needed care: with no terminal available the binary fails at terminal
setup, which is *after* the load, so asserting "it kept running" would have made
the test a test of the harness. It asserts reaching terminal setup instead —
strictly downstream of the load, so reaching it proves the load did not refuse.

**Two were honest.** `TUI-RUN-002` and `TUI-RUN-003` need a real pty: entering
raw mode from a test would put the developer's own terminal into raw mode, and
faking one proves nothing about the real thing. `term.rs` says so plainly and
has said so all along. The difference between these and the stale notes is not
candour but currency — a note is only honest while its reason still holds, and
nothing rereads a note when its blocker is removed.

### Promoted out of the experiment

The scan is now `scripts/spec-test-coverage.py`, with the two pty specs as
documented exemptions. It exits non-zero on a new gap *and* on a stale
exemption, so the list cannot rot the way the prose notes did. Current state:
303 specs marked implemented, 328 asserted by a test, clean.

## Addendum 13 — a second detector, and why it is not shipping

Batch sixteen found a spec the extractor could not audit at all: `TUI-NAV-013`'s
only source annotation sat on a one-line field doc declaring that count rows
exist, while the behaviour lived in a handler whose marker was a plain comment.
The coverage scan is blind to that — the spec has two tests, so it looks
covered. This addendum is the attempt to detect the class, and the reason it
stops at a one-off pass.

### Two rounds of being wrong

The first cut flagged 79 specs as cited only on declarations. Most were
annotations *inside* function bodies — precise, not misplaced. Excluding those
left 14. Spot-checking found `SIM-MOVE-006` among them, cited on
`fn great_meadow_move`, which is exactly right: the detector reads only the line
carrying `@spec`, and that annotation runs across several lines, so its anchor
was invisible. Handling continuation lines gave 18.

### Why 18 is not a finding

Read against their spec texts, most of the 18 are correctly annotated. A spec
that says "define exactly the recipes of the CRAFT table" belongs on the table.
So does one that says "draw all randomness from a single serializable
generator", or "define exactly six palette roles", or "count ticks owed as
integers" — that last is a claim about a representation, and the field's type
*is* the behaviour.

Two were genuinely misplaced and are moved: the site gate, which was on the set
of reached sites rather than on the function that consults it, and act-to-the-
maximum, which was on the action variant and the shell-side cap but never on
the handler. One more, "use attention for every element that wants notice", has
no entry point to move to — the requirement is about call sites, and picking one
would be arbitrary.

Roughly one in nine flagged specs was actionable. The coverage scan's ratio was
nine in nine. **A structural check cannot tell a spec about a definition from a
spec about behaviour**; that needs the spec text, which is to say a reader. So
this stays a one-off pass, and `scripts/` gets no second script.

### What the pass found on the way

Reading eighteen spec texts against their citations turned up something the
detector was not looking for: `SAVE-014` still said the save carries version 3.
It carries 4 — the version moved when banked ticks were added and the spec did
not follow. Worse, `SAVE-013` asks for a committed fixture of *every* prior
version, and there was none for v3 — which is the version a player who has been
playing actually has. The migration path a real save would take was untested.

It works: the added field defaults, so a v3 save loads owing nothing. That is
now pinned by a committed fixture and two tests rather than by inspection.
`SAVE-014` no longer names a version number, since that is the thing about it
that will keep changing.

## Addendum 14 — the sim partition, and a correction

Running the extractor over all 175 remaining sim specs — cheap, since it is
local file processing — leaves 28 it cannot prepare. That set is where every
shell finding came from, so it was the first place to look.

| Why | Count | What it turned out to be |
|---|---|---|
| Annotated region too small | 21 | Short functions and table entries. Harness floor, not a defect. |
| Annotated region too large | 6 | Genuinely large functions, correctly cited. Harness ceiling. |
| No `@spec` in source | 1 | A reachability property with no production sink, asserted by two tests. Correct as it stands. |

**No arrow defects.** The shell's citation problems — a spec pointing at the
wrong file, a marker written as prose, a behaviour cited on a declaration — do
not replicate here.

### The correction

The obvious story is that the defects concentrate where the process was skipped:
the shell's loop was hand-written ahead of its tests and says so in its own
header. Checking the history rather than asserting it, that holds for some of
them and not others.

The four engine specs with no test, and the save-version spec that went stale,
both date from the days the shell was being built — the queries added to serve
it, and the format bump made for it. Consistent.

The twelve short-form spec ids do not. They were introduced on 2026-07-05, in
commits whose messages say *tests-first*, more than a month before the shell
existed. They survived a process followed correctly.

That is the more useful conclusion, and it is not the neat one. A careful
process protects against the defect classes it looks at. Tests-first catches a
spec nobody asserted. Nothing in it reads a spec *reference*, so nothing
noticed that a reference was malformed — and a malformed reference is invisible
to the prefix grep the whole ID scheme exists to serve. Care was never the
variable there.

## Addendum 15 — what the sample says, and what it does not

Thirteen sim specs, one per segment, three blind runs each way.

| Outcome | Count |
|---|---|
| Recovered whole | 6 |
| Partial, in classes already catalogued | 3 |
| Harness artifact — the reader was shown the wrong code | 3 |
| A real finding | 1 |

The sim reads far better than the shell did: six of thirteen came back exact,
formulas and constants and guard clauses included, where the shell rarely
managed that. The three partials are the familiar shapes — a negative clause, an
architecture claim no single region can show, a requirement whose second half
lives at another site.

### The artifact, named precisely

The extractor walks from an annotation to its enclosing item. For an annotation
on a function that is the function. For an annotation *inside* a body — on the
one line that computes the thing — it hands the reader the entire surrounding
function, which is to say everything except the precision the annotation
carried. One spec in the sample is annotated on exactly the right line and was
audited against a large reward routine two hundred lines wide.

A second shape: where a spec has several citations of very different sizes, the
largest dominates what the reader sees. Two of the sample's three artifacts are
this.

**The tempting conclusion does not hold.** Forty-three drift verdicts across the
whole audit rest on inline annotations, which looked like grounds to doubt them
all. Checking rather than asserting: inline-annotated specs drift at 64%,
item-annotated at 80%. Inline placement does not predict drift, and on fifteen
item-annotated specs the difference is noise either way. The artifact is real
case by case and does not show up as a bias in aggregate — so individual
verdicts want checking before they are acted on, and the aggregate picture of
saturation stands.

### The real finding

`SIM-SKY-005` — see `cosmic/FINDING-longsun-cap-has-no-spec.md`. An implemented
behaviour has no EARS, so its annotation went to the nearest plausible id in the
same leaf, and three readers dutifully reconstructed the wrong mechanic.

It also corrects Addendum 14. That said the sim partition has no citation
defects, on the strength of a scan that asks whether every spec can be
*prepared*. This one prepares beautifully. Nothing structural asks whether the
cited code has anything to do with the spec — that question needs a reader, and
the blind runs are the readers.

One finding in thirteen. At that rate the remaining 139 hold perhaps ten, for
278 more sessions.

## Addendum 16 — the extractor fix that did not fix it, and a change of reader

### The fix

The extractor now distinguishes an annotation *on* an item from one *inside* a
body. The first still yields the item. The second yields the statement the
annotation sits above, extended over following siblings only until there is
enough to reconstruct from — where before it yielded the whole enclosing
function.

Measured on the sample's worst case, the extraction fell from about two hundred
lines to nineteen, and the nineteen begin on the annotated line.

**The reconstruction did not change.** All three readers still described what
the surrounding code computes rather than what the spec states, because the
spec describes a value's *definition* and the annotated site *consumes* it. No
window can make a consumer look like a definition. Scoping was the wrong
diagnosis for that case; the fix is real and helps elsewhere, and it did not
help here.

### The reader changed

Mid-tick the blind sessions began returning a usage-limit message instead of a
reconstruction. Checked first whether any earlier output carried it: none did,
so no past classification rests on a non-answer.

Sonnet and Haiku both still run, so the method continues on Sonnet — but a
different reader is a different instrument. Four already-classified specs were
re-run on it before trusting it. All four kept their classification. Sonnet was
noisier on one (one run of three inverted "most recently arrived" to "least")
and better on another, recovering an emitted event that the earlier reader had
missed in all three runs.

Stable enough to continue, and worth stating that batches from here are read by
a different model than batches before.

### What the change of reader found

That recovered event came with something neither reader was asked about: both
models, unanimously, reported the speed command substituting 1.0 for a
non-finite value. The spec said only "clamp to [0.25, 8]".

The guard is right and necessary — `f64::clamp` propagates NaN rather than
bounding it, and speed is serialized, so a NaN would reach a save that cannot
represent it and poison every interval priced against it afterwards. Nothing
stated it and nothing asserted it, so it could have been simplified away with
the suite staying green.

Now stated in the spec, with the reason, and asserted for all three non-finite
values. The same shape as the cap with no requirement: implemented behaviour
that no one wrote down.

## Addendum 17 — three proxies, none of them predictive

Batch nineteen: eight specs, read by the new model, no new class and no new
finding. Two recovered whole, six partial in shapes already catalogued.

Its most interesting result is one spec. `SIM-ADV-002` specifies seven distinct
twists plus a handoff in a single EARS line, and is annotated at ten sites
across five files. Every annotation is correct. Each reader recovered a
different twist and none recovered the handoff — because no extraction of any
one site can yield the other six. That is not a citation defect; it is seven
requirements wearing one id.

Which raised the obvious question, and the obvious question has now been asked
three times with the same answer.

| Proposed proxy for "this spec will drift" | Drift rate | Against |
|---|---|---|
| Citation crowding (specs sharing a site) | 3.53 vs 3.29 mean | no discrimination |
| Inline annotation vs annotation on an item | 64% vs 80% | backwards, and noise |
| Compound spec (2+ clauses or 2+ "shall") | 70% vs 63% | noise |

**No structural property of a spec or its citations predicts whether the blind
runs will recover it.** Drift sits near 65% throughout. The catalogued causes —
a negative with no sink, a clause whose other half lives elsewhere, a rationale
about a change that has not happened, a constraint on a table's values — cut
across crowding, placement and compoundness alike. The failure is semantic, and
nothing that counts symbols will find it.

That closes the question of a cheap substitute. There isn't one, and the case
for bounding the sweep rests where it has rested since Addendum 10: on yield.

### What compoundness is still good for

Splitting `SIM-ADV-002` remains worth doing, and this addendum is careful not to
argue otherwise. Compoundness does not predict *drift verdicts*; it does make a
spec untraceable, because seven behaviours under one id cannot be marked
implemented or not independently, and a prefix grep cannot separate them. That
is a traceability argument, which is the argument that was always there.

## Addendum 18 — mining the reconstructions already paid for

Both real findings this cycle had one signature: every reader named a mechanism
the spec text does not mention. A cap with no requirement; a guard against a
non-finite value with no requirement. That is a *semantic* signal, so it escapes
Addendum 17's conclusion that no structural property predicts anything — and
113 audited specs already have their reconstructions on disk. Free to ask.

First attempt flagged eighteen specs on plain vocabulary — "allowed", "away",
"capping" — which is a difference of wording, not of behaviour. Useless.

Narrowing to code-shaped tokens only (CamelCase, SCREAMING_SNAKE, snake_case,
decimal literals) named by **every** reconstruction and absent from the spec
text gives seventeen specs. Triaged:

| | Count | What it was |
|---|---|---|
| A constant's *name* for a number the spec already gives | 14 | Benign — the spec says "at least 3", the code says `FAMINE_WANDER_DAYS`. |
| Already-known findings resurfacing | 1 | The breakpoints owned by two leaves. |
| Reader arithmetic, not code | 1 | A number three readers computed by summing the spec's own tiers; it appears nowhere in the source. |
| A genuine ambiguity | 1 | Below. |

### The ambiguity

Three readers named a soft cap on one colony's warming that `SIM-TRADE-005` does
not mention. The number and its rate are stated — in `SIM-TRADE-006`, the spec
that owns that mechanic, which is the correct place and the same arrangement as
the twist that four ceremony reconstructions named earlier.

But the two specs attach the same rate and cap to *different quantities*: one to
a success chance, the other to standing. Either one spec describes both loosely,
or there are two mechanics sharing constants. Settling it needs the design read
by someone who knows what it intends, so it is recorded and not called a defect.

### What the mining is worth

One ambiguity from 113 specs. The honest reading is not that the technique
failed but that **the already-audited set holds nothing more** — which is worth
knowing, because it means the remaining findings, if any, are in the 154 not yet
run rather than in what has already been read and filed.
