---
name: work-issue
description: Work one eligible GitHub issue end to end through linked-intent development, push it to main, and close it. One issue per run, built to be repeated by `/loop /work-issue` from a jj workspace of its own. Use when Jess invokes it or a loop fires it; not for ad-hoc fixes she asks for in conversation.
---

# Work one issue

Take one eligible issue from Clowder's GitHub tracker through the whole arrow, ship it to `main`, and close it. `/loop` brings you back for the next one.

Everything in `AGENTS.md` applies, above all the **Spoiler Policy**. Nothing you write in this conversation, in an issue comment, in a commit message, or in the changelog names game content: no resource, structure, job, idea, event, or adventure names, no balance numbers, and no flavour text. Reports stay structural: issue numbers, segments, file paths, and test counts.

## Keeping the loop alive

The loop runs unattended, so every turn ends in one of two ways:

- **Continuing.** Call `ScheduleWakeup` with `delaySeconds: 1800` and the same `/loop` prompt, then end the turn. This is a fallback heartbeat: a background command started this turn re-invokes you when it finishes, usually sooner. Never poll with short delays.
- **Stopping.** Call `ScheduleWakeup` with `stop: true` and say why in one line. Stop only when the queue is empty, or something comes up that this skill tells you to stop on.

A denied permission prompt for any command this skill needs (a label edit, a commit, a push) is a stop. Never work around it with a different command.

## 0. Preconditions and resuming

- **Its own workspace.** Run only in a jj workspace no other session edits. If `jj workspace list` shows this checkout as `default`, stop. Jess sets one up with `jj workspace add ../clowder-loop`, and the loop runs from there. jj snapshots every file in the working copy into the next commit, so a shared checkout would ship someone else's edits under this loop's `Fixes #N`.
- **The labels exist.** The skill uses `approved`, `agent-found`, `in-progress`, and `needs-jess`. If a `gh` label command fails because one is missing, stop. Never carry on without a claim.
- **Resume before you pick.** Only one `work-issue` loop runs at a time, and its claim on an issue is the `in-progress` label. If an open issue carries it, that issue is yours from an earlier firing: resume it, reading its state from the working copy, the commits above `main@origin`, and the issue's comments.
  - If a background test run of this loop is still going, schedule the heartbeat and end the turn.
  - If commits for the issue are above `main@origin` and not yet pushed, treat them as unverified: run step 5 before shipping, whatever an earlier firing may have seen.

## 1. Sync

Run `jj git fetch`. To start a new issue, the working copy must hold no changes (`jj st` is clean). If it holds changes no `in-progress` issue explains, stop and say so. Begin with `jj new main@origin`, so the work sits on the remote's latest `main` whatever the local bookmark says.

## 2. Pick

An issue is **eligible** when it is open, carries neither `needs-jess` nor `in-progress`, and either:

- carries the `approved` label, or
- was created by `jszmajda` and does **not** carry `agent-found`. Issues agents file with `gh` are authored by Jess's account too, so the label is what tells them apart.

```bash
gh issue list --state open --limit 100 --search "label:approved -label:needs-jess -label:in-progress" --json number,title,labels,createdAt
gh issue list --state open --limit 100 --search "author:jszmajda -label:agent-found -label:needs-jess -label:in-progress" --json number,title,labels,createdAt
```

Take the union. Order bugs (`bug` label) before everything else, then oldest first. Claim the first with `gh issue edit N --add-label in-progress`, and check the claim landed before you start.

If nothing is eligible, the queue is done: stop the loop and report that it is empty.

## 3. Work it through the arrow

Consult the `linked-intent-dev` skill and run every phase for the change. The standing overrides in `AGENTS.md` apply:

- Phase-boundary reviews resolve against the HLD's tenets, not against Jess. Do not stop between phases to wait for her.
- Everything below the root HLD is yours to change: LLDs, specs, tests, and code, across segment boundaries included, and what a feature is. Note every segment you touch.
- Where a ratified LLD leaves a detail open, pin it in that LLD, resolved against the tenets.
- Where code has drifted from a ratified design, the design is the intent: fix the code.

