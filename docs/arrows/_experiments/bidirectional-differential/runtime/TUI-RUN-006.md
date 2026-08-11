# TUI-RUN-006

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-11 · **Extraction:** 47 lines

## Spec as written

> On startup the system shall acquire a lock beside the save file recording the owning process's id and start time, and shall release it on every exit path.

## What the blind runs found

All three readers recovered exclusive creation and the recording of both process id and start time, at up to 65%.

## Recommendation

No action.
