# Persistence Specs (SAVE)

## Format & Versioning

- [x] **SAVE-001**: The system shall serialize saves as pretty-printed JSON with the shape { save_version, saved_at_unix, state }, where saved_at_unix is supplied by the caller (the sim never reads clocks).
- [x] **SAVE-002**: If a save's save_version is newer than the binary understands, then load shall fail with a TooNew error naming the found version, never guessing forward.
- [x] **SAVE-003**: The system shall write saves atomically: serialize to a temp file in the same directory, fsync, then rename over the save file; a leftover temp file from a crashed write is ignored by load and overwritten by the next save.
- [x] **SAVE-004**: If the save file is absent, load shall return NotFound (the caller starts a new game); if the file is unparseable, missing required fields, or references unknown content ids, load shall return a Corrupt error with detail — and core shall never delete, overwrite, or rewrite a save file during load.
- [x] **SAVE-005**: When a parseable save contains out-of-range numeric state, the system shall normalize it on load — numeric fields deserialize wide and clamp, stocks clamp to caps, counters clamp to range — rather than rejecting the save (the format is advertised as hand-editable).
- [x] **SAVE-011**: The system shall persist only canonical state — derived values (happiness, rates, season) are recomputed after load — including the bounded event journal, which is canonical because it records what the player has already lived through and cannot be recomputed from anything else.
- [x] **SAVE-014**: The system shall carry save_version 3, migrating a v2 save forward by giving it an empty journal — a save written before the journal existed has no history to recover — with migration fidelity tested against a committed v2 fixture file.
- [x] **SAVE-015**: When catch-up spans at least a minimum interval, the system shall record the resulting OfflineReport into the journal as a retained entry, so a report survives being dismissed unread or the game being closed before it is read; a catch-up below that interval shall record nothing.
- [x] **SAVE-017**: The system shall carry, alongside a save's timestamp, the count of ticks the caller had banked but not yet run, and shall replay those ticks as counted rather than re-deriving them from an interval — ticks earned at one speed and saved at another would otherwise be lost or conjured by the ratio between the two.
- [x] **SAVE-016**: The system shall expose the catch-up cap as part of its public surface, so a caller applying the same limit to an interval it observed itself uses this value rather than a copy of it, there being one rule for how much absence is replayed.
- [x] **SAVE-012**: Every persistence IO function shall take an explicit file path, with the default path helper resolving to the platform data directory + clowder/save.json.

- [x] **SAVE-013**: When loading a save whose save_version is older than the current version, the system shall migrate it forward one version at a time — added fields receive their new-game defaults and existing state is preserved exactly — verified against a committed fixture file of each prior version.

## Offline Catch-up

- [x] **SAVE-006**: When loading with a caller-supplied now_unix, the system shall compute elapsed = max(0, now_unix − saved_at_unix), cap it at 365 days, and replay elapsed × 4 × speed ticks (speed = the save's persisted game-speed) through the core's replay entry point — the same simulation step as live play in every respect but journaling, which is history-keeping rather than a game rule.
- [x] **SAVE-007**: If now_unix is earlier than saved_at_unix (a backwards clock), then the system shall load the save unchanged with zero ticks replayed, never regressing or refusing the state.
- [x] **SAVE-008**: The system shall fold catch-up replay events into an OfflineReport containing the elapsed span, a truncated flag when the 365-day cap applied, per-resource net deltas, gross name lists of kittens arrived/wandered/returned aggregated with counts per name (a kitten may appear in more than one list, and more than once per list), and seasons passed. (Commands cannot occur during replay, so the report never contains builds or research.)
- [x] **SAVE-009**: Replaying 30 days of offline time shall complete in under 5 seconds in a release build.
- [x] **SAVE-010**: A save/load round-trip with zero elapsed time shall reproduce bit-identical game state, verified by comparing serialized bytes (which requires canonical collection ordering per SIM-ENG's determinism contract).
