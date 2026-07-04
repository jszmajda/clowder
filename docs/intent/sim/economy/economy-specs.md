# Economy Specs (SIM-ECO)

## Resources & Caps

- [ ] **SIM-ECO-001**: The system shall track catnip, twigs, fluff, and curiosity as f64 stocks with base caps of 200, 50, 25, and 0 respectively.
- [ ] **SIM-ECO-002**: The system shall extend caps per owned structure: +250 catnip cap per Pantry, +40 twig cap per Twig Pile, +50 curiosity cap per Cardboard Box, +10 fluff cap per Cat Tree.
- [ ] **SIM-ECO-003**: During the clamp phase, the system shall trim each stock to its cap; production may exceed caps transiently within a tick, and consumption may draw from that transient before clamping.
- [ ] **SIM-ECO-022**: The system shall use epsilon-tolerant affordability checks (stock + 1e-9 ≥ cost) for all spends, and deductions shall floor the stock at zero.

## Manual Actions

- [ ] **SIM-ECO-004**: When the gather-catnip action is applied, the system shall add 1 catnip — 2 where the gather-doubling idea (SIM-CUR) is researched.
- [ ] **SIM-ECO-005**: When the forage action is applied with at least 10 catnip in stock, the system shall deduct 10 catnip and add 1 twig.
- [ ] **SIM-ECO-006**: If a manual action's yield resource is at its cap, then the system shall reject the action with StorageFull and change nothing.

## Structures

- [ ] **SIM-ECO-007**: The system shall price the next copy of a structure at ceil(base × ratio^owned) per cost component, and when Build is applied with sufficient stock, deduct the full cost, increment the owned count, and emit a StructureBuilt event.
- [ ] **SIM-ECO-008**: The system shall define the Catnip Patch: base cost 10 catnip, ratio 1.12, producing 0.125 catnip/tick each, multiplied by the season modifier; available from game start.
- [ ] **SIM-ECO-009**: The system shall define the Cozy Cushion: base cost 25 catnip + 5 twigs, ratio 1.15, housing 1 kitten; available from game start.
- [ ] **SIM-ECO-010**: The system shall define the Cardboard Box: base cost 8 twigs, ratio 1.15, adding 50 curiosity cap and producing 0.005 curiosity/tick each; available from game start.
- [ ] **SIM-ECO-011**: The system shall define the Pantry: base cost 20 twigs, ratio 1.15, adding 250 catnip cap; buildable only after its unlocking idea (SIM-CUR) is researched.
- [ ] **SIM-ECO-012**: The system shall define the Twig Pile: base cost 50 catnip + 10 twigs, ratio 1.15, adding 40 twig cap; buildable only after its unlocking idea (SIM-CUR) is researched.
- [ ] **SIM-ECO-013**: The system shall define the Scratching Post: base cost 15 twigs + 5 fluff, ratio 1.15, adding 5 happiness percentage points each (consumed by SIM-KIT); buildable only after its unlocking idea (SIM-CUR) is researched.
- [ ] **SIM-ECO-014**: The system shall define the Cat Tree: base cost 30 twigs + 10 fluff, ratio 1.15, housing 3 kittens and adding 10 fluff cap; buildable only after its unlocking idea (SIM-CUR) is researched.
- [ ] **SIM-ECO-024**: The system shall keep each structure's first 20 copies reachable: every cost component of copies 1–20 shall fit within the maximum cap achievable with cap-extending structures purchasable alongside them. (Beyond that horizon, exponential costs outrunning linear caps is an intentional late-game plateau — a slowdown, not an error.)

## Job Yields

Job yields are per effective worker (SIM-KIT: Home kittens holding the job) per tick, multiplied by the mood multiplier (SIM-KIT) and any active zoomies multiplier (SIM-ENG).

- [ ] **SIM-ECO-015**: The system shall define the Gardener job yielding 0.15 catnip/tick per effective worker, season-independent; available once any kitten exists.
- [ ] **SIM-ECO-016**: The system shall define the Twig Collector job yielding 0.03 twigs/tick per effective worker; available once any kitten exists.
- [ ] **SIM-ECO-017**: The system shall define the Groomer job yielding 0.02 fluff/tick per effective worker; assignable only after its unlocking idea (SIM-CUR) is researched.
- [ ] **SIM-ECO-018**: The system shall define the Observer job yielding 0.025 curiosity/tick per effective worker; assignable only while at least one Cardboard Box is owned.

## Seasons

- [ ] **SIM-ECO-019**: The system shall multiply Catnip Patch production (only — no other production is seasonal) by the season modifier: Spring ×1.5, Summer ×1.0, Autumn ×0.85, Winter ×0.25, with Winter eased to ×0.40 where the winter-easing idea (SIM-CUR) is researched.

## Consumption

- [ ] **SIM-ECO-020**: While kittens are at Home, the system shall deduct 0.085 catnip/tick per Home kitten as one pooled sum during the consumption phase, unmodified by mood; kittens at the neighbor's yard shall consume nothing.
- [ ] **SIM-ECO-021**: If pooled demand exceeds the available catnip stock, then the system shall set the stock to zero and mark the tick as a shortfall; a stock exactly covering demand is not a shortfall.

## Multiplier Scope

- [ ] **SIM-ECO-023**: The system shall apply mood and zoomies multipliers to job production only; structure production shall never be mood- or zoomies-modified.
