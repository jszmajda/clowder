# TUI-RUN-015 — commands before ticks

**Classification:** `B-ONLY-DRIFT`
**Runs:** A=3, B=3

## What the blind readers saw

All three B-runs described the drain loop accurately — arrival order, a fixed
per-frame maximum, footer errors, resize coalescing — and not one of them
recovered the ordering the spec is about. They reconstructed TUI-RUN-016.

The reason is structural. "Commands before ticks" is a claim about two
operations, and the cited region contains only one of them. A reader given the
drain loop can see everything the drain does and still cannot see what it
precedes. The second operand is out of frame.

## Finding — the spec was marked implemented with no test

Both files naming this spec said in prose that it could not be covered:
`loop_order.rs` because a child process observes a run rather than a frame, and
`loop_body.rs` because `nav::update` was `todo!()` at the time. Both blockers
were gone — `step` takes its events and clock reading as arguments, and
navigation resolves — and the marker had been `[x]` throughout.

A spec whose only record of coverage is a note explaining the absence of
coverage is not covered, whatever the suite's colour.

## Repair

Test added: a frame worth twenty ticks carrying one keypress, asserting the
resulting journal entry is stamped with the tick the frame opened at. The gap
is the discriminator — applied first it stamps 0, applied after the catch-up it
would stamp 20. The code was already correct; only the assertion was missing.

`loop_order.rs` no longer cites what it does not assert.
