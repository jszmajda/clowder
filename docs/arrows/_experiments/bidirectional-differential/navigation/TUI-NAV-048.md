# TUI-NAV-048

**Classification:** `INCONSISTENT-BLIND` · **Action:** `harness-fixed` · **Audited:** 2026-08-10 · **Extraction:** 3 lines

## Spec as written

> Where the terminal is about 120 columns or wider, the system shall lay out three columns — sidebar, list capped for scannability, and inspector taking the remainder — giving extra width to the inspector and the ticker.

## What the blind runs found

Same fault as TUI-NAV-047. All three readers refused rather than confabulating, which is the second time that refusal has correctly signalled a broken harness rather than a broken project.

## Recommendation

Left out of the audited count. Re-run once the citation is placed on code rather than a module header.
