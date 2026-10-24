---
parent: sim
prefix: SIM-KIT
---

# Kittens

## Context and Design Philosophy

Kittens are the heart of the game and the reason the cozy invariants exist. This leaf owns the population lifecycle (arrival, wandering off, returning), kitten identity (names and profiles), job assignment mechanics, and the happiness/mood computation. It exposes exactly two values to the economy leaf — effective workers per job and the mood multiplier — and consumes housing capacity, catnip stock, and unlock flags.

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
    pub profile: Profile,      // drawn at arrival; what it looks like and loves (§ Profiles)
}
```

The two `_seq` stamps come from the engine's `next_seq` counter and exist to make ordering rules implementable from stored state alone: "most recently assigned" = highest `job_seq`; "first out, first back" = lowest `away_seq`. Arrival order alone cannot reconstruct either across multiple famine episodes.

### Names

**The name pool** is a content table of at least three hundred names, chosen by hand in one cozy register — foods and sweets, spices and herbs, garden plants, weather and sky, small household things, and a few storybook names (Biscuit, Mochi, Pixel, Waffles, Clementine, Captain Whiskers, …). It is sized to a grown clowder, not to the opening hours. A meadow needs 71 kittens before it can move (prestige leaf), and a meadow that keeps building houses runs well past that, so a pool of a few dozen would put most of a mature clowder's newcomers under a numeral. With three hundred, a generation's roster normally draws a fresh name for every kitten, and because each generation draws its own sample, one generation's roster — and so its page of the Family Album — reads differently from the last.

Every name in the pool:

- is different from every other name in the pool by more than one letter — one letter changed, added, or removed, ignoring case — so no two names in a roster read alike at a glance;
- is at most 16 characters, ASCII letters with single spaces between words and a capital at the start of each word, so it is single-width and short enough to stand whole in the roster's name column at the 80-column floor (render leaf);
- is not the name of any other thing in the game, ignoring case: not a resource (as the stores spell it, or that spelling without its final "s"), and not a structure, job, idea, craft, refinement, house rule, rite, arch, tradition, dream loom, colony, site, adventure, season, or kitten trait. Events name kittens in sentences (`{name} became a Gardener`), and a kitten that shared a store's or a building's name would make such a line read as being about that thing.

Names are drawn without replacement via RNG; when the pool exhausts, reuse begins with Roman-numeral suffixes (Biscuit II) — the clowder can always outgrow the pool. The remaining pool is *derived from the roster*, never stored: the current cycle is the smallest suffix count at which some base name is unused, and the draw is a uniform RNG index over the remaining names in content-table order, each cycle drawn without replacement in turn.

A name is stored on its kitten, and the pool is only where new names come from. So the pool can grow or change between versions without renaming anyone: a saved kitten keeps its name whether or not the current pool holds it, and every name already in the roster, at home or away, counts as taken for every later draw, compared ignoring case, so no draw can repeat it.

## Profiles

Every kitten has a **profile**: what it looks like and what it loves. A profile is there for delight and does nothing else. No rule in the simulation reads it — no production, mood, arrival, assignment, wandering, event, or random draw depends on any part of it — so a kitten is never better or worse at anything for its profile, and a profile gives the player nothing to chase, compare, or hope for. The facts about a kitten that do bear on play are the Kitten Facts below, which the shell shows beside the profile.

```rust
pub struct Profile {
    pub fur: Fur,         // a pattern and the colours its slots took, in slot order
    pub eyes: EyeLook,    // one entry of the eye-look table
    pub favourites: Vec<Favourite>, // four to six, each of a different kind, in kind order
    pub pose: Pose,       // how its portrait draws it: sitting up, napping, or a loaf
}

pub struct Fur {
    pub pattern: FurPattern,
    pub colours: Vec<FurColour>, // one per slot of the pattern
}

