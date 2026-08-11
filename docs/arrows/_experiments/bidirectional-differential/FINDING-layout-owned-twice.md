# Layout breakpoints are specified in two leaves at once

**Found by:** three specs the extractor could not prepare in batch seventeen
**Status:** reported, not resolved — resolving it edits two LLDs, which this
loop holds constant

## What is duplicated

| Navigation leaf | Render leaf | Same claim |
|---|---|---|
| `TUI-NAV-017` — persistent pane at "about 100 columns", swap at 80 | `TUI-RENDER-005`, `-006` | where the inspector is a pane and where it swaps |
| `TUI-NAV-048` — three columns at "about 120 columns or wider" | `TUI-RENDER-007` | where the list stops growing |
| `TUI-NAV-047` — a dead-band at the inspector's breakpoint | `TUI-RENDER-008` | that a wobble must not flip the layout |

Two leaves state the same three rules. The wording differs in a way that
matters: navigation hedges ("about 100", "about 120") where render commits
(100, 120). Nothing makes them agree. Edit one and the other still reads as
current intent, because it *is* current intent — of a different leaf.

## How it surfaced

Not by anyone reading both files. The differential's extractor could not
prepare `TUI-NAV-047` or `TUI-NAV-048` at all: their only source citations sat
in the module header of the inspector *widget*, which renders inspector content
and decides no layout. Chasing why the extraction was empty led to the code that
does decide it — `render::layout` — which cited only the render leaf.

So the duplication was invisible from either end. Each leaf's specs looked
covered; the code looked annotated; only the absence of an anchor showed that
one leaf's specs pointed at a file that does not implement them.

## What was done

The citations moved to where the behaviour is, and the constants now name both
leaves — one dead-band at every breakpoint satisfies the general requirement and
the specific one; one cap breakpoint satisfies both statements of it. The
inspector widget says plainly that layout is not decided there.

That makes the duplication visible in the code, which is where it should have
been visible all along. It does not resolve it.

## What resolving it looks like

One leaf owns the breakpoints and the other cites them. Render is the natural
owner: the rules are about width and drawing, `render::layout` implements them,
and the render leaf states them without hedging. Navigation would then say what
it actually needs — that the inspector is reachable at every width, and that a
resize preserves cursor and armed state — without restating where the columns
fall.

That is an edit to two LLDs and their specs, so it waits.
