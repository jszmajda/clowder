---
parent: sim
prefix: SIM-ADV
---

# Adventures

## Context and Design Philosophy

Adventures are clowder's challenge modes, reimagined as *optional themed outings* the clowder chooses to try — the inspiration's self-imposed challenge runs, kept entirely opt-in and kept cozy. An Adventure is declared *before* a Move; the next meadow plays under the Adventure's gentle twist, and completing its goal earns a small permanent reward that stacks (diminishing) across attempts. Nothing about an Adventure is punishing: the twists are flavor and mild constraint, never loss, and declining every Adventure forever is a complete way to play.

The leaf owns the Adventure catalogue, the declare/complete lifecycle, and the permanent LimitedDR-capped reward pool. It contributes reward effects through the effect-key system and reads Move state from the prestige leaf.

## Lifecycle

- `DeclareAdventure(id)` (available once the Adventures surface is unlocked — the Adventurous Spirit prestige tradition OR the same-named curiosity idea, either suffices) arms an Adventure for the *next* Move. Only one may be armed at a time; re-declaring replaces it; declaring None disarms.
- On the next Move, the armed Adventure becomes *active* for the new meadow, applying its twist.
- While active, the meadow tracks the Adventure's goal. When the goal is met, the Adventure is *completed*: its completion count increments and its reward pool grows (LimitedDR-capped), and the twist lifts at the following Move.
- A never-completed Adventure simply stays active until the player Moves again with a different (or no) Adventure armed — no failure state, no penalty.

## The Catalogue (~7 adventures)

Twist (while active) → goal → reward per completion (LimitedDR-capped):

| # | Adventure | Twist | Goal | Reward |
|---|---|---|---|---|
| 0 | **Quiet Retreat** | no jamborees, softer arrivals | reach 50 kittens | +5% spring catnip (cap ×2) |
| 1 | **Long Winter** | perpetual winter | reach a Sunbeam Altar | +5% winter catnip and +5% Sunning-Deck summer energy (cap ×2) |
| 2 | **Nap Time** | kittens half as productive (lazy) | build a Yarn Computer | +0.2 master-skill multiplier (cap ×4) |
| 3 | **Frugal Meadow** | energy costs +10% | build the full energy set (Dynamo, Stream Wheel, Star Basin) | −2% energy consumption (cap ×1) |
| 4 | **Silent Sun** | ceremonies unavailable | Move with a full Memory Chest | +10% Long Sun effectiveness and +cap (cap ×4) |
| 5 | **Gentle Paws** | pounces yield nothing | research the Pen Pal Networks idea, then Move | +10% pounce ratio and +1 trade-knowledge (cap ×2) |
| 6 | **Tidy Meadow** | mess accrues faster | reduce mess to zero for a full year | −exclusive tidy tradition unlock (one-shot) |

Rewards stack across completions with per-reward LimitedDR caps (the inspiration's per-effect challenge caps), so an Adventure stays worth re-running without ever breaking balance. All twists are *slowdowns or removals of options*, never deletions of the player's things (Tenet 1).

## What an Adventure Says

Each Adventure carries a **blurb**: a short joke, then in plain words its twist, its goal, and what one completion adds to its reward, marking each resource it names as the curiosity leaf's blurbs do (SIM-CUR-014). The shell shows it as what the Adventure is, so a player declares one knowing what it asks and what it gives (Tenet 3).

- *It states what the core applies.* Every figure a blurb states is the figure the core applies — the twist's, the goal's, and the reward's — and a part of a catalogue entry the core does not apply is left out of its blurb rather than promised. A player reads the blurb to choose, so a promise the meadow does not keep is worse than a shorter blurb.
- *A reward is stated as a ceiling.* The pool is LimitedDR-capped, so a completion adds its full amount only while the pool is well under its cap, and less as it nears it. The blurb states the reward's per-completion amount as the most one completion adds and its cap as the most every completion together comes to: both are upper bounds, so neither reads as a gain every completion is sure to bring.
- *Goals by the names the player reads.* A goal that asks for a structure or an idea names it as that item's own row names it, so the blurb points at a row the player can find.

## Cozy Notes

- The inspiration's challenges include an apocalyptic "Post-Apocalypse" start on a dead world and an "Anarchy" that halves productivity. Clowder keeps the *shape* (a themed constraint that pays a permanent buff) but the constraints are cozy — a quiet retreat, a long winter, a lazy nap — and every one is opt-in.
- No Adventure ever costs the player kittens, structures, or resources beyond the meadow's ordinary Move.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Opt-in, armed before a Move | Declare → active next meadow → complete | Always-on challenge toggles | Matches the inspiration's reset-coupled challenge model and keeps Adventures a deliberate choice |
| Rewards LimitedDR-capped per effect | Stacking with per-reward caps | Uncapped stacking; one-shot rewards | The inspiration's proven infinite-replay-without-breaking-balance mechanism |
| Twists are cozy constraints | Slowdowns/removals of options only | Keep the grim challenges (apocalypse, anarchy) | Tenet 1 — the challenge *shape* survives; the theme becomes gentle |
| What an Adventure says about itself | A hand-written blurb per Adventure, as traditions and rites carry | A statement the shell assembles from the definition's fields | Twists and goals are engine behavior, not definition fields (SIM-ADV-002, SIM-ADV-004), so no field holds them for a shell to state; a blurb keeps the words in the core's table beside the reward they describe |
| Blurb scope | Only what the core applies, the reward as a ceiling | The catalogue entry as designed; a fixed gain per completion | Tenet 3 — the blurb is the plain statement behind a playful name, and a statement the meadow does not keep, or a gain the LimitedDR cap would shrink, misleads the choice it exists to inform |

## Open Questions & Future Decisions

### Deferred
1. A cosmic-tier Adventure (a long voyage) — lands with the cosmic wave.
2. Whether completing all Adventures grants a capstone Tradition — a nice-to-have, decided after play.
3. Long Winter's **Sunning-Deck summer-energy** reward half. The winter-catnip half ships through `SeasonResourceRatio(Winter, Catnip)`; the summer-energy half has no clean effect key yet (a per-season energy-production ratio) and is deferred until one lands.

## References

- Parent: `../sim-design.md` (adventures as cozy challenges, Tenet 1)
- Move coupling & Adventurous Spirit tradition: `../prestige/`; twists read season/energy/ceremonies/mess from their owning leaves
