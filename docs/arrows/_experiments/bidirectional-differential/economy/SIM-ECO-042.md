# SIM-ECO-042

**Classification:** `HAND-AUDITED`
**Method:** citation corrected this cycle; no longer preparable, and deliberately so.

This spec was annotated twice — on the jobs table, where the yield and the gate
live, and on the enum that merely names the jobs. The extractor took the enum,
because a self-contained type reads cleanly and a three-line accessor does not,
and all three readers dutifully described an enumeration. Confidently, and about
the wrong thing.

The enum annotation is removed, which makes this spec match the four older jobs
that were always annotated on the table alone. It also makes it unpreparable:
the accessor is below the harness floor. That is the correct trade — a blind run
describing the wrong thing is worse than no blind run.

Verified by reading the table instead: the yield, its resource, and the idea
that unlocks it all match the spec exactly.
