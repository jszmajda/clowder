# TUI-THEME-012

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 18 lines

## Spec as written

> When the terminal does not advertise truecolor support, the system shall say so once at startup in plain words naming what to set, and shall then render unchanged rather than substituting a reduced palette.

## What the blind runs found

All three readers recovered the detection precisely, including the case-insensitive comparison and both accepted values, and that rendering continues unchanged. 55% on the strongest run.

## Recommendation

No action. A spec whose behaviour is a *message* rather than a state change, and it still read cleanly — which is a useful counterexample to the assumption that only state changes are recoverable.
