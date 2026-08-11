# TUI-NAV-042

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 17 lines

## Spec as written

> The system shall ignore a printable key with no binding in the current context and show no footer message, and shall show a footer note only for a key that is bound but not meaningful in the current state.

## What the blind runs found

All three readers recovered the footer note for a recognised-but-not-now action. None recovered the other half: that an unbound printable key is ignored *silently*.

## Recommendation

Compound, and the missing clause is an absence — there is no code for doing nothing quietly. Split it, and let a test assert that an unbound key leaves the footer untouched.