Run no audit or experiment skill that spends heavily or wants a scoping conversation, such as the bidirectional differential or the differential audit. The loop has no one to scope it with. The `docs/arrows/` overlay is not part of an issue's work unless the issue is about it.

Tests come before code, as the linked-intent-dev skill requires. Write them, see them fail for the right reason, then implement.

### When the issue goes to Jess instead

Some issues this loop must not settle on its own. Hand the issue to Jess when settling it would:

- change the root HLD, `docs/high-level-design.md`;
- change `AGENTS.md` (agents cannot commit it; Jess makes those edits herself);
- do something hard to undo, such as a save-format change that old saves cannot survive, a migration that drops data, or anything with an effect outside the repository besides this skill's own push and issue updates.

To hand it over: leave a comment on the issue asking the question in terms Jess can answer without spoilers (scope, risk, which source of truth wins). Name any commits already made for it by their jj change IDs, and leave them where they are; don't abandon them. Revert uncommitted work for it, swap `in-progress` for `needs-jess`, and run `jj bookmark set main -r main@origin` if you had moved the local bookmark. Then go back to step 1 for the next issue in the same turn, or schedule the heartbeat.

## 4. Commit

Commit each coherent, passing step with `jj commit` as it lands (`AGENTS.md`: commit liberally). The commit that finishes the issue:

- says `Fixes #N` in its body;
- names the segments touched (`Segments touched: …`);
- ends with the co-author line the session's attribution instructions give.

Add one spoiler-free line for the change to `CHANGELOG.md` under `## [Unreleased]` (create that heading above the newest release if it is missing). Follow the file's own rule: say what changed for the player, never what anything is called, what it costs, or what it does in numbers.

## 5. Verify

Run the whole workspace once, in the background (`run_in_background`), keeping its log, then schedule the heartbeat and end the turn until it reports:

```bash
cargo test --workspace --no-fail-fast > target/work-issue-test.log 2>&1; echo "exit=$?"; grep -E "FAILED|panicked|test result: F|^error" target/work-issue-test.log; echo "ok-binaries: $(grep -c 'test result: ok' target/work-issue-test.log)"
```

When it is done, and not while it runs (both wait on the same build lock), run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets`. No new clippy warning may land in a file you touched.

- If the exit code is 0 and nothing failed, go to step 6.
- If the only failure is a held-key test that paces itself with real sleeps, a known flake under parallel load (`~/src/papercuts.md`), rerun that test binary alone. If it passes, go on, and append the occurrence to `~/src/papercuts.md`.
- Any other failure: fix it, through the arrow, and verify again. If you cannot, comment on the issue (structurally) and hand it to Jess as step 3 describes, without pushing.

**Never push with a failing test.**

## 6. Ship

Pushing to `main` from this skill carries Jess's standing authorization; anywhere else, pushing still waits for her word.

1. `jj git fetch`. If `main@origin` moved since your tests ran, rebase onto it (`jj rebase -b @- -d main@origin`). If the rebase changed anything, run step 5 again before going on; if it conflicts, resolve it through the arrow first.
2. Only now, `jj bookmark set main -r @-` (the last commit, not the empty working copy). jj refuses a sideways move without `-B`; never pass it.
3. `jj git push --bookmark main`. Never force a push. If the push is refused, reset the bookmark (`jj bookmark set main -r main@origin`), then fetch, rebase, verify, and come back to 2.
4. Confirm the issue closed (`gh issue view N --json state`); GitHub closes it from `Fixes #N`, sometimes after a few seconds. If it is still open, close it with a comment naming the commit. Remove `in-progress`.

## 7. Findings outside the issue

Anything you find that the issue does not cover: file it as its own issue rather than widening this one. Use the `create-issue` skill's shape (problem, impact, scope, intent touchpoints, code pointers as `file:line`, spec IDs), written structurally with no game content. Label it `agent-found` plus `bug` or `enhancement`. It waits for Jess's `approved` before any loop works it. Do not interview her; the loop runs unattended.

## 8. Report

End the issue with one structural line, then schedule the heartbeat (the next firing picks the next issue):

`#N fixed · segments: a, b · 70/70 test binaries · pushed <short hash> · filed #M (agent-found)`
