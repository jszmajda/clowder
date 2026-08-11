# TUI-THEME-001 — no background of its own

**Classification:** `B-ONLY-DRIFT`
**Runs:** A=3, B=3

## Spec

The system paints no background, rendering ordinary text in the inherited foreground.

## What the blind readers saw

All three B-runs recovered the role-to-colour mapping and the inherited role that sets no foreground, and none recovered that no background is ever painted. Absence again

## Repair

 the property is that a call does not appear anywhere in the module.:No repair needed. The inherited-foreground half is recoverable and was recovered; the never-painted half is not visible in code by construction.
