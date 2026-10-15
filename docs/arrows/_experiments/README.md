# Experiments

Records from experimental audit methods run against this repository. Nothing here is intent. The design lives in [`docs/intent/`](../../intent/), and each segment's current state lives in [`../index.yaml`](../index.yaml). These files are kept as evidence of what the audits found and how they found it.

## `bidirectional-differential/`

Output of the bidirectional-differential audit from LID's `lid-experimental` plugin. It ran across all 17 arrow segments on 2026-08-10 and 2026-08-11, with the high-level design and the component designs held fixed.

For each requirement, fresh Claude sessions given nothing but one artifact worked in both directions. Some rebuilt the code from the EARS line alone. Others rebuilt the EARS line from the code, with its annotations stripped. The drift between each reconstruction and the real artifact was then classified.

| Path | What it holds |
|---|---|
| `<segment>/<SPEC-ID>.md` | One record per audited requirement, 256 in all |
| `<segment>/FINDING-*.md`, `FINDING-*.md` | Write-ups of findings larger than a single requirement |
| `ledger.json` | The per-segment ledger: which requirements were auditable and how each audit came out |
| `structural-scan.md`, `structural-scan.json` | Grep-based scans for the patterns the differential surfaced, sized across the whole tree, with numbered addenda |
| `partition.json`, `crowding.json`, `uncited.json`, `untested.json`, `unpreparable-sim.json`, `module-only.json` | Intermediate sets the audit and the scan worked from, such as which method each requirement was routed to and which requirements lacked a citation or a test |

[`../AUDIT-SUMMARY.md`](../AUDIT-SUMMARY.md) summarizes what the audit found and what it left for a human decision.
