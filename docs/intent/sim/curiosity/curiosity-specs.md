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
- [x] **SIM-CUR-008**: The idea prerequisite graph shall be acyclic with every idea reachable from the roots (property-tested), and an idea listing multiple prerequisites shall require all of them. Mystic gate-craft costs (starshards, echoes) escalate steeply against their faucet rates by design; their absolute reachability is a balance concern verified at the integration wave, not a per-idea property.
- [x] **SIM-CUR-009**: The system shall expose idea-granted unlocks (structures, crafts, manual actions, jobs, and subsystem surfaces) as pure lookups consumed by the owning leaves, exactly as the tree tables assign them.

## Depth: Refinements & House Rules

- [x] **SIM-CUR-010**: The system shall define exactly the refinements of the curiosity design's Refinements table (35, append-only ids), purchasable once each via BuyRefinement with curiosity + material costs spent all-or-nothing, gated by their ideas (Locked before), contributing their tabled effects permanently.
- [x] **SIM-CUR-011**: The system shall define the four House Rule pairs per the design table; AdoptRule spends charm all-or-nothing, permanently blocks the sibling rule for the current meadow (Locked), and both adoption and blocking survive save/load.
- [x] **SIM-CUR-012**: Refinement and house-rule effects shall flow through the typed effect-key system like every other source, in the engine's fixed source order.

## Blurbs

- [x] **SIM-CUR-013**: The system shall mark in every idea's, refinement's, and house rule's blurb each mention of a resource — the resource's name as the stores spell it, or that spelling with its last word made singular (a final "s" dropped, or a final "ies" read as "y"), as whole words in any case, a craft's name (which is the name of the resource it makes) included — as `{text|Id}` or `{Id}`, Id being that resource's identifier, leaving unmarked only a mention that lies within the name of a structure, job, idea, refinement, house rule, rite, arch, tradition, dream loom, colony, site, or adventure written in the tables' own capitalisation (or that name made plural: a final "s" added, or a final "y" read as "ies"), or a mention whose first word begins with a capital letter and stands within a sentence — neither at the blurb's start nor directly after a full stop, question mark, or exclamation mark and a space.
- [x] **SIM-CUR-014**: The system shall read each blurb both as a sequence of plain text and resource references and as plain words, a mark running from a `{` to the next `}`: `{text|Id}` a reference to the resource whose identifier is exactly Id, read as text (the words before the first `|`), and `{Id}` a reference read as Id character for character; a mark naming no resource identifier reading as plain text, its text without the braces or the identifier, and a `{` with no `}` after it, or a `}` outside a mark, reading as itself.
- [x] **SIM-CUR-015**: The system shall hold every brace in the content tables' blurbs to a mark naming a resource identifier, each mark's text being a mention of that resource as SIM-CUR-013 spells one, so no blurb carries a mark that names no resource, names a resource its words do not spell, or a brace that opens or closes no mark.
