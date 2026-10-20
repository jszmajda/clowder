# Economy Specs (SIM-ECO)

## Resources & Caps

- [x] **SIM-ECO-001**: The system shall track exactly the resources of the economy design's RES table (46 ids, append-only) as f64 stocks, with each resource's base cap and tier as tabled; uncapped (∞) resources are never clamped.
- [x] **SIM-ECO-002**: The system shall extend storage caps by the cap effects of owned structures exactly as the economy design's BLD table specifies (including multi-resource extenders like the Shed and Porch).
- [x] **SIM-ECO-003**: During the clamp phase, the system shall trim each stock to its cap; production may exceed caps transiently within a tick, and consumption may draw from that transient before clamping.
- [x] **SIM-ECO-022**: The system shall use epsilon-tolerant affordability checks (stock + 1e-9 ≥ cost) for all spends, and deductions shall floor the stock at zero.

## Manual Actions

- [x] **SIM-ECO-004**: When the gather-catnip action is applied, the system shall add 1 catnip — 2 where the gather-doubling idea (SIM-CUR) is researched.
- [x] **SIM-ECO-005**: When the forage action is applied with at least 10 catnip in stock, the system shall deduct 10 catnip and add 1 twig.
- [x] **SIM-ECO-006**: If a manual action's yield resource is at its cap, then the system shall reject the action with StorageFull and change nothing.

## Structures

- [x] **SIM-ECO-007**: The system shall price the next copy of a structure at ceil(base × ratio^owned) per cost component, and when Build is applied with sufficient stock, deduct the full cost, increment the owned count, and emit a StructureBuilt event.
- [x] **SIM-ECO-008**: The system shall define the Catnip Patch: base cost 10 catnip, ratio 1.12, producing 0.125 catnip/tick each, multiplied by the season modifier; available from game start.
- [x] **SIM-ECO-009**: The system shall define the Cozy Cushion: base cost 25 catnip + 5 twigs, ratio 1.15, housing 1 kitten; available from game start.
- [x] **SIM-ECO-010**: The system shall define the Cardboard Box: base cost 8 twigs, ratio 1.15, adding 50 curiosity cap and producing 0.005 curiosity/tick each; available from game start.
- [x] **SIM-ECO-011**: The system shall define the Pantry: base cost 20 twigs, ratio 1.15, adding 250 catnip cap; buildable only after its unlocking idea (SIM-CUR) is researched.
- [x] **SIM-ECO-012**: The system shall define the Twig Pile: base cost 50 catnip + 10 twigs, ratio 1.15, adding 40 twig cap; buildable only after its unlocking idea (SIM-CUR) is researched.
- [x] **SIM-ECO-013**: The system shall define the Scratching Post: base cost 15 twigs + 5 fluff, ratio 1.15, adding 5 happiness percentage points each (consumed by SIM-KIT); buildable only after its unlocking idea (SIM-CUR) is researched.
- [x] **SIM-ECO-014**: The system shall define the Cat Tree: base cost 30 twigs + 10 fluff, ratio 1.15, housing 3 kittens and adding 10 fluff cap; buildable only after its unlocking idea (SIM-CUR) is researched.
- [x] **SIM-ECO-024**: The system shall keep each structure reachable over its banded horizon — bands 0–2: first 20 copies; band 3 and declared walls (Tea House, Treat Bakery): first 5 copies; band 4 and declared monuments (Yarn Computer): first copy — where every *capped-resource* cost component fits within the maximum cap achievable with cap-extending structures at ≤ 20 copies each plus the Porch kite ratchet (uncapped components, hushfelt, and the site-gated cosmic outposts 40–52 exempt — the latter are gated by the cosmic reach-ladder, not by storage). Beyond each horizon, exponential costs outrunning linear caps is an intentional plateau.

## Job Yields

Job yields are per effective worker (SIM-KIT: Home kittens holding the job) per tick, multiplied by the mood multiplier (SIM-KIT) and any active zoomies multiplier (SIM-ENG).

