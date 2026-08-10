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