pub struct Favourite {
    pub kind: FavouriteKind, // which kind of favourite: a food, a secret talent, …
    pub entry: String,       // what it is, as the kind's table words it
}
```

"Fur" is this leaf's word for a kitten's colouring. It is not the theme leaf's "coat", which is the colours a resource's name wears; the two never meet.

**Fur.** The fur-pattern table lists every pattern a kitten can wear. Each pattern has one to three **slots**, and each slot lists the fur colours it may take, so every fur a kitten draws is one a real cat could have: a pattern's white patches take only white, a tortoiseshell's two colours come from the warm and the dark families, and a colourpoint's body is pale while its points are dark. Each pattern carries its own words for a kitten wearing it, with each slot's colour name filled in where the words mark it (`{0} tabby`). Those words are the fur's first channel, read by anyone, and the colours the shell paints are its second (theme leaf: Fur). Patterns and fur colours are typed ids. Their names and slots are content in this leaf, and the colour each is painted in belongs to the theme leaf, which keeps every colour decision.

**Eyes.** The eye-look table lists every look a kitten's eyes can have. Each look names the colour of the left eye and of the right, and carries its words (`green eyes`). A look whose two eyes differ is an ordinary entry, so it is exactly as rare as its share of the table.

**Favourites.** A kitten's favourites are the little things that make it itself, and what makes one kitten unlike another is as much *which* things it is known for as what they are: one is known for its favourite food and the thing it is secretly afraid of, another for its signature move and the smell it loves. So favourites come in **kinds**, and each kitten has its own handful of kinds. The favourite-kind table lists at least twelve kinds. Each kind has a label, the words that introduce it on the profile (`Favourite food`, `Secret talent`), and a table of at least sixteen entries. A label is printable ASCII of at most 20 characters; an entry is a short phrase written to follow its label: printable ASCII, at most 40 characters, distinct in its table ignoring case. Kinds are typed ids, append-only like every content id. No label or entry names anything in the game: not as a whole word, nor as a run of whole words, ignoring case, does it contain the name of a resource (as the stores spell it, or that spelling with its last word made singular), structure, job, idea, craft, refinement, house rule, rite, arch, tradition, dream loom, colony, site, adventure, season, or kitten trait — the vocabulary the name pool is also kept apart from. A favourite that named something in the game would read as a reason to make or buy it, and a profile gives no such reason (Tenet 3).

### Drawing a profile

A kitten's profile is drawn once, when it arrives, and stored on the kitten. It takes **no draw from the state RNG**. It is a fixed function of two things the save holds: the kitten's name, and the generation it arrives in. The two are hashed with 64-bit FNV-1a, over the name's UTF-8 bytes followed by the generation as four little-endian bytes. The hash seeds a PCG-32 — seeded as the state RNG is seeded from a new game's seed — that is local to the draw and never stored, and that generator draws, in this order:

1. the pattern;
2. each slot's colour, in slot order, drawn over only those of the slot's colours that no earlier slot took, so two slots never share a colour (a slot left with no colour to take is impossible in the table, and a content test holds it so);
3. the eye look;
4. how many favourites it has: four, five, or six;
5. that many kinds, each drawn over the kinds not yet drawn, so no kind is drawn twice; the kinds are then held in kind-table order, the order the profile states them in;
6. an entry for each of those kinds, in kind-table order, from the kind's table;
7. the pose.

Each draw is uniform over its table, in content-table order. The fur and the eyes come first, so a change to how favourites or poses are drawn leaves every kitten's fur and eyes as they were.

**Poses.** The pose is how the kitten's portrait draws it — sitting up, napping, or lying in a loaf (render leaf: The portrait) — so a page of kittens reads as a room of cats doing their own things rather than a row of identical pictures. It is a profile field like any other: drawn once, kept for life, read by no rule of the simulation.

Taking nothing from the state RNG keeps its fixed draw schedule (SIM-ENG-018) exactly as it was, so every random outcome in a meadow is the one it would have been without profiles. Keying the draw on what the save holds means a profile can be drawn again from any save, which is how a save written before profiles existed gives its kittens theirs (persistence leaf). Names are unique within a generation (§ Names), so no two kittens drawn in one generation share a seed, and namesakes in different generations draw different profiles.

### Kept for life

A profile is stored, never re-derived: what a kitten looks like and what it loves is part of who it is, kept for the same reason its name is. A favourite is kept as its words, so the favourite tables can grow, shrink, or be reworded between versions without changing any kitten. A pattern, a fur colour, an eye look, a favourite's kind, and a pose are kept as ids, append-only like every content id, so their tables can grow without changing any kitten either; rewording a colour's name changes how a fur is said, never what it is. A profile never changes once drawn — not when the kitten wanders off or comes home, not when it is promoted or made the Elder. A kitten carried in a Traveling Basket brings its profile into the new meadow, and the Family Album's memorial of every kitten keeps the profile it had (prestige leaf), so the album remembers what each kitten looked like and loved.

One save format predates kinds and poses: format v8 held six favourites of the same six kinds on every kitten, and no pose. A profile from that format keeps its fur and eyes on load and has its favourites and pose drawn as an arrival draws them (persistence leaf), so a clowder from that format looks as it did and gains the variety every later kitten has.

## Kitten Facts

A few things the simulation holds about a kitten bear on play and are stated on no other surface. The shell states them beside the profile, read from the core's facts query for that kitten, which works each one out from the kitten and the content tables and holds nothing of its own:

- **Temperament.** The kitten's trait by its name, with the trait's blurb: what the trait does while that kitten is the Elder, in the same terms as the trait list (§ The Elder). It is what the player needs in order to choose an Elder at all.
- **Rank.** The kitten's rank (§ Skills, Ranks, and the Elder).
- **Best at.** The job in which the kitten holds the most skill XP, where it holds any above zero, ties going to the earliest job in job order, together with the yield bonus that skill's tier gives the kitten in that job (SIM-KIT-020) — zero below the first tier. A kitten with no skill above zero is best at nothing yet, and the query says so.
- **Since.** The day the kitten arrived, with that day's season and year. A kitten whose arrival day is 0 is one a Move carried along from the old meadow (SIM-MOVE-002 sets a carried kitten's arrival day to 0), since no arrival check runs before the first day close, and the query reports it as carried along rather than as arriving on day 0.

## Arrival

Checked once per day (engine day boundary), when there is at least one free housing slot **and** catnip stock ≥ 50:

- Base chance 15%, +5% per full 100 catnip in stock, capped at 60%; one idea then adds +15% *on top of the cap* (effective maximum 75% — the idea's bonus is always real).
- **Pity counter:** after 4 consecutive failed eligible checks, the next eligible check succeeds automatically. Ineligible days (no free slot, or stock < 50) *pause* the counter — they neither extend nor reset it. Dry streaks are frustration, not drama.
- On success: a kitten joins (Home, loafing), gets a name, a trait, and a profile (§ Profiles), resets the pity counter, and emits `KittenArrived`.

At most one kitten arrives per day. Housing capacity = sum of structure housing effects (economy leaf). **Every rostered kitten holds a slot, Home or Away** — a wandered-off kitten's cushion is kept warm for its return, so a return can never find the house full and an arrival can never displace an absentee. Free slots = capacity − roster size.

## Jobs

Assignment is by count, targeted at individuals: `AssignJob(job)` puts the **most recently arrived** idle Home kitten into the job (newest kittens are keenest) and stamps its `job_seq`; `UnassignJob(job)` returns the **most recently assigned Home** worker (highest `job_seq` among Home holders) to loafing. If the job's only holders are Away, `UnassignJob` returns `NoWorkersAssigned` — they aren't here to reassign. Deterministic selection, and events name the kitten (`{name} became a Gardener`) — identity makes assignment delightful rather than bureaucratic.

Effective workers per job = count of Home kittens holding that job. Away kittens keep their job title but don't count (they're not here).


## Skills, Ranks, and the Elder

**Skills.** A working Home kitten gains 0.01 skill XP/tick in its current job × (1 + skill-XP effects), mirrored into its lifetime `exp` pool; every *other* job's skill decays 0.001/tick (floor 0) — switching jobs means retraining, specialization is rewarded (a refinement later removes the decay). Skill tiers multiply that kitten's own yield: <100 +0%, ≥100 +1.25%, ≥500 +2.5%, ≥1200 +4.5%, ≥2500 +7.5%, ≥5000 +12.5%, ≥9000 +18.75%. Production therefore resolves per kitten, in roster order (determinism).

**Ranks.** `Promote(kitten_index)` spends 500 × 1.75^rank exp (from that kitten's pool) plus 25 × (rank+1) shinies, incrementing `rank`. Rank scales the Elder's leadership and a Tinkerer's ability to work high-tier recipes without penalty (each tier above rank: −15% effectiveness, floor 25%).

**The Elder.** `AppointElder(kitten_index)` names one kitten (replaceable anytime, never removable by the game). The Elder's own-job yield ×(rank+1)/1.4 (min ×1.0), and its birth trait activates clowder-wide: Crafty +5% craft ratio; Brassy +10% metal crafts (badges/brass/cogs/moonsteel); Alchemical +7.5% process crafts (pavers/moonmilk/dreamweave/cometsilk); Charming +3% trade ratio; Bold +0.5 pounce ratio; Curious −5% idea curiosity costs; Serene −9% rite costs; None — nothing (some cats simply are). Traits are rolled uniformly at arrival (one RNG draw after the name draw, appended to the fixed schedule).

Each trait has a name the player reads it by and a blurb stating its effect plainly. The name is the trait's own (Crafty, Brassy, …), save that None is read as **Easygoing**, since "None" read as a temperament says nothing about a cat. The blurb begins `As Elder: ` and states the trait's effect as the refinement blurbs state theirs (`As Elder: every craft +5%.`); a blurb that names a resource marks it as the curiosity leaf's blurbs do (SIM-CUR-014). Easygoing's blurb says that its Elder changes nothing, because some cats simply are. A content test holds the figure each blurb states equal to the figure its trait's effect applies, so the two cannot drift apart.

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
| Name pool | At least three hundred names chosen by hand, Roman numerals past them | A few dozen names; names built from parts (two halves compounded, or a given name and a surname); a syllable generator | A mature generation runs well past the 71 kittens a Move needs, so a few dozen names put most newcomers under a numeral. Names built from parts multiply cheaply but share their parts, so a large roster becomes rows of look-alikes that differ by a syllable, which is the opposite of what a name is for; a chosen name is distinct at a glance. Numerals stay as the tail, so the pool never caps the clowder |
| Pool names and other names | No pool name is the name of any other thing, ignoring case | Allow overlaps | An event line naming a kitten that shares a store's, a building's, or a trait's name reads as being about that thing (Tenet 3) |
| A pool that changes between versions | Saved names never change; every rostered name counts as taken | Rename saved kittens from the current pool; reset the draw | A kitten's name is what the player is attached to, and the roster is append-only for that reason; renaming one would be a quiet loss (Tenet 1) |
| Assignment semantics | Count-based commands resolved to named individuals (LIFO) | Per-kitten assignment UI, anonymous worker pools | Anonymous pools discard identity (the game's heart); per-kitten micro-management fights the idle tenet; LIFO is deterministic and narratable |
| Arrival pity counter | Guaranteed success after 4 eligible misses | Pure probability, deterministic schedule | Pure RNG produces cruel droughts; fixed schedules feel mechanical; pity keeps surprise with bounded frustration |
| Crowding penalty | −3%/kitten past the 5th, floor 25% | Hard population cap, no penalty | Soft pressure toward housing/happiness investment without ever forbidding growth |
| Away kittens don't eat | Suspended entirely while away | Eating from neighbor's stock, half-rate | The neighbor feeds them (it's that kind of neighborhood); simplest rule that reads warmly |
| Skill decay is cozy-compatible | Gentle 0.001/tick on non-current skills only, floor 0, removable by a refinement | No decay; harsh decay | Specialization needs a cost to be a choice; the cost is a slowdown with a purchasable off-switch, and SIM-KIT-027 carves it out of the confiscation ban explicitly |
| What a profile does in play | Nothing: no rule reads it, and the facts that bear on play are stated beside it | A few favourites carrying small effects (a stocked favourite food lifting that kitten's mood, a favourite job learned faster) | An effect on a profile makes it something to compare and hope for, and assignment is count-based and newest-first, so a kitten's favourite job would be luck the player cannot steer. A profile that only delights asks nothing of the player (Tenet 2), and the facts beside it give the clarity (Tenet 3) |
| Where a profile's randomness comes from | A local generator seeded by a hash of the kitten's name and generation; no draw from the state RNG | Draws appended to the state RNG's fixed schedule; a profile derived anew on every read | A state-RNG draw shifts every random outcome after it, and a save written before profiles has no draws to replay for its kittens, so migration would need a second mechanism. Deriving on every read lets a table edit between versions change a kitten that already exists. Hashing what the save holds gives one draw for arrival and migration alike, and storing its result keeps it fixed |
| The hash | 64-bit FNV-1a over the name's bytes and the generation's | The standard library's default hasher | The standard hasher's output may change between Rust releases, and a profile drawn for migration has to match one drawn at arrival on any build |
| How a favourite reads | A phrase from a table, kept clear of every name in the game | Phrases free to name game things ("loves catnip") | A favourite naming a store or a structure reads as a reason to make or buy it, and no such reason exists (Tenet 3) |
| How fur is chosen | Patterns with slots, each slot drawing from the colours a real cat could have there | Any colours in any combination | Free combinations draw kittens no cat looks like, and a profile is most charming when the player can picture the cat |
| The trait None, read by the player | Easygoing | None | "Temperament: None" says nothing about a cat, and the trait list already says some cats simply are |
| Which favourites a kitten has | Four to six kinds, drawn from a table of kinds | The same fixed set of kinds on every kitten; every kind on every kitten | The same kinds on every kitten makes every profile read as one form filled in many times; what a kitten is known for is part of its character. Every kind on every kitten is a profile too long to read at a glance or to fit a short pane |
| How a portrait draws a kitten | One of three poses, drawn once and kept | A single pose for every kitten; a face alone | A face filled solid with fur and two small eyes reads as a mask rather than a cat. One pose for all makes a roster of identical pictures; a few poses make each kitten look like it is getting on with its day |
| Profiles saved before kinds and poses (format v8) | Favourites and pose drawn on load, fur and eyes kept | Kept as they were, given a pose alone | Kept as they were, every kitten of such a save would carry the same six kinds — the sameness the kinds exist to end — for as long as that clowder lives, while the kittens arriving after it vary. Keeping the fur and eyes keeps the part the player knows a kitten by at a glance exactly as it was |

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
- Profiles: painted by `../../tui/theme/` (Fur), kept in the Family Album by `../prestige/`, drawn for kittens in saves from before profiles by `../../persistence/`; the blurb marks are the curiosity leaf's (SIM-CUR-014)