- [x] **SIM-ECO-015**: The system shall define the Gardener job yielding 0.15 catnip/tick per effective worker, season-independent; available once any kitten exists.
- [x] **SIM-ECO-016**: The system shall define the Twig Collector job yielding 0.03 twigs/tick per effective worker; available once any kitten exists.
- [x] **SIM-ECO-017**: The system shall define the Groomer job yielding 0.02 fluff/tick per effective worker; assignable only after its unlocking idea (SIM-CUR) is researched.
- [x] **SIM-ECO-018**: The system shall define the Observer job yielding 0.025 curiosity/tick per effective worker; assignable only while at least one Cardboard Box is owned.

## Seasons

- [x] **SIM-ECO-019**: The system shall multiply Catnip Patch production (only — no other production is seasonal) by the season modifier: Spring ×1.5, Summer ×1.0, Autumn ×0.85, Winter ×0.25, with Winter eased to ×0.40 where the winter-easing idea (SIM-CUR) is researched.

## Consumption

- [x] **SIM-ECO-020**: While kittens are at Home, the system shall deduct 0.085 catnip/tick per Home kitten as one pooled sum during the consumption phase, unmodified by mood; kittens at the neighbor's yard shall consume nothing.
- [x] **SIM-ECO-021**: If pooled demand exceeds the available catnip stock, then the system shall set the stock to zero and mark the tick as a shortfall; a stock exactly covering demand is not a shortfall.

## Multiplier Scope

- [x] **SIM-ECO-023**: The system shall apply mood and zoomies multipliers to job production only; structure production shall never be mood- or zoomies-modified.

## Depth: Resources & Tiers (RES facet)

- [x] **SIM-ECO-025**: The system shall assign every resource the tier from the RES table, and transient, luxury, mystic, and dreamtime resources shall skip production-stack layers 6–8 (global production, Elder Wisdom, the Long Sun) entirely.
- [x] **SIM-ECO-026**: While kittens are Home, the system shall drain stocked luxuries per Home kitten per tick (feathers 0.001, bells 0.0007, mint 0.0001, scaled by max(0, 1 + luxury-demand effects — Gift Stands −0.04 each, an active jamboree +0.2)), flooring at zero; an empty luxury never triggers shortfall or Grumpiness — its happiness bonus simply lapses.
- [x] **SIM-ECO-027**: When the Rummage action is applied after its granting idea is researched, the system shall add 0.5 pebbles, subject to the StorageFull rule; before that idea it shall reject with Locked.

## Depth: Structures (BLD facet)

- [x] **SIM-ECO-028**: The system shall define exactly the structures of the economy design's BLD table (53 structures, append-only) — ids, costs, ratios, effects, and gates — with no staged rebuilds; a structure's gate is Always, an Idea, or a cosmic Site (the outposts 40–52 are gated on their site being reached, not an idea, and are Locked until then). Cross-leaf structures carry costs/ramps here and effects in their owning leaves.
- [x] **SIM-ECO-029**: The system shall run every converter structure (Hearth, Kiln, Treat Bakery, Prism Spinner, Dynamo Wheel fuel draw) through the engine's throttle: all inputs and outputs scale by the worst-supplied input fraction each tick.
- [x] **SIM-ECO-030**: The system shall scale Treat Bakery output by the pep-cap ratio mpr = pep_cap × 0.007 / 100 (feathers ×1.25·mpr, bells ×0.3·mpr per active bakery per tick) — storage feeding production.
- [x] **SIM-ECO-031**: The system shall multiply all Porch cap extensions by 1 + limited_dr(0.01 × the highest kite stock ever held, 2.25) — a persisted high-water ratchet, so spending kites never shrinks storage.
- [x] **SIM-ECO-032**: The system shall contribute global production (layer 6) as +2% per active Dynamo Wheel × (1 + 0.15 × Cogworks owned) plus +5% per active Star Basin, applied to material-tier flows only.

## Depth: Crafting (CRAFT facet)

