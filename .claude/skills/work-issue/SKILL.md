---
name: work-issue
description: Work one eligible GitHub issue end to end through linked-intent development, close out every arrow it touched, push to main, and close the issue. One issue per run, built to be repeated by `/loop /work-issue` from a jj workspace of its own. Use when Jess invokes it or a loop fires it; not for ad-hoc fixes she asks for in conversation.
---

# Work one issue

Take one eligible issue from Clowder's GitHub tracker through the whole arrow, leave every arrow it touched complete, ship it to `main`, and close it. `/loop` brings you back for the next one.

This skill says what is particular to Clowder's issue loop. For how each part is done, lean on the skills that own it: `/loop` for pacing and keeping the loop alive, `linked-intent-dev` for the change itself, `arrow-maintenance` for auditing an arrow, and `create-issue` for writing an issue. Where they and this skill meet, use your judgment about what Jess would want; she would rather you think than follow a script.

Everything in `AGENTS.md` applies, above all the **Spoiler Policy**. Nothing you write in this conversation, in an issue comment, in a commit message, or in the changelog names game content. Reports stay structural: issue numbers, segments, file paths, and test counts.

## Running unattended

No one is watching. Every turn either sets up the next firing or stops the loop, as `/loop` describes; stop only when the queue is empty or when this skill says to. A denied permission prompt for a command the loop needs is a stop; never route around it.

Run only from a jj workspace no other session edits (Jess sets one up with `jj workspace add ../clowder-loop`). If this checkout is the `default` workspace, stop. jj snapshots every file in the working copy, so a shared checkout would ship someone else's edits under this loop's `Fixes #N`.

## 1. Resume or pick

The loop's claim on an issue is the `in-progress` label, and only one loop runs at a time. If an open issue carries it, that issue is yours from an earlier firing: pick it up where its state shows it stands. Treat any of its commits not yet on `main@origin` as unverified until step 4 passes again.

Otherwise, `jj git fetch`, make sure the working copy holds no stray changes, and start from `jj new main@origin`. Choose the next **eligible** issue: open, carrying neither `needs-jess` nor `in-progress`, and either labeled `approved` or created by `jszmajda` without the `agent-found` label. Agents file issues through `gh` under Jess's account, so authorship alone cannot tell hers from theirs; the label does. Take bugs first, then the oldest. Claim it with `in-progress`, and stop if the label can't be applied. If nothing is eligible, the queue is done: stop the loop.

Never add `approved` to an issue. That label is Jess's alone.

## 2. Work it through the arrow

Run the change through `linked-intent-dev`, every phase, tests before code. The standing overrides in `AGENTS.md` hold. Phase reviews resolve against the HLD's tenets rather than waiting on Jess, and everything below the root HLD is yours to change, across segments included.

**The HLD.** By creating or approving the issue, Jess approved the HLD change it plainly calls for, so make it. The HLD is the one design doc she reads: keep an edit at its altitude, and name no game content it does not already name. An HLD change she would not expect from the way the issue is written still goes to her.

**What goes to Jess.** Hand the issue over instead of settling it when settling it would:

- change the HLD in a way the issue's wording would not lead her to expect;
- change `AGENTS.md`, which agents cannot commit;
- be hard to undo, such as a save-format change old saves cannot survive, or an effect outside the repository beyond this loop's push and issue updates.

To hand it over, comment on the issue with a question she can answer without spoilers. Name any commits already made for it by their change IDs, and leave them in place. Revert uncommitted work, swap `in-progress` for `needs-jess`, put the local `main` bookmark back on `main@origin` if you moved it, and move on to the next issue.

Commit each passing step as it lands. The issue's last commit says `Fixes #N` and names the segments touched. Add a spoiler-free line under `## [Unreleased]` in `CHANGELOG.md`, following that file's own rule.

## 3. Close out every arrow you touched

The issue is finished only when every arrow segment it touched is complete. That means every segment whose design, specs, tests, or code changed, and every segment the change reached by cascade. Audit each with `arrow-maintenance`, scoped to those segments, and fix what it finds. This is the directed pass the loop is authorized to run.

Complete means the arrow holds together end to end. What the design says, the specs say. Every spec marked done is truly implemented, tested, and annotated, and nothing is left silently open. The overlay in `docs/arrows/` reflects all of that and records the audit. Gaps the segment already had count too. Where closing one would be a feature of its own, file it (step 6) and defer it with that issue's number, rather than leaving it open.

## 4. Verify

Run the whole workspace once, in the background, keeping the log and the exit code:

```bash
cargo test --workspace --no-fail-fast > target/work-issue-test.log 2>&1; echo "exit=$?"; grep -E "FAILED|panicked|^error" target/work-issue-test.log; echo "ok-binaries: $(grep -c 'test result: ok' target/work-issue-test.log)"
```

Then run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets`, after the tests rather than alongside them, since they share the build lock. Add no clippy warnings to files you touched.

A lone failure in a held-key test that paces itself with real sleeps is a known flake under load (`~/src/papercuts.md`). Rerun that binary alone, and note the occurrence there. Fix any other failure. If you can't, hand the issue to Jess. **Never push with a failing test.**

## 5. Ship

Pushing to `main` from this skill carries Jess's standing authorization; anywhere else, pushing still waits for her word.

`jj git fetch`. If `main@origin` has moved, rebase onto it, and go through steps 3 and 4 again if that changed anything. Then move `main` to the issue's last commit and `jj git push --bookmark main`. Never force. If the push is refused, put the local bookmark back on `main@origin` and try again from the fetch. Confirm the issue closed; GitHub closes it from `Fixes #N`, sometimes after a few seconds. If it didn't, close it with a comment naming the commit. Remove `in-progress`.

## 6. Findings outside the issue

File anything the issue doesn't cover as its own issue, in `create-issue`'s shape, written with no game content. Label it `agent-found`, plus `bug` or `enhancement`, and don't interview anyone. It waits for Jess's `approved` before any loop works it.

## 7. Report

One structural line per issue, then set up the next firing:

`#N fixed · arrows closed: a (OK), b (PARTIAL, 2 deferred) · 70/70 test binaries · pushed <short hash> · filed #M (agent-found)`
