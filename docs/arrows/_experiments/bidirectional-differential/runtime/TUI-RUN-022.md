# TUI-RUN-022 — suspend costs no game time

**Classification:** `A-ONLY-DRIFT`
**Runs:** A=3, B=3

## The divergence

Every A-run, reading "exceeds monotonic elapsed by more than a small
tolerance", wrote a threshold constant — `SUSPEND_TOLERANCE`, at 500 ms, 2 s
and 2 s respectively, each commented as jitter or NTP slack. The code has no
such constant. It takes `monotonic.max(wall)`, converting any excess at all.

Three independent readers inventing the same absent constant is the spec
promising a mechanism the code does not have.

## Which is right

The code. A threshold is a floor below which absence is silently discarded,
and the excess of wall over monotonic is by construction non-negative, so the
only thing a tolerance can do is lose the player small amounts of time they
were owed. Sub-tick excess is carried as a fraction rather than dropped, so
converting it costs nothing. Forward clock drift credited as absence errs
toward giving time back, which is the direction the meadow should err.

Decided on **idle-respect**, the second ratified tenet.

## Repair

EARS reconciled: the tolerance clause is removed and the reason it is absent is
stated, so the next reader does not reintroduce it. Tests already assert the
max-of-both behaviour. Code unchanged.