- [x] **SIM-ECO-033**: When Craft(id) is applied for an unlocked recipe with sufficient inputs, the system shall spend the recipe's inputs (all-or-nothing, epsilon-tolerant) and add 1 × (1 + craft-ratio effects) of the output; recipes are Locked until their granting idea is researched.
- [x] **SIM-ECO-034**: The system shall define exactly the recipes of the economy design's CRAFT table, and every recipe's inputs shall be producible no later than the idea band in which the recipe unlocks (no dead recipes — property-tested), excepting the expedition/colony faucets (feathers, bells, mint, shinies, unicorns) owed by later waves.

## Depth: Energy (NRG facet)

- [x] **SIM-ECO-035**: The system shall sum energy production and consumption over active structures per the BLD table — Sunning Decks seasonal (winter ×0.75, summer ×1.25), Yarn Computers consuming 2n + n²/2 total at n copies — and expose both as derived queries.
- [x] **SIM-ECO-036**: While energy consumption exceeds production, the system shall multiply the contributions of production-stack layers 6–8 by clamp(production/consumption, 0.25, 1.0) — a brownout, never a blackout; while production exceeds consumption, effects that name the surplus (Moonlight Stage charm) scale by min(production/max(consumption, 1), 1.75).

## Depth: Mess (MESS facet)

- [x] **SIM-ECO-037**: The system shall accrue mess per tick from dirty structures per the BLD table, minus tidy effects, minus dissipation of 1e-4 × current mess per tick; mess level = floor(log10(mess/1e4)) when mess ≥ 1e4.
- [x] **SIM-ECO-038**: While mess level is 0 or higher, the system shall multiply catnip growth by 0.8 per level step (floor ×0.25); at level 1+, arrival chance ×0.75; at level 2+, −5 happiness points per level above 1 — slowdowns only, never loss or confiscation.
- [x] **SIM-ECO-039**: Where Cogworks exist at year close, the system shall auto-craft 2% per Cogworks (max 50%) of each raw resource standing at ≥98% of cap into its single-input crafted form, once per year, unjammed (twigs→wands, pebbles→tiles, buttons→badges, dew→moonmilk, stardust→cometsilk; other raws skipped) — overflow becomes goods, not waste.
- [x] **SIM-ECO-040**: The system shall define the Rummager job yielding 0.02 pebbles/tick per effective worker, assignable after its granting idea (SIM-CUR) is researched.
- [x] **SIM-ECO-041**: The system shall define the Pouncer job yielding 0.06 pep/tick per effective worker, assignable after the expedition-surface idea (SIM-CUR) is researched.
- [x] **SIM-ECO-042**: The system shall define the Sunwarden job yielding 0.0015 warmth/tick per effective worker, assignable after the ceremonies-surface idea (SIM-CUR) is researched.
- [x] **SIM-ECO-043**: AssignTinkerer(recipe) shall bind the most recently arrived unbound Home Tinkerer to an unlocked recipe (Locked if the recipe isn't; NoIdleKittens if no unbound Home Tinkerer exists; NoSeats if bindings, an Away Tinkerer's included, would exceed the meadow's tinkerer seats (SIM-ECO-045)); UnassignTinkerer(recipe) unbinds the highest-job_seq bound holder (NoWorkersAssigned if none). Each bound Home Tinkerer crafts its recipe at 0.0017 batches/tick × (1 + tinkerer-speed effects) × its rank-tier effectiveness (SIM-KIT-026), drawing inputs through the engine throttle for craft-ratio-boosted output; an Away Tinkerer's recipe idles and its binding persists.
- [x] **SIM-ECO-044**: The system shall extend the pep cap by housing: +25 per Cozy Cushion, +25 per Window Perch, +75 per Cat Tree, per the BLD table.
- [x] **SIM-ECO-045**: The system shall give every structure definition the number of tinkerer seats each copy gives — one for the Toy Workshop and none for any other structure — and shall take the meadow's tinkerer seats as the sum, over every structure, of the copies held times the seats each copy gives.
- [x] **SIM-ECO-046**: The system shall report the command gate (SIM-ENG-045) of AssignTinkerer and of UnassignTinkerer open, for every recipe, exactly while the Tinkerer job is unlocked.
