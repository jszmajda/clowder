# Two fields a Move keeps because nobody remembered them

**Found by:** asking whether a spec that quantifies universally is tested
universally — not by a blind run
**Status:** reported; the code is defensible, the spec is silent

## The claim and its enforcement

`SIM-MOVE-004` enumerates what a Move preserves and says everything else resets
to new-game defaults. The implementation resets **in place**, naming each field
in a hand-written list, and the test that covers it names each field too.

So a field added later is preserved by *omission*. It survives a Move because
nobody remembered it, the test does not mention it, and nothing fails.

Comparing every field of the state against the reset list and the spec's legacy
list: two are in neither.

| Field | What happens today |
|---|---|
| The roster high-water | Survives a Move |
| The journal | Survives a Move |

Both were added after the spec was written — one for the prestige reveal
ratchet, one for the event journal — and neither was cascaded into it.

## Whether that is wrong

Each is defensible on its own terms. A reveal keyed on having *met* a roster
size should not un-reveal after a Move, or an earned row vanishes from under the
cursor. And the Family Album already establishes that history outlives a
generation, so a journal that survives is in keeping.

But defensible is not specified. The spec is an exhaustive enumeration — that is
its whole value — and two fields are outside it, decided by an omission rather
than by anyone.

Note the journal one compounds the retained-report growth recorded elsewhere: if
absence reports survive Moves, they accumulate across generations too.

## Enforcement added

A test inside the crate destructures `GameState` exhaustively, with no rest
pattern. Adding a field now stops it compiling until whoever adds it puts the
field on one side of a Move or the other. It lives in the crate because several
of the fields are private.

That does not decide the two open ones. It makes the third impossible.
