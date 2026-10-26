---
parent: sim
prefix: SIM-ACH
---

# Achievements

## Context and Design Philosophy

Achievements are the clowder's scrapbook: a record of the moments worth remembering, from the first face at the door to the far end of time, and a good many small, silly kitten moments in between. They are there for delight and do nothing else. No rule in the simulation reads them — no production, price, chance, gate, or random draw depends on whether one has been earned — so there is nothing to grind, nothing to optimise, and nothing a player who ignores them misses in play (Tenets 2 and 4).

This leaf owns the achievement table, the rule that notices when one is earned, and the record of what has been earned and when. How the record is shown — the overlay, the celebration, the chip — is the navigation leaf's.

## The Table

Each achievement is a typed id, append-only, with a definition:

```rust
pub struct AchievementDef {
    pub name: &'static str,   // what it is called, shown once earned (and while locked, if not secret)
    pub hint: &'static str,   // a playful nudge shown while locked and not secret; never the exact condition
    pub words: &'static str,  // what happened, shown once earned
    pub secret: bool,         // hidden entirely until earned
}
```

The table holds at least forty achievements (The Achievements, below), spread across the whole game — kittens and their profiles, building and crafting, research, pouncing and neighbours, sunbeams and wonders, the sky, dreams, and the Move — with many of them whimsical kitten moments rather than milestones. A good share are secret, so some are only ever found by surprise. Every name is distinct; a hint gestures at an achievement without stating its condition, so a locked list reads as an invitation, not a checklist. Names and words are content in this leaf; they name things in the game freely, since an achievement is earned by meeting them.

## The Achievements

In table order. A *state* condition reads the meadow as it stands; a *moment* condition reads the events just emitted. Hints and words are content in the table.

| # | Achievement | Kind | Earned when | |
|---|---|---|---|---|
| 0 | A Face at the Door | state | the roster holds a kitten |  |
| 1 | A Proper Clowder | state | the roster holds 10 kittens |  |
| 2 | Wall-to-Wall Whiskers | state | the roster holds 50 kittens |  |
| 3 | Too Many to Name (Almost) | state | the roster holds 100 kittens |  |
| 4 | Full House | state | the housing holds at least 10 beds and every one is taken |  |
| 5 | Once in a Blue Moon | state | a rostered kitten wears surprising fur | secret |
| 6 | Odd One Out | state | a rostered kitten has odd eyes |  |
| 7 | Strike a Pose | state | the roster holds a kitten in every pose |  |
| 8 | A Coat of Many Colours | state | the roster holds a kitten in every fur pattern |  |
| 9 | Snack Buddies | state | two rostered kittens share a favourite food | secret |
| 10 | Matching Outfits | state | two rostered kittens wear the same fur, pattern and colours alike | secret |
| 11 | Seconds, Please | state | a rostered kitten's name carries a numeral, the name pool having run out | secret |
| 12 | Wise Whiskers | state | an Elder is appointed |  |
| 13 | Moving Up | state | a kitten holds rank 1 |  |
| 14 | Senior Purrsonnel | state | a kitten holds rank 5 |  |
| 15 | Paws of Experience | state | a kitten holds 9000 skill in a job |  |
| 16 | Everyone's on Break | state | at least 10 kittens are at home and none holds a job | secret |
| 17 | Visiting the Neighbours | moment | a kitten wanders to the neighbour's yard |  |
| 18 | Home Again | moment | a kitten comes home |  |
| 19 | Pure Bliss | state | the clowder's mood is Blissful |  |
| 20 | Hangry | state | the clowder is Grumpy | secret |
| 21 | The Zoomies! | moment | the zoomies start |  |
| 22 | Party Animals | state | a jamboree is running |  |
| 23 | Home Improvement | state | a structure is built |  |
| 24 | Sprawl | state | 100 structures stand in all |  |
| 25 | Kitten Metropolis | state | 500 structures stand in all |  |
| 26 | Handmade | state | any crafted good is in stock |  |
| 27 | Busy Paws | state | a tinkerer is bound to a recipe |  |
| 28 | What a Mess | state | the mess reaches level 1 |  |
| 29 | Lights Flicker | state | the meadow uses more energy than it makes | secret |
| 30 | Humming Along | state | the meadow makes at least 20 energy and no less than it uses |  |
| 31 | A Bright Idea | state | an idea is researched |  |
| 32 | Well Read | state | 20 ideas are researched |  |
| 33 | Know-It-All | state | every idea is researched |  |
| 34 | Refined Tastes | state | 10 refinements are bought |  |
| 35 | House Rules | state | a house rule is adopted |  |
| 36 | Pounce! | moment | a pounce comes home |  |
| 37 | The Great Hunt (for String) | moment | a single pounce sends 50 squads | secret |
| 38 | Followed Home | state | a unicorn is in the meadow |  |
| 39 | Open for Business | moment | a trade comes home |  |
| 40 | Shh, They're Sleeping | moment | a caravan finds its neighbour napping | secret |
| 41 | Dear Neighbour | state | a neighbour holds pen pal level 1 |  |
| 42 | Best Friends Forever | state | a neighbour holds pen pal level 10 |  |
| 43 | Know the Neighbourhood | state | every tradeable neighbour is discovered |  |
| 44 | Sunbather | state | a rite is bought |  |
| 45 | Enlightened | state | the enlightenment tier reaches 1 |  |
| 46 | Glitter Parade | moment | a parade is held |  |
| 47 | Sweet Dreams | state | a dreamcorn is in the meadow |  |
| 48 | Bottled Contentment | moment | a jar of purrs is condensed |  |
| 49 | Over the Fence | state | a site is reached |  |
| 50 | Wish Upon a Star | state | the furthest site is reached |  |
| 51 | Whale Watching | state | a Star Whale is visiting |  |
| 52 | A Very Long Nap | moment | a deep nap ends |  |
| 53 | Good Morning | state | the Distant Dawn has risen | secret |
| 54 | Moving Day | state | the clowder has made a Move |  |
| 55 | Family Tree | state | the clowder is in its fifth generation or later |  |
| 56 | Remembered | state | the Family Album holds 100 kittens |  |
| 57 | Off on an Adventure | state | an adventure is under way |  |
| 58 | The Old Ways | state | a tradition is bought |  |
| 59 | A Whole Year | state | the meadow reaches year 1 |  |
| 60 | Old Meadow | state | the meadow reaches year 100 | secret |

