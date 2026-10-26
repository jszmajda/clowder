# Achievements Specs (SIM-ACH)

## The Table

- [x] **SIM-ACH-001**: The system shall hold an achievement table of at least forty achievements, each with a non-empty name, hint, and words and a secret flag, no two names the same ignoring case, and at least eight of them secret.

## Earning

- [x] **SIM-ACH-002**: When an achievement's condition first holds — a state condition checked at every day close and after every applied command, other than SetSpeed, that emits an event, save that one reading individual kittens is checked at a day close only when that day's events include a kitten arriving or a new year; a moment condition checked against the events of every tick and every applied command — the system shall record it as earned with the current generation and the meadow's current day, and emit an AchievementEarned event naming it, classified a milestone; an earned achievement shall never be recorded again or un-earned.
- [x] **SIM-ACH-003**: The system shall let no rule of the simulation read the achievement record, so that two states differing only in their records remain identical, apart from those records and the events and journal entries earning emits, after any sequence of ticks and commands.
- [x] **SIM-ACH-004**: The system shall check achievements taking no draw from the state RNG, so that the fixed draw schedule (SIM-ENG-018) and every random outcome are unchanged by them.
- [x] **SIM-ACH-005**: The system shall expose a read-only query listing every achievement in table order with its definition and, where earned, the generation and day it was earned on.
- [x] **SIM-ACH-006**: The system shall decide each achievement's condition exactly as the achievement table's condition for it states, a state condition reading only the meadow as it stands and a moment condition only the events checked.
