# TUI-THEME-013 — notice before the alternate screen

**Classification:** `B-ONLY-DRIFT`
**Runs:** A=3, B=3

## Spec

The truecolor notice is emitted before entering the alternate screen, so it survives on the player's scrollback.

## What the blind readers saw

All three B-runs recovered what the notice says — the approximation, stderr, how to enable truecolor — and none recovered when it is said. Relational once more

## Repair

 the cited region holds the notice, and the alternate screen it must precede is elsewhere.:Covered by a test in theme.rs that asserts the ordering. No repair needed.
