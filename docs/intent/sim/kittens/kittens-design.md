---
parent: sim
prefix: SIM-KIT
---

# Kittens

## Context and Design Philosophy

Kittens are the heart of the game and the reason the cozy invariants exist. This leaf owns the population lifecycle (arrival, wandering off, returning), kitten identity (names), job assignment mechanics, and the happiness/mood computation. It exposes exactly two values to the economy leaf — effective workers per job and the mood multiplier — and consumes housing capacity, catnip stock, and unlock flags.

Every kitten is an individual with a name, and the roster is append-only: **a kitten, once arrived, is never removed**. Hard times move kittens to the neighbor's yard; good times bring them home. The player should feel that the clowder is theirs, permanently.

## Kitten Identity

```rust
pub struct Kitten {
    pub name: String,          // drawn from the name pool, unique in this save
    pub job: Option<JobId>,    // None = loafing
    pub location: Location,    // Home | NeighborsYard
    pub arrived_day: u64,
    pub job_seq: Option<u64>,  // stamp from the state's monotone counter at assignment
    pub away_seq: Option<u64>, // stamp at departure; cleared on return
    pub skills: [f64; 8],      // per-job skill XP, JobId-indexed (serde zero-fills)
    pub exp: f64,              // lifetime pool, spent on promotions
    pub rank: u8,              // promoted with exp + shinies
    pub trait_id: KittenTrait, // rolled at arrival; active only while Elder
}
```

The two `_seq` stamps come from the engine's `next_seq` counter and exist to make ordering rules implementable from stored state alone: "most recently assigned" = highest `job_seq`; "first out, first back" = lowest `away_seq`. Arrival order alone cannot reconstruct either across multiple famine episodes.

**Name pool** (content table, ~40 entries): Biscuit, Mochi, Pixel, Waffles, Miso, Clementine, Tofu, Pepper, Marmalade, Noodle, Pretzel, Juniper, Bean, Dumpling, Ziggy, Olive, Paprika, Maple, Sprocket, Turnip, Gnocchi, Butterscotch, Fig, Pancake, Nimbus, Tangerine, Pickles, Comet, Bramble, Cricket, Fennel, Sesame, Plum, Wasabi, Doodle, Crumpet, Nutmeg, Socks, Clover, Captain Whiskers. Names are drawn without replacement via RNG; when the pool exhausts, reuse begins with Roman-numeral suffixes (Biscuit II) — the clowder can outgrow the pool. The remaining pool is *derived from the roster*, never stored: the current cycle is the smallest suffix count at which some base name is unused, and the draw is a uniform RNG index over the remaining names in content-table order, each cycle drawn without replacement in turn.

## Arrival

Checked once per day (engine day boundary), when there is at least one free housing slot **and** catnip stock ≥ 50:

- Base chance 15%, +5% per full 100 catnip in stock, capped at 60%; one idea then adds +15% *on top of the cap* (effective maximum 75% — the idea's bonus is always real).
- **Pity counter:** after 4 consecutive failed eligible checks, the next eligible check succeeds automatically. Ineligible days (no free slot, or stock < 50) *pause* the counter — they neither extend nor reset it. Dry streaks are frustration, not drama.
- On success: a kitten joins (Home, loafing), gets a name, resets the pity counter, and emits `KittenArrived`.

At most one kitten arrives per day. Housing capacity = sum of structure housing effects (economy leaf). **Every rostered kitten holds a slot, Home or Away** — a wandered-off kitten's cushion is kept warm for its return, so a return can never find the house full and an arrival can never displace an absentee. Free slots = capacity − roster size.

## Jobs

Assignment is by count, targeted at individuals: `AssignJob(job)` puts the **most recently arrived** idle Home kitten into the job (newest kittens are keenest) and stamps its `job_seq`; `UnassignJob(job)` returns the **most recently assigned Home** worker (highest `job_seq` among Home holders) to loafing. If the job's only holders are Away, `UnassignJob` returns `NoWorkersAssigned` — they aren't here to reassign. Deterministic selection, and events name the kitten (`{name} became a Gardener`) — identity makes assignment delightful rather than bureaucratic.

Effective workers per job = count of Home kittens holding that job. Away kittens keep their job title but don't count (they're not here).


