# Curiosity Specs (SIM-CUR)

## Availability & Gating

- [x] **SIM-CUR-001**: While the curiosity storage cap is zero (no Cardboard Box exists — SIM-ECO's physical gate), the system shall report every idea as Locked.
- [x] **SIM-CUR-002**: While an idea's prerequisite idea is unresearched, the system shall report it as Locked, even when the curiosity cap is above zero.

## Research

- [x] **SIM-CUR-003**: When ResearchIdea is applied to an unlocked, unresearched idea with sufficient curiosity, the system shall deduct the full cost, mark the idea researched, and emit an IdeaResearched event; researching an already-researched idea shall reject with AlreadyResearched.
- [x] **SIM-CUR-004**: The system shall treat researched ideas as permanent — no mechanic shall remove, disable, or invert a researched idea's effect.

## The Tree

- [x] **SIM-CUR-005**: The system shall define exactly the nine v1 ideas with the names, curiosity costs (15, 25, 40, 60, 80, 120, 150, 200, 250), prerequisite chains, and effect targets listed in the curiosity design's Idea Tree table.
- [x] **SIM-CUR-006**: The system shall expose idea effects as pure lookups (researched-flags and derived modifiers) consumed inside the owning leaf's rules; researching an idea shall mutate nothing beyond the curiosity spend and the researched set.
