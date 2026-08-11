# TUI-RUN-003

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Extraction:** 22 lines

## Spec as written

> When the process panics, the system shall restore the terminal before the panic message is printed, so the message lands on a usable screen.

## What the blind runs found

Readers recovered the all-or-nothing terminal setup and its unwinding, and one reached the panic path explicitly.

## Recommendation

No action.
