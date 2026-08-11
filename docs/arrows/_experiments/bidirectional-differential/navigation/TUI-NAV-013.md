# TUI-NAV-013 — count rows

**Classification:** `UNANNOTATABLE` (harness), repaired

Not audited: the extractor found nothing to extract. The only `@spec` in source
sat on a one-line field doc declaring that count rows exist, while the behaviour
— one worker on, one worker off — lives in the handler, whose own marker was a
plain comment rather than an annotation.

That is the placement rule in AGENTS.md exactly: the annotation belongs at the
entry point of the behaviour's implementation graph, not on a declaration that
mentions it. Moved. The spec is covered by two tests, which is why the coverage
scan never flagged it — a misplaced source citation is invisible to that check.
