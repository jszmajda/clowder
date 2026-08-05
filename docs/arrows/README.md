# Arrow overlay

A navigation and audit layer over `docs/intent/`. The design tree there carries
the *intent*; this carries the *state* of each segment — what is implemented,
what has drifted, and what to open next — so neither has to be reconstructed by
reading the whole tree.

Start at `index.yaml`. It is authoritative for per-segment state (`status`,
`sampled`, `audited`, `audited_sha`, `next`, `drift`, dependencies). Load it
before any per-segment doc, then follow a node's `detail` to the page for the
thing you actually care about.

## Layout

`docs/arrows/` mirrors the shape of `docs/intent/`. A **leaf** segment — a node
that owns EARS specs — gets an arrow doc at its mirrored path. A **sub-HLD**
groups children and owns no specs, so it has no arrow doc; its `index.yaml`
entry points at its design doc instead.

```
index.yaml                              the manifest — read this first
persistence/persistence.md              a leaf at the root
sim/<leaf>/<leaf>.md                    the twelve simulation leaves
tui/<leaf>/<leaf>.md                    the four shell leaves
```

A segment's boundary is its leaf prefix in the path-concatenated spec IDs, so
`grep TUI-NAV` gathers everything the navigation segment owns — specs, tests,
and the `@spec` annotations in code.

## What is derived and what is authored

An arrow doc's **References** and **Spec Coverage** are *derived views*,
regenerated from source scans during an audit. Do not hand-edit them to say
something the source does not; the next audit will overwrite it, and in the
meantime it lies.

State fields live in `index.yaml`. Design rationale lives in the LLD. An arrow
doc is an orientation page and should stay one.

## Provenance of this overlay

Bootstrapped from the existing LID docs rather than from an audit. Coverage
reflects the spec files' own `[x]`/`[ ]`/`[D]` markers — what the project
*believes* — which is not the same as what the code does. Every segment carries
`audited: null` and `audited_sha: null` to say so plainly.

That distinction has already mattered here once: a batch of specs was marked
implemented for behaviour that did not exist, and a green test suite did not
notice. Treat marker counts as a claim to be checked, not as a finding.