## Earning

An achievement is earned the first time its condition holds, and never un-earned. Each condition is one of two kinds:

- **A state condition** — something the meadow now is or holds (a roster this large, a kitten of this kind in it, a building count, a standing, a generation). These are checked at every day close and after every applied command that tells of something happening — one that emits an event, a speed change aside — save those that read individual kittens (their looks, favourites, names, ranks, skills, jobs, and whereabouts), which a day close checks only when that day saw a kitten arrive or a new year begin. What such a condition reads changes only then or by a command that emits an event, except a kitten's skill, which grows a little every tick, and its whereabouts, which a lean pantry and a full one change as often as every day; both are noticed at the year's turn at the latest. A condition changed by a command that emits nothing — a craft, a tinkerer bound to a recipe — reads no individual kitten's looks or standing, and is noticed at the day's close. Because state conditions read the meadow as it stands, a meadow that already meets one is credited at its next check, which is how a save from before achievements earns everything it has already done.
- **A moment condition** — something that just happened (a kitten wandered off, a caravan napped, the zoomies started). These are checked against the events of each tick and of each applied command.

Earning one records it with the generation and the meadow's day it was earned on, and emits `AchievementEarned`, a milestone event, so it is journaled permanently and celebrated where milestones are. Checking takes no draw from the state RNG and changes nothing else.

Checks skip what is already earned. The daily checks read only small summaries of the meadow (counts, sets, flags, the clowder's mood and energy), and the checks that walk the roster run only on the few days a kitten arrives or a year turns, so a long catch-up stays well inside its bound (SAVE-009).

## The Record

```rust
pub achievements: BTreeMap<AchievementId, Earned>,

pub struct Earned {
    pub generation: u32,
    pub day: u64, // the meadow's day it was earned on
}
```

The record is canonical — what has been earned cannot be recomputed for a moment condition — and it survives every Move: it belongs to the player, not to a meadow (prestige leaf). The core answers, for each achievement in table order, whether it has been earned and when, and whether it is secret, so a frontend shows the scrapbook without a rule of its own.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| What an achievement does | Nothing in play | A small permanent bonus each | A bonus turns the scrapbook into a checklist to chase, and a player who walks away would fall behind one who hunts them (Tenet 2); a record of fun asks nothing |
| What a player sees before earning | Most shown with a playful hint; some secret until earned | All secret; all shown with exact conditions | All secret leaves nothing to look forward to; exact conditions read as chores. Hints invite, secrets surprise (Tenet 4) |
| When conditions are checked | State conditions at each day close and after each command, those reading individual kittens only on a day close that brought a kitten or turned the year; moments against each tick's and command's events | Every tick for everything; every condition at every day close; the kitten checks on every day a kitten wanders off or comes home too | A day close is soon enough to notice a meadow-sized fact; a year's turn is soon enough to notice a skill or a kitten's whereabouts. Walking every kitten every day would spend a large share of the catch-up bound (SAVE-009) re-reading kittens that have not changed, and a meadow whose pantry runs lean sees kittens wander off and come home on as many as one day in five |
| A save from before achievements | Credited by the state checks as soon as it loads and plays | Starting from nothing; reconstructing moments from the journal | A long-played meadow earning nothing for what it plainly has done would be a quiet loss; the journal keeps only a rolling window of moments, so reconstruction would credit some and not others for no reason a player could see |
| The game's speed | Earns nothing, and a speed change is no occasion to look | An achievement each for the fastest and slowest speeds | Speed is the pace the meadow is watched at, not something that happens in it, and it leaves no trace in the meadow (SIM-ENG-024); a record of it, or a look it prompted, would be the one trace |
| Which commands prompt a look | Those that emit an event | Every applied command | A command that tells of nothing happening may be one that must change nothing at all — appointing the Elder who already sits (SIM-ENG-058) — and a look after it could credit something; what the silent commands change is read at the day's close |
| Across a Move | Kept | Reset per meadow | Achievements belong to the player; the Move takes nothing the player is attached to (Tenet 1) |

## Open Questions & Future Decisions

### Deferred
1. Whether the Family Album should note which generation earned which achievement on its pages, when the album gains a surface.

## References

- Parent: `../sim-design.md`
- Shown by: `../../tui/navigation/` (the Achievements overlay, the celebration, the chip)
- Kept across Moves: `../prestige/`; saved and migrated: `../../persistence/`
