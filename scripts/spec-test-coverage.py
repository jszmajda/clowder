#!/usr/bin/env python3
"""Every spec marked implemented should have something asserting it.

The arrow runs HLD -> LLD -> EARS -> Tests -> Code. A spec marked `[x]` with an
`@spec` on production code but none on any `#[test]` has skipped the middle
term: it reads as finished from every direction anyone normally looks, and
nothing fails when it stops being true.

This exists because that happened. Two runtime specs sat marked implemented
while both files naming them said in prose why they *could not* be covered —
notes whose stated blockers had since been removed and which nobody revisited.
A third turned out to have a real defect behind it.

Run: python3 scripts/spec-test-coverage.py     (non-zero exit means a new gap)
"""

import collections
import glob
import re
import sys

# Specs whose behaviour genuinely cannot be asserted from a headless test, with
# the reason. Anything here must also say so where the code lives, so the gap is
# visible to a reader of the tests and not only to this script.
EXEMPT = {
    "TUI-RUN-002": "entering raw mode and the alternate screen needs a real pty; "
                   "faking one would prove nothing about the real terminal (see tests/term.rs)",
    "TUI-RUN-003": "restoring the terminal before the panic message needs the same pty "
                   "(see tests/term.rs)",
}

SPEC = re.compile(r"\s*-\s*\[x\]\s*\*\*([A-Z][A-Z0-9-]*-\d{3})\*\*")
ID = re.compile(r"\b[A-Z][A-Z0-9-]*-\d{3}\b")
TEST = re.compile(r"\s*#\[(?:\w+::)?test\]\s*$")
ITEM = re.compile(r"\s*(pub |fn |impl |struct |enum |const |static |let )")


def main() -> int:
    marked = set()
    for path in glob.glob("docs/intent/**/*-specs.md", recursive=True):
        with open(path) as fh:
            for line in fh:
                found = SPEC.match(line)
                if found:
                    marked.add(found.group(1))

    cited, on_test = collections.defaultdict(set), collections.defaultdict(set)
    sources = glob.glob("crates/*/tests/*.rs") + glob.glob("crates/*/src/**/*.rs", recursive=True)
    for path in sources:
        with open(path) as fh:
            lines = fh.read().splitlines()
        for i, line in enumerate(lines):
            if "@spec" not in line:
                continue
            ids = ID.findall(line)
            for spec in ids:
                cited[spec].add(path)
            if line.lstrip().startswith("//!"):
                continue  # a module header attaches to nothing in particular
            for j in range(i, min(i + 16, len(lines))):
                if TEST.match(lines[j]):
                    for spec in ids:
                        on_test[spec].add(path)
                    break
                if ITEM.match(lines[j]):
                    break

    gaps = sorted(s for s in marked if cited.get(s) and not on_test.get(s))
    # A spec with no `@spec` at all is not a milder case of this — it is the
    # same defect one step further along, and an earlier version of this script
    # skipped it by testing `cited` first. Nothing traced it in either
    # direction, so nothing could notice it had stopped being true.
    uncited = sorted(s for s in marked if not cited.get(s))
    unexpected = [s for s in gaps + uncited if s not in EXEMPT]
    stale = [s for s in EXEMPT if s not in gaps and s not in uncited]

    print(f"{len(marked)} specs marked implemented, {len(on_test)} asserted by a test")

    for spec in uncited:
        if spec not in EXEMPT:
            print(f"  UNCITED  {spec}: marked implemented, no @spec anywhere")
    for spec in unexpected:
        if spec in uncited:
            continue
        print(f"  GAP  {spec}: marked implemented, cited on code, asserted by no test")
    for spec in stale:
        print(f"  STALE EXEMPTION  {spec} is covered now and can leave the exempt list")
    if not unexpected and not stale:
        print(f"  clean ({len(EXEMPT)} documented exemptions)")

    return 1 if unexpected or stale else 0


if __name__ == "__main__":
    sys.exit(main())