## Skills, Ranks, and the Elder

**Skills.** A working Home kitten gains 0.01 skill XP/tick in its current job × (1 + skill-XP effects), mirrored into its lifetime `exp` pool; every *other* job's skill decays 0.001/tick (floor 0) — switching jobs means retraining, specialization is rewarded (a refinement later removes the decay). Skill tiers multiply that kitten's own yield: <100 +0%, ≥100 +1.25%, ≥500 +2.5%, ≥1200 +4.5%, ≥2500 +7.5%, ≥5000 +12.5%, ≥9000 +18.75%. Production therefore resolves per kitten, in roster order (determinism).

**Ranks.** `Promote(kitten_index)` spends 500 × 1.75^rank exp (from that kitten's pool) plus 25 × (rank+1) shinies, incrementing `rank`. Rank scales the Elder's leadership and a Tinkerer's ability to work high-tier recipes without penalty (each tier above rank: −15% effectiveness, floor 25%).

**The Elder.** `AppointElder(kitten_index)` names one kitten (replaceable anytime, never removable by the game). The Elder's own-job yield ×(rank+1)/1.4 (min ×1.0), and its birth trait activates clowder-wide: Crafty +5% craft ratio; Brassy +10% metal crafts (badges/brass/cogs/moonsteel); Alchemical +7.5% process crafts (pavers/moonmilk/dreamweave/cometsilk); Charming +3% trade ratio; Bold +0.5 pounce ratio; Curious −5% idea curiosity costs; Serene −9% rite costs; None — nothing (some cats simply are). Traits are rolled uniformly at arrival (one RNG draw after the name draw, appended to the fixed schedule).

## Jamborees

Jamborees unlock in band 1 but their charm bill (1500) is deliberately a band-2 project — the idea researches early and the ambition teases (the silhouette philosophy, applied to a command). `HostJamboree` spends 800 pep + 1500 charm + 40 quills and sets a 100-day (one game year) jamboree: +30 happiness points × (1 + jamboree-ratio effects), arrival chance × (2 + jamboree-arrival effects), luxury drain ×1.2 (celebrations consume). One jamboree at a time (no stacking until the relevant Tradition); hosting while one runs is rejected with Locked (the shell's hint should read "already celebrating"). Ends silently; the shell may celebrate.

## Happiness (Mood)

Recomputed each tick (derived, never stored):

```
happiness = 100%
          − 3% × max(0, home_kittens − 5)
                × (1 − limited_dr(0.048·StoryCircles + 0.075·MoonlightStages, 0.9))
          + happiness-point effects (posts, hearths, biscuit idea, altars' rites later)
          + 1% × min(loafers, 5)                  (a loafing cat improves any room)
          + 10% per distinct stocked luxury (feathers, bells, mint — dust guard applies)
          + 30% × (1 + jamboree ratio) while a jamboree runs
          − mess penalty (economy leaf, level ≥ 2)
```

Floor at 25%, no upper cap. The **mood multiplier** applied to job production is `happiness / 100`, ×0.25 while the clowder is Grumpy (below). Structures are never mood-affected.

Mood tiers for event/reporting purposes: **Blissful** (≥120), **Content** (≥90), **Restless** (≥60), **Fussy** (<60), and **Grumpy** — a famine override that outranks the happiness tiers while active. `KittenMood` events fire on tier transitions only (entering/leaving Grumpy is a transition).

## Famine and the Neighbor's Yard (starvation, replaced)

When consumption falls short (demand exceeds stock — the economy leaf's *shortfall*), kittens simply don't finish their meal — nothing extra is deducted, nobody is harmed — and the clowder is **Grumpy**: job production ×0.25 (they nap through their shifts). A tick whose stock exactly covers demand is *not* a shortfall — ending the day with a licked-clean bowl is thrift, not famine.

- **Famine clock:** at each day close, if the day ended in shortfall (the engine's persisted Grumpy flag at the boundary), `famine_days` increments and `recovery_days` resets. On the day close where `famine_days` reaches 3 — and each famine day-close after — one Home kitten wanders to the neighbor's yard (`KittenWanderedOff`), most recent arrival first (the newest bonds are loosest); its `away_seq` is stamped. Wanderers keep their name and job title, stop eating and working. **The last Home kitten never wanders** — someone has to mind the sunbeam. The meadow is never empty, the player always has a face to come back to, and consumption never reaches zero during an absence (which would make an abandoned pantry read as "recovery").
- **Recovery clock:** at each day close without shortfall, `recovery_days` increments and `famine_days` resets. While `recovery_days ≥ 1` **and the day closed with catnip in stock**, one away kitten returns per day close (`KittenReturned`), **first out, first back** (lowest `away_seq`, an absent stamp sorting lowest — the ones who left earliest missed home the most); the stamp clears on return. Kittens come back when the pantry has actually refilled, not merely when nobody went hungry. A return resets neither clock; returns continue daily while recovery holds.
- The two clocks are mutually exclusive by construction — every day close increments exactly one and resets the other — so wandering pauses the moment any day ends with food.

The wander threshold (3 days ≈ 7.5 real seconds per day of famine at live speed) means brief oversights cost only mood; sustained neglect costs presence, temporarily, reversibly. No state in this system can delete a kitten (cozy invariant).

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Roster is append-only | Location enum, never deletion | Removing starved/departed kittens | The mechanical form of Tenet 1; also keeps names stable for the player's attachment |
| Assignment semantics | Count-based commands resolved to named individuals (LIFO) | Per-kitten assignment UI, anonymous worker pools | Anonymous pools discard identity (the game's heart); per-kitten micro-management fights the idle tenet; LIFO is deterministic and narratable |
| Arrival pity counter | Guaranteed success after 4 eligible misses | Pure probability, deterministic schedule | Pure RNG produces cruel droughts; fixed schedules feel mechanical; pity keeps surprise with bounded frustration |
| Crowding penalty | −3%/kitten past the 5th, floor 25% | Hard population cap, no penalty | Soft pressure toward housing/happiness investment without ever forbidding growth |
| Away kittens don't eat | Suspended entirely while away | Eating from neighbor's stock, half-rate | The neighbor feeds them (it's that kind of neighborhood); simplest rule that reads warmly |
| Skill decay is cozy-compatible | Gentle 0.001/tick on non-current skills only, floor 0, removable by a refinement | No decay; harsh decay | Specialization needs a cost to be a choice; the cost is a slowdown with a purchasable off-switch, and SIM-KIT-027 carves it out of the confiscation ban explicitly |

## Open Questions & Future Decisions

### Resolved
1. ✅ Whether wandered kittens can wander *while some are already away*: yes, one per famine day — but never the last Home kitten. A floor of one keeps the meadow warm, keeps absence legible ("a few may wander," per the HLD), and keeps consumption nonzero so an abandoned pantry can never masquerade as recovery.

### Deferred
1. Kitten skills/experience per job — roadmap (sim sub-HLD).
1a. Society-wave happiness obligations from the economy leaf: the +10-points-per-distinct-stocked-luxury term and the crowding-relief term crowding × (1 − limited_dr(0.048·StoryCircles + 0.075·MoonlightStages, 0.9)) enter the happiness formula when this leaf's society wave revises SIM-KIT-011.
2. Whether crowding should ease with a later structure — decide with the housing tier of the crafting segment.

## References

- Parent: `../sim-design.md` (ownership boundaries: this leaf → workers + mood; economy → yields)
- Housing capacity and catnip stock read from `../economy/` state; arrival-rate idea flag from `../curiosity/`
