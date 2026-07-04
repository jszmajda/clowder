---
parent: sim
prefix: SIM-CUR
---

# Curiosity

## Context and Design Philosophy

Curiosity is the game's research system: kittens investigate the world and their accumulated curiosity is spent on **Ideas** — one-time unlocks that open structures, jobs, and permanent modifiers. The leaf owns the idea tree (definitions, costs, prerequisites, effects-as-flags). The curiosity *resource* itself (stock, cap, generation) belongs to the economy leaf; this leaf is the spending side.

Design intent: the tree is a ladder, not a maze. v1 has a single mostly-linear track with a few branches, each idea readable as a small joke that also states its effect (Tenet 3: whimsy with clarity — the flavor name goes on the label, the mechanical effect in the description the shell always shows).

## The Idea Tree (v1)

Costs in curiosity. Prerequisite `—` means available once the curiosity cap is above zero (i.e., the first Cardboard Box exists — the economy leaf's physical gate); before that, root ideas are `Locked`. An unlocked idea the player can't yet afford is `InsufficientResources`, per the engine's error precedence.

| Idea | Cost | Prereq | Effect (flag consumed by) |
|---|---|---|---|
| **Pantry Organization** | 15 | — | unlocks Pantry structure (economy) |
| **The Scratching Arts** | 25 | — | unlocks Scratching Post structure (economy) |
| **Groomed to Purrfection** | 40 | Pantry Organization | unlocks Groomer job (economy/kittens) |
| **Twig Architecture** | 60 | The Scratching Arts | unlocks Cat Tree + Twig Pile structures (economy) |
| **Selective Hearing** | 80 | Pantry Organization | manual gather yields ×2 (economy) |
| **Biscuit Making** | 120 | Groomed to Purrfection | +10% happiness (kittens) |
| **Seasonal Awareness** | 150 | Twig Architecture | winter patch modifier ×0.25 → ×0.40; season forecast flag (shell may show next season) |
| **Zoomie Channeling** | 200 | Biscuit Making | zoomies ×1.25/120t → ×1.5/240t (engine) |
| **Neighborhood Watch** | 250 | Seasonal Awareness | arrival chance +15% (kittens) |

Research is a command (`ResearchIdea`): full-cost spend, all-or-nothing, rejected with `Locked` when prerequisites are missing and `AlreadyResearched` when owned. Completed ideas emit `IdeaResearched` and are permanent — ideas are never lost or disabled (cozy invariant: no confiscation).

## Effect Plumbing

Effects are **flags and modifier values exposed by pure lookup** (`ideas.has(IdeaId)` and derived modifier accessors), consumed inside the owning leaf's math. This leaf never mutates other leaves' state; researching an idea changes *lookups*, not stocks or rosters. In v1 every idea touches its own distinct knob, so no composition question arises; if future ideas ever share a knob, the owning leaf's formula must state the composition explicitly at that knob (rates compose multiplicatively, percentage-point bonuses sum) rather than relying on a global rule.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Tree shape | Mostly-linear ladder, 9 ideas | Wide branching tree, flat shop | Early incrementals live on anticipation, not build variety; a ladder keeps the TUI list simple and every idea a visible milestone |
| Effects as flags/lookups | Pure `has(idea)` + derived modifiers | Event-driven effect application, direct state mutation on research | Idempotent under save/load and catch-up; no ordering bugs; each owning leaf keeps its own math legible |
| Ideas are permanent | No respec, no disable | Respec mechanic | Nothing the player earns is ever taken (Tenet 1); respec adds decision anxiety to a cozy game |
| Cost curve | 15 → 250, roughly ×1.5 steps | Flat costs, steeper exponential | Each idea should land a session-scale wait apart in the early game without stalling the mid game |

## Open Questions & Future Decisions

### Deferred
1. Tier-2 ideas that interact with roadmap systems (ceremonies, expeditions) — land with those segments.
2. Whether Seasonal Awareness's forecast belongs in core state or as a pure derived flag the shell reads — trivially the latter in v1; revisit if forecasts gain mechanics.

## References

- Parent: `../sim-design.md` (curiosity → economy/kittens contract: flags, never reach-ins)
- Curiosity resource generation/cap: `../economy/economy-design.md`
