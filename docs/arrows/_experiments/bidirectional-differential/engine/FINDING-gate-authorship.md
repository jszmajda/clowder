# The gate query authors rules the core does not

**Found by:** writing the missing test for `SIM-ENG-045` (Addendum 11's scan)
**Status:** reported, not repaired — the repair is a cascade through other segments
**Pinned by:** `the_gate_query_authors_rules_the_core_does_not` (ignored test)

## What SIM-ENG-045 asks for

> …expose whether a command's gate is open — the precondition deciding whether
> a thing exists for this meadow yet, as distinct from whether it can be
> afforded — so a caller deciding what to show does not restate rules the
> command layer already owns.

The purpose clause is the load-bearing part: the query exists so the shell does
not restate a rule **the core already owns**. Where the core owns no such rule,
there is nothing to restate, and the query is not reflecting a gate but
inventing one.

## What is there

Fifteen actions have no content id and so answer through this query rather than
through `availability`. On a meadow at tick zero:

| | Count | Behaviour |
|---|---|---|
| Query and core agree | 10 | Gate closed, and the core calls it `Locked`. |
| Query stricter than the core | 4 | Gate closed; the core refuses for **cost**, not as locked. |
| Query stricter than the core | 1 | Gate closed; the core **accepts**. |

The four are basking, both refinements, and the meadow move. Each is hidden by
the query while the core would have said "you cannot afford this" — so the
reason the player is given and the reason the core holds differ, which is the
divergence `SIM-ENG-036` forbids for content and nothing forbids here.

The fifth is passing it down, and it is the sharp one. On a meadow that has met
nothing the core returns `Ok`, having mutated nothing, carrying an event that
announces a turning point which did not turn. `SIM-ENG-005` allows a command
two outcomes — fully applied, or a typed error having mutated nothing. This is
neither. Today nothing reaches it, because this query hides the row; that is
precisely the arrangement the spec's purpose clause exists to end.

A sibling detail found alongside: the meadow move reports
`InsufficientResources` with an **empty** list of what is missing — a shortfall
naming nothing, which is not the typed data `SIM-ENG-031` promises a caller.

## Why it stops here

The engine segment owns the query. The five actions belong to the prestige, sun
and refinement segments, and giving the core the gates it is missing means
editing their command paths and their specs. Across-segment cascade pauses for
review rather than proceeding.

## What the repair looks like

1. Decide, per action, whether the gate is a real precondition (belongs in the
   core, as a `Locked`) or a display choice (belongs in the shell, and the spec
   should say the query serves display rather than mirroring the core).
2. Whichever way each goes, the passing-it-down `Ok`-that-does-nothing is a
   defect under `SIM-ENG-005` on its own terms and wants a typed refusal.
3. Give the empty shortfall a cause, or a different error.
