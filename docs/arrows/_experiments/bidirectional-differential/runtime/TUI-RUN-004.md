# TUI-RUN-004

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-11 · **Extraction:** 10 lines

## Spec as written

> When SIGTERM or SIGHUP is received, the system shall record it and let the event loop take its ordinary shutdown path — saving and restoring the terminal — rather than performing that work inside the signal handler.

## What the blind runs found

Readers recovered that handlers are installed on Unix and stubbed elsewhere. None recovered the spec's actual constraint: that the handler only *records*, leaving the work to the loop.

## Recommendation

The constraint is an absence — the handler is required not to do things. Async-signal-safety is the reason and it cannot be seen in code that already obeys it. Assert it as a property of the handler's body, or state it in the design and drop it from the spec.
