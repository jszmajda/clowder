# How Clowder was built

Clowder was built with [Linked-Intent Development](https://github.com/jszmajda/lid) (LID). Claude Code agents did the design, specs, tests, code, reviews and commits. Jess Szmajda, the human owner, set the direction and played the game. This document uses the session logs, the prompt history and the repository history to measure how much direction she gave and where it went.

## At a glance

| | |
|---|---|
| Calendar span | 2026-07-04 → 2026-10-15 (about 15 weeks) |
| Prompts Jess typed | 77 in total: 64 natural-language messages, plus 13 slash commands, shell commands or pasted resume commands |
| Her words | about 1,340 before this write-up was requested; median message length 15 words; longest message 120 words |
| Question dialogs answered | 10 dialogs, 18 questions |
| Agent activity (from 2026-07-25 onward; see *Gaps*) | about 51 wall-clock hours on 26 days; 23.8k model turns; 8.4M output tokens; 13.7k tool calls; about 450 subagent transcripts; 26 workflow launches |
| Model mix (share of model turns) | Opus 44%, Fable 29%, Sonnet 27% |
| What exists now | 144 commits; the HLD plus 19 design docs; 467 EARS specs (466 implemented); about 17k lines of Rust source and 63k lines of Rust tests; 1,125 passing tests |
| Files Jess edited by hand (visible in the logs) | one: `AGENTS.md`, on 2026-10-14 |

## Sources and gaps

- **Prompt history** (`~/.claude/history.jsonl`) holds every prompt Jess typed for this project, including the founding session.
- **Session transcripts** exist for 2026-07-25 onward: four main sessions and their subagent and workflow transcripts. Two of the sessions were resumed from earlier ones, so messages were de-duplicated by ID before counting.
- **The founding session is missing.** It ran from 2026-07-04 to 07-06 in `~/src`, before the repo had its own directory, and its transcript no longer exists. Jess's prompts from it survive. The agent work in it does not, so the agent totals above leave it out, although that session produced the first 29 commits.
- **The blind audit sessions are missing.** The bidirectional-differential audits ran fresh Claude sessions as separate processes, and none were kept. The 256 per-spec audit records under `docs/arrows/_experiments/` are what remains of them.
- **Permission approvals are not counted.** Every session ran in Claude Code's auto permission mode, so approving tool use was not a human step.
- **Play time is not logged.** It shows up only through the bug reports it produced.
- **Agent-active hours** are wall-clock time during which any main or subagent transcript logged an event, counting gaps of up to five minutes. Agents running in parallel add no extra hours.
- **One commit cannot be traced.** The commit dated 2026-07-20 has no matching session or prompt in the logs.

## Timeline

### 1. Founding: Jul 4–6

The project began with one prompt:

> Hi. I'd like to design a TUI incremental game using /linked-intent-dev:linked-intent-dev . I'd like it to be loosely based on https://wiki.kittensgame.com/en/home - but with a more actual kittens theme. The idea of sacrificing kittens and unicorns is a little.. dark.. so let's adjust those to be more like some kind of fun / obeyance kind of thing. Let's build it using Rust and ratatui, with a fun full-spectrum colorscheme (my terminals can handle it), and let's think carefully about the keyboard shortcuts we can use when embedded in tmux.

Less than three hours later the agent had drafted a high-level design. Jess approved it and handed over the rest:

> Yay - this looks super perfect. I'd like to do an odd thing with you here though with LID - I want you to run the full LLD design loop for the game engine core, but I also want to be surprised. As this is just for me in my own terminal, I can accept a lot of risk, so I'm totally happy for you to run with this entirely on your own. Feel free to check in with me on aspects that might affect what I'd like - but _don't reveal secrets_ of the game. […]

The Spoiler Policy in `AGENTS.md` comes from this message. Ten minutes later she added one preference ("I relaly enjoy presitge system unlocks :)"). The next morning she pushed on scope ("Sounds like the game system is pretty basic so far? Let's make sure we get to the level of depth kittens game has - read the wiki in detail please") and checked that the agent was following LID ("I assume you expanded on intent first before moving to specs, then tests, then code?"). After a rate-limit interruption she told the agent to continue and to consult Fable on any change under `docs/`.

**Her input:** six messages. **Result by Jul 6:** a headless simulation core, with 14 design docs and 187 specs, all implemented and tested. There was not yet anything to play.

### 2. Building the terminal interface: Jul 25 – Aug 1

Jess came back three weeks later with "Hi - this app is .. mostly done I think, for v1.0. Can you review to see what's left?" The agent pointed out that the terminal interface had not been built. Most of her direction in this stretch was about how the work should run, not what the game should be:

> Let's plan a workflow, and try to use Sonnet where you can […] Build tests with Opus, implementations with Sonnet if possible, but keep implementation agent jobs scoped small.

She asked for the shared types to be written first and checked against each other. She added a Fable code review after the types, at interim points, and as a gate inside every workflow. She told the agent to "Ask Fable to create reasonable answers for the ambiguities, using the intent tree as context" instead of bringing those questions to her. She once pushed back on a question she shouldn't have needed to answer: "Do I really need to make a choice here or is there an obvious answer?"

She made two product decisions in prose: whether a second copy of the game may launch, and whether a feature had to persist. She answered three dialogs, four questions in all. On one of them, the order of the interface work, she chose something other than the recommended option.

**Her input:** 18 messages, 7 of them go-aheads.

### 3. First launch and the audit loop: Aug 2 – Sep 9

Running the program for the first time changed the plan:

> huh. well. I booted it and the TUI is oddly wired. At _least_ there's no indicator of where my cursor is, and the menu system doesn't seem to be taking advantage of ratatui constructs. Where are we _really_ on that? Get a fable inspeciton.

The agents' account of the interface had not matched what was on screen. She asked for the fix and for verification with the bidirectional differential. She then started an unattended `/loop` with one instruction: "hold HLD and LLDs constant, leverage the /bidirectional-differential skill until all arrow segments have been audited and confirmed aligned to intent […] Follow the full LID practices". The loop ran for about 20 hours of wall-clock time and stopped when she told it to finish its last batch.

The rest of this stretch was about LID itself, not Clowder:

- whether an open LID issue would have prevented an error the audit had found
- how to write a reusable loop that takes any LID project from finished design to delivered, audited intent

Those conversations fed back into the LID repository.

**Her input:** 15 messages and one dialog, which set the scope of the first audit. Five messages were about Clowder's direction; the rest were status questions or about LID.

### 4. Finishing the interface: Sep 24 – Oct 13

The second course correction also came from running the binary:

> start fresh please. fwiw `cargo run` and `cargo run crates/clowder-tui` just seem to hang. […] we should use dynamic workflows to implement the unimplemented work there - opus for design and test writing, sonnet for code, fable for reviews

She made one decision in prose, by applying her own tenet: "for this game, where I'm looking for you to surprise me, choice 1 is more consistent with that". A dialog on Sep 25 asked her to ratify an extension to the autonomy grant: agents may settle details a ratified LLD leaves open, judged against the HLD's tenets. An agent had drafted that clause; she confirmed it. The longest unattended stretch in the logs came next: about 11 agent-active hours between resuming a paused workflow and the next question for her.

Then she started playing:

> I'm playing the game now, it's cute! Why are kittens showing up twice in the jobs screen? Also there's no clear nav that shows which menu I'm in

Most of her feedback on the game itself came from these play sessions:

- a duplicated list
- a missing "which screen am I on" indicator
- a request to show more on one list and to split up a crowded pane
- an action that worked but left no lasting sign on screen

On Oct 13: "We can call this 1.0 easily."

**Her input:** 10 messages and three dialogs with seven questions. She overrode the recommendation once, on how rows with several actions should display.

### 5. Version 1.1: Oct 13–15

The headline request was mouse support: "For 1.1, can we add mouse support to navigate things easily? I connect via tmux if that matters". She also reported a display bug, asked for a way to hide finished items, and asked for "more color on the resource names". In the color dialog she replied in free text instead of picking an option: "you can use all three in whatever mixture you like. Our clowder is diverse :)".

The HLD changed once after July, on Oct 14, and she reviewed that change: "what's the HLD wording proposed?" … "oh yeah, that's fine".

The one hand edit in the logs happened in this phase. Claude Code's safety classifier blocks agent commits to `AGENTS.md`, so she applied the agent's proposed diff herself and committed it. That diff added the clause that lets an agent fix a bug she reported even when the fix crosses arrow segments.

**Her input:** 13 messages and three dialogs with six questions.

## What kind of direction it was

The 62 natural-language messages before this write-up was requested, grouped by purpose:

| Kind | Count | Examples |
|---|---:|---|
| Go-ahead, approval or wrap-up | 14 | "yeah go ahead", "let's do it", "go ahead and commit" |
| Process and orchestration | 13 | which model does what; review gates; workflows; LID phase checks |
| Bug reports and UX feedback from playing | 10 | duplicated rows; a hang on launch; a missing resource; hiding finished items; more color |
| Status questions | 8 | "ok so.. where are we on building this game then?" |
| About LID itself, not Clowder | 7 | designing a reusable delivery loop; commenting on a LID issue |
| Vision and scope | 5 | the opening brief; "the level of depth kittens game has"; "call this 1.0" |
| Product decisions in prose | 2 | single-instance launch; applying the surprise tenet |
| Applying an `AGENTS.md` diff by hand | 2 | "show me the diff you want me to change for AGENTS?" |
| Autonomy grant | 1 | "I want to be surprised" |

**Dialogs.** The 10 dialogs asked 18 questions, and 17 of them came with a recommended option. Jess took the recommendation 13 times. She chose differently 4 times:

- the order of the interface work
- how rows with several actions display
- what a mouse click does
- how resource names are colored, where she gave a free-text answer

## What the human contributed

1. **The brief and her taste.** The opening prompt set the genre, the tone (nothing grim), the stack, the colors and the tmux constraint. She later added two scope pushes, toward more depth and toward release, and a few preferences.
2. **The autonomy grant.** She granted it once in plain words. Agents drafted both later extensions, and she ratified each one.
3. **How the work was run.** She chose model routing (Opus for design and tests, Sonnet for code, Fable for review), the review gates and the long-running loops. Agents carried them out.
4. **Reality checks.** Three times the project was not where the agents' reports suggested:
   - the core was thinner than the game she had described ("pretty basic so far?")
   - the interface was "oddly wired"
   - the binary hung on launch

   The last two came to light only when she ran the program. Play-testing then found bugs the test suite had missed. A person running the program was the one check that agent review did not replace.
5. **What she did not do.** By design, she did not read the LLDs, specs, tests or code; the spoiler policy forbade it. Agent reviewers and the HLD's ratified tenets stood in for the reviews LID normally pauses for. Only HLD-level changes came back to her.

## How hands-off it was

Over 15 weeks Jess wrote about as much as a long email: some 1,340 words, a median of 15 per message. About a quarter of those messages were go-aheads. Six unattended stretches each ran for more than three agent-active hours.

Her involvement had three peaks:

- the founding afternoon
- the decisions about how agents would be organized and reviewed
- the play sessions, where running the game caught what the tests did not

Between those peaks the agents designed, specified, tested, implemented, reviewed and committed nearly everything. The one exception was `AGENTS.md`, which the agents are not allowed to commit.

The commits do not show any of this. All 144 are recorded under Jess's git identity, because the agents committed from her machine, and only 71 carry a Claude co-author trailer.
