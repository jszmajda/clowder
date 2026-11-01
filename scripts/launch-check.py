#!/usr/bin/env python3
"""Is macOS stalling the first launch of freshly built test binaries?

Two machine-level problems have each made every first launch of a new test
binary take 30-70 seconds. Neither fails a test: the suite just takes an hour
instead of minutes.

1. macOS scans a new executable before letting it run, unless the app the
   shell runs under is listed, switched on, under System Settings -> Privacy &
   Security -> Developer Tools.
2. An overgrown target/ directory stalls binaries linked into it even when the
   scan is skipped. `cargo clean` clears it.

This relinks the clowder-core integration-test binary and launches it twice in
place (problem 2), then launches re-signed copies from a temporary directory,
which macOS treats as executables it has never seen (problem 1).

Run: python3 scripts/launch-check.py     (non-zero exit means a stall was found)
"""

import json
import pathlib
import platform
import secrets
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
SUITE = ROOT / "crates/clowder-core/tests/integration/main.rs"
STALL = 2.0  # seconds a first launch may run over the second before it counts as stalled


def build() -> str:
    SUITE.touch()  # a fresh link makes the next launch this binary's first
    out = subprocess.run(
        ["cargo", "test", "-p", "clowder-core", "--test", "integration", "--no-run",
         "--message-format=json"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    for line in out.splitlines():
        msg = json.loads(line)
        if msg.get("reason") == "compiler-artifact" and msg.get("executable"):
            return msg["executable"]
    sys.exit("cargo reported no integration-test binary")


def launch(exe: str) -> float:
    start = time.monotonic()
    subprocess.run([exe, "--list"], capture_output=True, check=True)
    return time.monotonic() - start


def stalls(label: str, exe: str) -> bool:
    first, second = launch(exe), launch(exe)
    stalled = first - second > STALL
    print(f"  {label}: first launch {first:.2f} s, second {second:.2f} s{'  STALLED' if stalled else ''}")
    return stalled


def main() -> int:
    if platform.system() != "Darwin":
        print("not macOS: nothing to check")
        return 0

    exe = build()
    in_place = stalls("freshly linked, in target/", exe)

    # The scan does not catch every new executable, so one fast launch proves little.
    scanned = False
    with tempfile.TemporaryDirectory() as tmp:
        for n in range(3):
            copy = f"{tmp}/probe{n}"
            shutil.copy2(exe, copy)
            subprocess.run(
                ["codesign", "--force", "--sign", "-",
                 "--identifier", f"clowder.launch-check.{secrets.token_hex(4)}", copy],
                capture_output=True, check=True,
            )
            scanned |= stalls(f"never-seen copy {n + 1}", copy)

    if scanned:
        print("macOS is scanning new executables: list your shell's app under Developer Tools (see README).")
    elif in_place:
        print("target/ is stalling launches: run `cargo clean`.")
    else:
        print("  no stall")
    return 1 if scanned or in_place else 0


if __name__ == "__main__":
    sys.exit(main())
