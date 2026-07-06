# Curiosity Specs (SIM-CUR)

## Availability & Gating

- [x] **SIM-CUR-001**: While the curiosity storage cap is zero (no Cardboard Box exists — SIM-ECO's physical gate), the system shall report every idea as Locked.
- [x] **SIM-CUR-002**: While an idea's prerequisite idea is unresearched, the system shall report it as Locked, even when the curiosity cap is above zero.

## Research

- [x] **SIM-CUR-003**: When ResearchIdea is applied to an unlocked, unresearched idea with sufficient curiosity, the system shall deduct the full cost, mark the idea researched, and emit an IdeaResearched event; researching an already-researched idea shall reject with AlreadyResearched.
- [x] **SIM-CUR-004**: The system shall treat researched ideas as permanent — no mechanic shall remove, disable, or invert a researched idea's effect.

## The Tree

- [x] **SIM-CUR-005**: The system shall define exactly the ideas of the curiosity design's banded Idea Tree tables (59 ideas, ids append-only, the founding nine unchanged), with the tabled costs, gate-craft components, prerequisites, and unlock targets.
- [x] **SIM-CUR-006**: The system shall expose idea effects as pure lookups (researched-flags and derived modifiers) consumed inside the owning leaf's rules; researching an idea shall mutate nothing beyond the curiosity spend and the researched set.

## Depth: The Banded Tree

- [x] **SIM-CUR-007**: When an idea's cost includes a gate craft (stories, anthologies, schemes, starshards, echoes), ResearchIdea shall spend curiosity and the gate components together, all-or-nothing, under the same epsilon-tolerant affordability rule.
- [x] **SIM-CUR-008**: The idea prerequisite graph shall be acyclic with every idea reachable from the roots (property-tested), and an idea listing multiple prerequisites shall require all of them.
- [x] **SIM-CUR-009**: The system shall expose idea-granted unlocks (structures, crafts, manual actions, jobs, and subsystem surfaces) as pure lookups consumed by the owning leaves, exactly as the tree tables assign them.

## Depth: Refinements & House Rules

- [x] **SIM-CUR-010**: The system shall define exactly the refinements of the curiosity design's Refinements table (35, append-only ids), purchasable once each via BuyRefinement with curiosity + material costs spent all-or-nothing, gated by their ideas (Locked before), contributing their tabled effects permanently.
- [x] **SIM-CUR-011**: The system shall define the four House Rule pairs per the design table; AdoptRule spends charm all-or-nothing, permanently blocks the sibling rule for the current meadow (Locked), and both adoption and blocking survive save/load.
- [x] **SIM-CUR-012**: Refinement and house-rule effects shall flow through the typed effect-key system like every other source, in the engine's fixed source order.
