//! The action model — `nav::update`, the reducer that folds one `Action` into
//! the view and says what, if anything, the core must be told.
//!
//! Everything here is asserted through the reducer's own seam: an `Action`
//! goes in, a `ViewState` is mutated, an `Outcome` comes out. That is
//! deliberate. `update` takes `&GameState`, never `&mut` — it *cannot* spend a
//! resource, and it never learns whether a command it emitted was accepted.
//! The core is the single arbiter (TUI-NAV-053), so these tests never assert
//! that a thing was bought; they assert what the shell *asked for*.
//!
//! The safety property this file exists for is TUI-NAV-015 + TUI-NAV-051: a
//! heavy action arms on the first Enter and commits only on a distinct second
//! one. Auto-repeat is a machine typing, not a player deciding. If a held key
//! could satisfy the second press, the confirm would be theatre and holding
//! Enter would perform the heaviest action in the game in a tenth of a second.
//! It is tested here the way an adversary would test it: by holding the key.

use std::time::{Duration, Instant};

use std::collections::BTreeSet;

use clowder_core::content::MEADOW_MOVE_MIN_KITTENS;
use clowder_core::{
    ArchId, Availability, ColonyId, Command, ContentId, CraftId, GameState, IdeaId, JobId, Kitten,
    KittenTrait, LockReason, Location, ResourceId, RiteId, StructureId,
};

use clowder_tui::nav::rows::{rows, RowKind};
use clowder_tui::nav::{
    update, Action, ArmedRow, Outcome, Overlay, RowKey, Section, ViewState,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn kitten(name: &str, job: Option<JobId>) -> Kitten {
    Kitten {
        name: name.to_string(),
        job,
        location: Location::Home,
        arrived_day: 0,
        job_seq: job.map(|_| 1),
        away_seq: None,
        skills: [0.0; 8],
        exp: 1.0e6,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
    }
}

/// Every surface the core can be told is open, is open, and everything is
/// affordable many times over — including a roster large enough that the
/// heaviest action in the game would really succeed if the shell asked for it.
/// A commit that the core would reject anyway proves nothing about whether the
/// shell asked.
fn rich() -> GameState {
    let mut s = GameState::new(7);

    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e12;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 5;
    }
    s.ideas = IdeaId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect::<BTreeSet<_>>();

    // A meadow this size has been played into existence, so its high-water
    // reflects that: prestige stays revealed when a kitten later wanders off.
    // @spec SIM-ENG-046
    s.roster_high_water = MEADOW_MOVE_MIN_KITTENS + 9;
    let mut roster: Vec<Kitten> = (0..MEADOW_MOVE_MIN_KITTENS + 9)
        .map(|i| kitten(&format!("kitten-{i}"), None))
        .collect();
    for k in roster.iter_mut().take(4) {
        k.job = Some(JobId::Tinkerer);
    }
    roster[0].tinkering = Some(CraftId::ALL[0]);
    s.kittens = roster;
    s.elder = Some(0);

    s.peak_devotion = 1.0e12;
    s.devotion = 1.0e6;
    s.serenity = 1.0e12;
    s.rites = [RiteId::TheReverie, RiteId::Enlightenment]
        .into_iter()
        .collect::<BTreeSet<_>>();

    // One arch already bought: the ✓ owned one-shot TUI-NAV-041 speaks of.
    s.arches_bought = [ArchId::EverdreamArch].into_iter().collect::<BTreeSet<_>>();

    s.discovered = ColonyId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.sites_reached = BTreeSet::new();
    s.whale_visit_years = 5;
    s.whale_contentment = 1;

    s.generation = 1;
    s.elder_wisdom = 1.0e9;
    s.warm_memories = 1.0e3;

    s
}

/// A meadow one keypress old.
fn fresh() -> GameState {
    GameState::new(3)
}

// ---------------------------------------------------------------------------
// Helpers over the reducer
// ---------------------------------------------------------------------------

/// A view sitting in `section` with `key` focused. Constructed rather than
/// navigated to, so a test of arming is not also a test of the landing rule.
fn focused(section: Section, key: RowKey) -> ViewState {
    let mut v = ViewState::default();
    v.section = Some(section);
    v.focus[section.index()] = Some(key);
    v
}

/// The first row in `section` whose command satisfies `pred`, as (identity,
/// command).
fn find_row(
    state: &GameState,
    section: Section,
    pred: impl Fn(&Command) -> bool,
) -> (RowKey, Command) {
    rows(state, section)
        .into_iter()
        .find_map(|r| match &r.kind {
            RowKind::Act { command, .. } if pred(command) => Some((r.key.clone(), *command)),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!("fixture: no row in {} fires the wanted command", section.title())
        })
}

/// The first `+`/`-` row in `section`, as (identity, increment, decrement).
fn find_count_row(state: &GameState, section: Section) -> (RowKey, Command, Command) {
    rows(state, section)
        .into_iter()
        .find_map(|r| match &r.kind {
            RowKind::Count { increment, decrement } => {
                Some((r.key.clone(), *increment, *decrement))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture: no count row in {}", section.title()))
}

/// The four actions TUI-NAV-015 names as heavy, each with the row that offers
/// it. Named by the spec, not by a flag, so this list stays the requirement
/// even if the row builder's `heavy` flag disagrees with it.
fn heavy_rows(state: &GameState) -> Vec<(&'static str, Section, RowKey, Command)> {
    let mut out = Vec::new();

    let (key, command) = find_row(state, Section::Legacy, |c| matches!(c, Command::GreatMeadowMove));
    out.push(("GreatMeadowMove", Section::Legacy, key, command));

    let (key, command) = find_row(state, Section::Legacy, |c| matches!(c, Command::PassItDown));
    out.push(("PassItDown", Section::Legacy, key, command));

    let (key, command) = find_row(state, Section::Ideas, |c| matches!(c, Command::AdoptRule(_)));
    out.push(("AdoptRule", Section::Ideas, key, command));

    let (key, command) = find_row(state, Section::Dreams, |c| matches!(c, Command::DeepNap(_)));
    out.push(("DeepNap", Section::Dreams, key, command));

    out
}

/// The Move row: heavy by every reading, and offered in every state.
fn move_row(state: &GameState) -> (RowKey, Command) {
    find_row(state, Section::Legacy, |c| matches!(c, Command::GreatMeadowMove))
}

/// A deliberate second press: a real pause with no Enter in it, then one Enter.
///
/// Neither TUI-NAV-051 nor the design fixes a number for "distinct", so this
/// escalates rather than guessing one — and asserts, between attempts, that a
/// press too soon to count left the row armed. TUI-NAV-015 allows exactly two
/// things to disarm a row: about eight seconds passing, and navigation.
fn commit_deliberately(view: &mut ViewState, state: &GameState) -> Outcome {
    for _ in 0..4 {
        std::thread::sleep(Duration::from_millis(300));
        let out = update(Action::Activate, view, state);
        if out != Outcome::Handled {
            return out;
        }
        assert!(
            view.armed.is_some(),
            "an Enter that did not commit also disarmed the row; only the ~8s expiry \
             or a navigation may disarm (TUI-NAV-015)"
        );
    }
    panic!(
        "four deliberate Enters, each after a 300ms pause with no key in it, never committed \
         the armed row — a confirm that cannot be confirmed is a wall, not a confirm"
    );
}

fn commands_of(out: &Outcome) -> Option<Command> {
    match out {
        Outcome::Apply(c) => Some(*c),
        Outcome::Repeat(c, _) => Some(*c),
        Outcome::Handled | Outcome::Quit => None,
    }
}

/// Whether an outcome would put `command` through the core.
fn would_fire(out: &Outcome, command: Command) -> bool {
    commands_of(out) == Some(command)
}

// ---------------------------------------------------------------------------
// TUI-NAV-015 / TUI-NAV-051 — the armed row, and the held key
// ---------------------------------------------------------------------------

/// The first Enter on a heavy row arms it and tells the core nothing. The
/// consequence summary the player then reads is rendered from current state,
/// which is why the armed row carries the row's identity and the exact command
/// rather than a rendered string.
///
/// @spec TUI-NAV-015
#[test]
fn the_first_enter_on_a_heavy_row_arms_it_and_fires_nothing() {
    let state = rich();
    let (key, command) = move_row(&state);
    let mut view = focused(Section::Legacy, key.clone());

    let out = update(Action::Activate, &mut view, &state);

    assert_eq!(
        out,
        Outcome::Handled,
        "the first Enter on a heavy row reached the core; the confirm exists so that press \
         cannot be the one that acts"
    );
    let armed = view.armed.as_ref().expect("the first Enter armed nothing at all");
    assert_eq!(armed.key, key, "the armed row is not the row the cursor is on");
    assert_eq!(
        armed.command, command,
        "the armed row holds a different command than the row offers; the commit re-runs \
         *this* command, so it must be the real one"
    );
}

/// Arming expires on its own, in about eight seconds of wall time. Walking
/// away simply disarms — that is what makes a confirm that does not block the
/// simulation safe to leave sitting on screen.
///
/// @spec TUI-NAV-015
#[test]
fn arming_expires_about_eight_seconds_out() {
    let state = rich();
    let (key, _) = move_row(&state);
    let mut view = focused(Section::Legacy, key);

    update(Action::Activate, &mut view, &state);
    let armed = view.armed.as_ref().expect("nothing armed");

    let left = armed.expires_at.saturating_duration_since(Instant::now());
    assert!(
        left >= Duration::from_secs(7) && left <= Duration::from_secs(9),
        "an armed row expires in {left:?}; TUI-NAV-015 says about eight seconds — long enough \
         to read the consequences, short enough that a forgotten arming is gone before the \
         player comes back"
    );
}

/// Any navigation disarms, silently: no footer, no command, no fuss. Walking
/// away is the disarm.
///
/// @spec TUI-NAV-015
#[test]
fn any_navigation_disarms_silently() {
    let state = rich();
    let (key, command) = move_row(&state);

    for nav in [Action::CursorDown, Action::CursorUp, Action::SectionNext, Action::SectionPrev] {
        let mut view = focused(Section::Legacy, key.clone());
        update(Action::Activate, &mut view, &state);
        assert!(view.armed.is_some(), "fixture: the row must be armed before navigating away");

        let out = update(nav.clone(), &mut view, &state);

        assert!(
            !would_fire(&out, command),
            "{nav:?} committed the armed heavy action; navigating away must never be the \
             deciding press"
        );
        assert!(
            view.armed.is_none(),
            "{nav:?} left the row armed; TUI-NAV-015 disarms on any navigation, so the \
             player who wandered off does not return to a live trigger"
        );
    }
}

/// An arming that has run out of time is not a first Enter that can be
/// completed later. The expiry is silent, so the next Enter is a *first* Enter
/// again — it may re-arm, but it may never commit.
///
/// @spec TUI-NAV-015
#[test]
fn an_expired_arming_never_commits() {
    let state = rich();
    let (key, command) = move_row(&state);
    let mut view = focused(Section::Legacy, key);

    update(Action::Activate, &mut view, &state);
    // Age the arming past its expiry without waiting eight real seconds.
    view.armed.as_mut().expect("fixture: armed").expires_at = Instant::now();

    std::thread::sleep(Duration::from_millis(300));
    let out = update(Action::Activate, &mut view, &state);

    assert!(
        !would_fire(&out, command),
        "an Enter after the arming expired committed the heavy action; the expiry is silent, \
         so that press is a first press, not a second one"
    );
    let armed = view
        .armed
        .as_ref()
        .expect("the expired row neither committed nor re-armed, so Enter did nothing at all");
    assert!(
        armed.expires_at > Instant::now(),
        "the re-armed row is still carrying its old, already-expired deadline"
    );
}

/// A distinct second Enter — a real pause, then a press — commits, and the
/// commit hands the core the row's real command.
///
/// @spec TUI-NAV-015, TUI-NAV-043
#[test]
fn a_distinct_second_enter_commits_the_heavy_action() {
    let state = rich();
    let (key, command) = move_row(&state);
    let mut view = focused(Section::Legacy, key);

    update(Action::Activate, &mut view, &state);
    assert!(view.armed.is_some(), "fixture: the first Enter must arm");

    let out = commit_deliberately(&mut view, &state);

    assert_eq!(
        out,
        Outcome::Apply(command),
        "the committing Enter did not put the row's real command through the core"
    );
    assert!(
        view.armed.is_none(),
        "the row is still armed after committing; a second commit would then be one keypress away"
    );
}

/// THE safety property. A held Enter is a machine typing, not a player
/// deciding: it must arm the row and go no further, however long it is held
/// and however fast the terminal repeats.
///
/// This hammers the reducer with a dense, gapless stream of Enters — faster
/// than any terminal's key repeat, which is exactly the adversary TUI-NAV-051
/// names. Any implementation that separates the two presses by a mere elapsed
/// threshold fails here, and should: after that threshold, a *held* key would
/// still commit.
///
/// @spec TUI-NAV-051, TUI-NAV-010
#[test]
fn a_held_enter_arms_the_row_and_goes_no_further() {
    let state = rich();
    for (name, section, key, command) in heavy_rows(&state) {
        let mut view = focused(section, key);

        let start = Instant::now();
        let mut presses = 0u32;
        while start.elapsed() < Duration::from_millis(400) {
            presses += 1;
            let out = update(Action::Activate, &mut view, &state);
            assert!(
                !would_fire(&out, command),
                "holding Enter on the {name} row committed it after {presses} presses in \
                 {:?} — the heaviest actions in the game must not be reachable by leaning \
                 on a key",
                start.elapsed()
            );
        }

        assert!(
            presses > 50,
            "fixture: only {presses} presses in 400ms — the stream must be dense enough to \
             model auto-repeat for this test to mean anything"
        );
        assert!(
            view.armed.is_some(),
            "holding Enter on the {name} row left nothing armed; a held key must still arm, \
             it simply must not commit"
        );
    }
}

/// `X` is the max-affordable accelerator, and it may never reach a heavy
/// action — not to fire one, and not to finish one that is already armed. The
/// second half is the adversarial reading: an armed row plus one uppercase
/// keystroke must not be a committed Move.
///
/// @spec TUI-NAV-051
#[test]
fn x_max_never_commits_a_heavy_action() {
    let state = rich();
    for (name, section, key, command) in heavy_rows(&state) {
        let mut fresh_view = focused(section, key.clone());
        let out = update(Action::MaxAffordable, &mut fresh_view, &state);
        assert!(
            !would_fire(&out, command),
            "X on the {name} row put it through the core; TUI-NAV-051 says X never commits a \
             heavy action"
        );

        let mut armed_view = focused(section, key);
        update(Action::Activate, &mut armed_view, &state);
        assert!(armed_view.armed.is_some(), "fixture: {name} must arm first");
        std::thread::sleep(Duration::from_millis(300));
        let out = update(Action::MaxAffordable, &mut armed_view, &state);
        assert!(
            !would_fire(&out, command),
            "X committed the already-armed {name} row; only a distinct Enter may commit"
        );
    }
}

/// The four heavy actions are the four the spec names. An ordinary purchase is
/// instant — the core's all-or-nothing spend makes that safe — and a heavy,
/// hard-to-undo one is not.
///
/// @spec TUI-NAV-015
#[test]
fn the_four_heavy_actions_are_the_ones_the_spec_names() {
    let state = rich();

    for (name, section, key, command) in heavy_rows(&state) {
        let mut view = focused(section, key);
        let out = update(Action::Activate, &mut view, &state);
        assert_eq!(
            out,
            Outcome::Handled,
            "Enter on the {name} row went straight through to the core; TUI-NAV-015 names it \
             heavy, so the first Enter must arm"
        );
        assert!(
            view.armed.is_some(),
            "Enter on the {name} row armed nothing (it fired {:?} instead)",
            commands_of(&out)
        );
        assert_eq!(
            view.armed.as_ref().expect("armed").command,
            command,
            "the {name} row armed a different command than it offers"
        );
    }

    // And the converse: an ordinary purchase is instant, or the confirm has
    // spread to rows that do not need it and every build costs two keys.
    let (key, command) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));
    let mut view = focused(Section::Meadow, key);
    let out = update(Action::Activate, &mut view, &state);
    assert_eq!(
        out,
        Outcome::Apply(command),
        "an ordinary build asked to be confirmed; only the four heavy actions arm"
    );
    assert!(view.armed.is_none(), "an ordinary build armed a row");
}

// ---------------------------------------------------------------------------
// TUI-NAV-043 — arming is advisory; the core re-validates
// ---------------------------------------------------------------------------

/// Arming reserves nothing. It is a label on a row, not a hold on the stores —
/// which is why walking away from one costs the player nothing at all.
///
/// @spec TUI-NAV-043
#[test]
fn arming_reserves_nothing() {
    let state = rich();
    let before = state.clone();
    let (key, command) = move_row(&state);
    let mut view = focused(Section::Legacy, key);

    let out = update(Action::Activate, &mut view, &state);

    assert_eq!(out, Outcome::Handled, "arming asked the core to do something");
    assert_eq!(state, before, "arming changed the game state");
    let mut probe = state.clone();
    assert!(
        probe.apply(command).is_ok(),
        "with a row armed, the very command it is armed with no longer succeeds — something \
         was held back for the commit"
    );
}

/// Arming a heavy row disarms whatever was armed before it. The dangerous
/// reading is the one this test takes: an Enter on row B must not be read as
/// the second Enter for row A, however the two came to be armed.
///
/// @spec TUI-NAV-043
#[test]
fn arming_a_heavy_row_disarms_any_row_already_armed() {
    let state = rich();
    let (move_key, move_command) = move_row(&state);
    let (pass_key, pass_command) =
        find_row(&state, Section::Legacy, |c| matches!(c, Command::PassItDown));
    assert_ne!(move_key, pass_key, "fixture: two distinct heavy rows");

    let mut view = focused(Section::Legacy, move_key.clone());
    view.armed = Some(ArmedRow {
        key: pass_key,
        command: pass_command,
        expires_at: Instant::now() + Duration::from_secs(8),
    });

    let out = update(Action::Activate, &mut view, &state);

    assert!(
        !would_fire(&out, pass_command),
        "an Enter on one heavy row committed a different heavy row that happened to be armed"
    );
    assert!(
        !would_fire(&out, move_command),
        "an Enter on a freshly-focused heavy row committed it outright; the arming that was \
         live belonged to another row, so this press is a first press"
    );
    let armed = view.armed.as_ref().expect("nothing is armed at all now");
    assert_eq!(
        armed.key, move_key,
        "the previously-armed row survived; arming disarms whatever came before"
    );
    assert_eq!(armed.command, move_command, "the armed command is the wrong row's");
}

/// The commit re-runs the *real* command, so the core re-validates it against
/// current state. The shell keeps no pre-check of its own: a Move that was
/// valid when armed and is not when committed is still handed over, and fails
/// to the footer like any other rejection.
///
/// @spec TUI-NAV-043
#[test]
fn the_commit_re_runs_the_real_command_for_the_core_to_re_validate() {
    let state = rich();
    let (key, command) = move_row(&state);
    let mut view = focused(Section::Legacy, key);

    update(Action::Activate, &mut view, &state);
    assert!(view.armed.is_some(), "fixture: armed");

    // Between arming and committing, the clowder falls below the size a Move
    // needs. Nothing about the row changed; the world did.
    let mut lapsed = state.clone();
    lapsed.kittens.truncate(1);
    assert!(
        lapsed.clone().apply(command).is_err(),
        "fixture: the Move must genuinely be invalid now, or this proves nothing"
    );

    let out = commit_deliberately(&mut view, &lapsed);

    assert_eq!(
        out,
        Outcome::Apply(command),
        "the shell refused to commit on its own judgement; the core is the single arbiter, \
         and a commit it will reject still goes to it — the refusal is a footer, not a \
         silent swallow"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-036 — the DeepNap sequence
// ---------------------------------------------------------------------------

/// `x` → type years → Enter *arms* (showing the nap's real cost and wake-year)
/// → a second Enter commits. The count prompt's Enter is the arming press, not
/// the firing one, and the count the player typed is what finally goes to the
/// core.
///
/// @spec TUI-NAV-036
#[test]
fn the_deep_nap_count_prompts_enter_arms_and_a_second_enter_commits() {
    let state = rich();
    let (key, _) = find_row(&state, Section::Dreams, |c| matches!(c, Command::DeepNap(_)));
    let mut view = focused(Section::Dreams, key);

    assert_eq!(update(Action::CountPrompt, &mut view, &state), Outcome::Handled);
    assert!(view.prompt.is_some(), "x on the nap row opened no count prompt");

    update(Action::Type('3'), &mut view, &state);
    assert_eq!(
        view.prompt.as_ref().expect("prompt").digits,
        "3",
        "the typed year count did not reach the field"
    );

    let out = update(Action::Activate, &mut view, &state);
    assert_eq!(
        out,
        Outcome::Handled,
        "the count prompt's Enter fired the nap; TUI-NAV-036 says that press sets the count \
         and *arms*, and a second Enter commits"
    );
    assert!(view.prompt.is_none(), "the count prompt is still open after accepting it");
    let armed = view.armed.as_ref().expect("the count prompt's Enter armed nothing");
    assert_eq!(
        armed.command,
        Command::DeepNap(3),
        "the armed nap does not carry the count the player typed; rows are rebuilt from state \
         every frame, so the count has nowhere else to survive"
    );

    let out = commit_deliberately(&mut view, &state);
    assert_eq!(
        out,
        Outcome::Apply(Command::DeepNap(3)),
        "the committing Enter did not send the counted nap to the core"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-010 — repeat, rate-limited; the cursor stays put
// ---------------------------------------------------------------------------

/// Holding Enter repeats — but at the shell's rate, not the terminal's. Uniform
/// across machines is the whole point: the same held key must not be worth
/// three times as much on a box with a faster key-repeat setting.
///
/// @spec TUI-NAV-010
#[test]
fn held_enter_fires_no_faster_than_ten_times_a_second() {
    let state = rich();
    let (key, command) = find_row(&state, Section::Meadow, |c| matches!(c, Command::GatherCatnip));
    let mut view = focused(Section::Meadow, key);

    let start = Instant::now();
    let mut fired = 0u32;
    let mut presses = 0u32;
    while start.elapsed() < Duration::from_secs(1) {
        presses += 1;
        if would_fire(&update(Action::Activate, &mut view, &state), command) {
            fired += 1;
        }
    }

    assert!(
        presses > 100,
        "fixture: only {presses} presses in a second — the stream must outrun any terminal's \
         key repeat for the limit to be under test"
    );
    assert!(
        fired >= 2,
        "a held Enter fired {fired} times in a second; it must still repeat — the cursor \
         staying put is what makes holding the key the way to repeat an action"
    );
    assert!(
        fired <= 12,
        "a held Enter fired {fired} times in a second from {presses} presses; TUI-NAV-010 caps \
         it at about ten a second regardless of how fast the keys arrive"
    );
}

/// The cursor does not move on action. Acting is not navigating: the row you
/// just built is the row still under the cursor, which is what makes holding
/// Enter mean "again" rather than "and then the next one".
///
/// @spec TUI-NAV-010
#[test]
fn the_cursor_does_not_move_on_action() {
    let state = rich();
    let (key, _) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));

    for action in [Action::Activate, Action::MaxAffordable, Action::CountPrompt] {
        let mut view = focused(Section::Meadow, key.clone());
        update(action.clone(), &mut view, &state);
        assert_eq!(
            view.focus[Section::Meadow.index()].as_ref(),
            Some(&key),
            "{action:?} moved the cursor off the row it acted on"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-011 / TUI-NAV-012 — ×N and ×max
// ---------------------------------------------------------------------------

/// `x` opens a field that captures the digits of a count. While it is open the
/// digits are a number being typed, not section jumps — and nothing else leaks
/// into it.
///
/// @spec TUI-NAV-011
#[test]
fn x_opens_a_count_prompt_that_captures_digits() {
    let state = rich();
    let (key, _) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));
    let mut view = focused(Section::Meadow, key);

    assert_eq!(update(Action::CountPrompt, &mut view, &state), Outcome::Handled);
    let prompt = view.prompt.as_ref().expect("x opened no count prompt");
    assert_eq!(prompt.digits, "", "the count prompt opened with something already typed");

    update(Action::Type('1'), &mut view, &state);
    update(Action::Type('2'), &mut view, &state);
    assert_eq!(view.prompt.as_ref().expect("prompt").digits, "12");

    update(Action::Erase, &mut view, &state);
    assert_eq!(
        view.prompt.as_ref().expect("prompt").digits,
        "1",
        "Backspace did not edit the field"
    );

    // A letter is not part of a count. Whatever the field does with it, the
    // count must stay a number — and it must certainly not quit the game.
    let out = update(Action::Type('q'), &mut view, &state);
    assert_ne!(out, Outcome::Quit, "a letter typed into the count prompt quit the game");
    let digits = &view.prompt.as_ref().expect("prompt").digits;
    assert!(
        digits.chars().all(|c| c.is_ascii_digit()),
        "the count field holds {digits:?}; it captures digits, and a stray letter must not \
         corrupt the number the player is typing"
    );
}

/// For a repeat-N command the shell calls the command N times. Partial success
/// is intended: you build as many as you can afford and stop at the first
/// refusal, each call kept clean by the core's all-or-nothing spend.
///
/// @spec TUI-NAV-011, TUI-NAV-031
#[test]
fn the_count_prompts_enter_repeat_calls_a_repeat_n_command() {
    let state = rich();
    let (key, command) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));
    let mut view = focused(Section::Meadow, key);

    update(Action::CountPrompt, &mut view, &state);
    update(Action::Type('7'), &mut view, &state);
    let out = update(Action::Activate, &mut view, &state);

    assert_eq!(
        out,
        Outcome::Repeat(command, 7),
        "×7 on a repeat-N row did not ask for seven calls; passing 7 into a command that has \
         nowhere to put it would silently build one"
    );
    assert!(view.prompt.is_none(), "the count prompt stayed open after firing");
}

/// For a count-taking command the count is a real argument — squads, caravans,
/// years — so N is passed through and the core clamps or rejects. Repeat-calling
/// it seven times would send seven squads instead of one squad of seven.
///
/// @spec TUI-NAV-011, TUI-NAV-031
#[test]
fn the_count_prompts_enter_passes_the_count_to_a_count_taking_command() {
    let state = rich();
    let (key, _) = find_row(&state, Section::Clowder, |c| matches!(c, Command::Pounce(_)));
    let mut view = focused(Section::Clowder, key);

    update(Action::CountPrompt, &mut view, &state);
    update(Action::Type('4'), &mut view, &state);
    let out = update(Action::Activate, &mut view, &state);

    assert_eq!(
        out,
        Outcome::Apply(Command::Pounce(4)),
        "×4 on a count-taking row did not pass the count into the command"
    );
    assert!(view.prompt.is_none(), "the count prompt stayed open after firing");
}

/// An empty count, or a zero one, cancels. "I typed x by accident" and "×0"
/// are the same intention, and neither should cost anything.
///
/// @spec TUI-NAV-011, TUI-NAV-031
#[test]
fn an_empty_or_zero_count_cancels() {
    let state = rich();
    let (key, command) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));

    for typed in ["", "0", "00"] {
        let mut view = focused(Section::Meadow, key.clone());
        update(Action::CountPrompt, &mut view, &state);
        for c in typed.chars() {
            update(Action::Type(c), &mut view, &state);
        }
        let out = update(Action::Activate, &mut view, &state);

        assert!(
            !would_fire(&out, command),
            "a count of {typed:?} still fired the command"
        );
        assert!(
            view.prompt.is_none(),
            "a count of {typed:?} left the prompt open with nothing to accept"
        );
        assert!(view.armed.is_none(), "a cancelled count armed a row");
    }
}

/// Esc cancels the count prompt and nothing happened. Esc only ever closes the
/// topmost transient.
///
/// @spec TUI-NAV-011
#[test]
fn esc_cancels_the_count_prompt_with_no_effect() {
    let state = rich();
    let (key, command) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));
    let mut view = focused(Section::Meadow, key.clone());

    update(Action::CountPrompt, &mut view, &state);
    update(Action::Type('9'), &mut view, &state);
    let out = update(Action::Dismiss, &mut view, &state);

    assert!(!would_fire(&out, command), "Esc on the count prompt fired the command");
    assert!(view.prompt.is_none(), "Esc left the count prompt open");
    assert_eq!(
        view.focus[Section::Meadow.index()].as_ref(),
        Some(&key),
        "cancelling a count moved the cursor"
    );
}

/// `X` acts to the maximum affordable by asking for the command repeatedly,
/// under a shell-side iteration cap. The core refuses when the stores run out
/// and the run stops there — the cap is the guard against a command that never
/// refuses.
///
/// @spec TUI-NAV-012
#[test]
fn x_max_repeats_under_a_shell_side_iteration_cap() {
    let state = rich();
    let (key, command) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));
    let mut view = focused(Section::Meadow, key);

    let out = update(Action::MaxAffordable, &mut view, &state);

    match out {
        Outcome::Repeat(fired, n) => {
            assert_eq!(fired, command, "X fired a different row's command");
            assert!(n > 1, "X asked for {n} call(s); ×max is not ×1");
            assert!(
                n <= 100_000,
                "X asked for {n} calls with no meaningful cap; a run that cannot be retired \
                 inside a frame stalls the sim it is supposed to keep ticking"
            );
        }
        other => panic!(
            "X yielded {other:?}; TUI-NAV-012 asks for the command applied repeatedly until \
             the core rejects it or the cap is reached"
        ),
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-013 — count rows
// ---------------------------------------------------------------------------

/// On a count row, `+` and Enter both assign one and `-` unassigns one. Enter
/// keeps its meaning — "do the focused row's thing" — on a row whose thing is
/// "one more".
///
/// @spec TUI-NAV-013
#[test]
fn a_count_row_assigns_on_plus_or_enter_and_unassigns_on_minus() {
    let state = rich();
    let (key, increment, decrement) = find_count_row(&state, Section::Clowder);

    for action in [Action::Increment, Action::Activate] {
        let mut view = focused(Section::Clowder, key.clone());
        assert_eq!(
            update(action.clone(), &mut view, &state),
            Outcome::Apply(increment),
            "{action:?} on a count row did not assign one"
        );
    }

    let mut view = focused(Section::Clowder, key.clone());
    assert_eq!(
        update(Action::Decrement, &mut view, &state),
        Outcome::Apply(decrement),
        "- on a count row did not unassign one"
    );
    assert_eq!(
        view.focus[Section::Clowder.index()].as_ref(),
        Some(&key),
        "adjusting a count moved the cursor off the row being adjusted"
    );
}

/// `x` scales a count row: the shell calls the assign command N times, since
/// assigning takes no count of its own.
///
/// @spec TUI-NAV-011, TUI-NAV-013
#[test]
fn x_scales_a_count_row_by_repeat_calling_its_assign() {
    let state = rich();
    let (key, increment, _) = find_count_row(&state, Section::Clowder);
    let mut view = focused(Section::Clowder, key);

    update(Action::CountPrompt, &mut view, &state);
    assert!(view.prompt.is_some(), "x on a count row opened no prompt");
    update(Action::Type('3'), &mut view, &state);
    let out = update(Action::Activate, &mut view, &state);

    assert_eq!(
        out,
        Outcome::Repeat(increment, 3),
        "×3 on a count row did not ask for three assignments"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-041 — no-ops that say so
// ---------------------------------------------------------------------------

/// A one-shot you already own answers "already yours" to Enter, `x` and `X`
/// alike. Nothing fires, nothing arms, and the row still says why — a silent
/// key would read as a dropped keystroke.
///
/// @spec TUI-NAV-041
#[test]
fn enter_x_and_x_max_are_all_no_ops_on_an_owned_one_shot() {
    let state = rich();
    let owned = ContentId::Arch(ArchId::EverdreamArch);
    assert_eq!(
        state.lock_reason(owned),
        Some(LockReason::AlreadyYours),
        "fixture: this arch must be one the core reports as already owned"
    );
    assert_ne!(
        state.availability(owned),
        Availability::Hidden,
        "fixture: an owned one-shot is still a row — that is where the ✓ lives"
    );

    let (key, command) =
        find_row(&state, Section::Wonders, |c| matches!(c, Command::BuyArch(ArchId::EverdreamArch)));

    for action in [Action::Activate, Action::CountPrompt, Action::MaxAffordable] {
        let mut view = focused(Section::Wonders, key.clone());
        let out = update(action.clone(), &mut view, &state);

        assert!(
            !would_fire(&out, command),
            "{action:?} on an owned one-shot asked the core to buy it again"
        );
        assert!(view.armed.is_none(), "{action:?} on an owned one-shot armed a row");
        assert!(
            view.prompt.is_none(),
            "{action:?} on an owned one-shot opened a count prompt for a thing there is no \
             more of to have"
        );
        assert_footer_says_something_plain(&view, &format!("{action:?} on an owned one-shot"));
    }
}

/// A row whose command is neither counted nor usefully repeated has nothing for
/// `x` or `X` to do, and says so. Enter still works — the row is not owned, it
/// is merely singular.
///
/// @spec TUI-NAV-041
#[test]
fn x_and_x_max_are_no_ops_on_a_row_that_is_neither_counted_nor_repeatable() {
    let state = rich();
    let (key, command) = find_row(&state, Section::Meadow, |c| matches!(c, Command::GatherCatnip));

    for action in [Action::CountPrompt, Action::MaxAffordable] {
        let mut view = focused(Section::Meadow, key.clone());
        let out = update(action.clone(), &mut view, &state);

        assert!(
            !would_fire(&out, command),
            "{action:?} fired a row the row builder tags as neither counted nor repeatable"
        );
        assert!(
            view.prompt.is_none(),
            "{action:?} opened a count prompt on a row that cannot take a count"
        );
        assert_footer_says_something_plain(&view, &format!("{action:?} on a single-shot row"));
    }

    // …and the row is still perfectly actionable by Enter.
    let mut view = focused(Section::Meadow, key);
    assert_eq!(
        update(Action::Activate, &mut view, &state),
        Outcome::Apply(command),
        "the no-op on x/X spread to Enter; a row with no count is still a row you can do"
    );
}

/// A footer note is a note, not a dialog: it never blocks, and it never eats
/// the next keystroke. The whole reason rejections render as a footer flash is
/// that the game keeps going while one is on screen.
///
/// @spec TUI-NAV-041, TUI-NAV-014
#[test]
fn a_footer_note_never_swallows_the_next_key() {
    let state = rich();
    let (build_key, build) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));

    let mut view = focused(Section::Meadow, build_key);
    view.footer = Some((
        "not enough of something, said plainly".to_string(),
        Instant::now() + Duration::from_secs(4),
    ));

    let out = update(Action::Activate, &mut view, &state);

    assert_eq!(
        out,
        Outcome::Apply(build),
        "a footer note on screen swallowed the next Enter; a note that has to be dismissed is \
         a dialog wearing a footer's clothes"
    );
    assert!(view.overlay.is_none(), "a footer note put an overlay on screen");
}

fn assert_footer_says_something_plain(view: &ViewState, context: &str) {
    let (message, until) = view
        .footer
        .as_ref()
        .unwrap_or_else(|| panic!("{context} did nothing and said nothing — a bound key that \
             is not meaningful right now earns a footer note (TUI-NAV-041)"));

    assert!(!message.trim().is_empty(), "{context} left an empty footer note");
    assert!(
        message.contains(' '),
        "{context} left the footer note {message:?}; it is shown to a player, in words"
    );
    for artefact in ['{', '}', '(', ')'] {
        assert!(
            !message.contains(artefact),
            "{context} left the footer note {message:?}, which reads like a debug-printed \
             value rather than plain language"
        );
    }
    let left = until.saturating_duration_since(Instant::now());
    assert!(
        left > Duration::from_millis(500) && left <= Duration::from_secs(10),
        "{context} left a footer note lasting {left:?}; TUI-NAV-014 asks for a few seconds — \
         long enough to read, short enough to forget"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-016 — quitting
// ---------------------------------------------------------------------------

/// `q` quits at once. Quitting is always safe — the shell saves first and the
/// offline accrual makes the gap up — so there is nothing to confirm, and
/// nothing an armed row or an open overlay may do to delay it.
///
/// @spec TUI-NAV-016
#[test]
fn q_quits_at_once_from_anywhere_and_commits_nothing() {
    let state = rich();
    let (key, command) = move_row(&state);

    // From a bare screen.
    let mut bare = ViewState::default();
    assert_eq!(update(Action::Quit, &mut bare, &state), Outcome::Quit);

    // From inside an overlay: leaving must work from anywhere.
    let mut overlaid = focused(Section::Legacy, key.clone());
    overlaid.overlay = Some(Overlay::Help);
    assert_eq!(
        update(Action::Quit, &mut overlaid, &state),
        Outcome::Quit,
        "q inside an overlay did not quit; the overlay takes the navigation keys, not the exit"
    );

    // And from an armed heavy row: quitting must not be a confirmation.
    let mut armed = focused(Section::Legacy, key);
    update(Action::Activate, &mut armed, &state);
    assert!(armed.armed.is_some(), "fixture: armed");
    let out = update(Action::Quit, &mut armed, &state);
    assert_eq!(
        out,
        Outcome::Quit,
        "q on an armed row did something other than quit — it fired {:?}",
        commands_of(&out)
    );
    assert!(
        !would_fire(&out, command),
        "quitting committed the armed heavy action on the way out"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-008 — the cursor remembers by identity
// ---------------------------------------------------------------------------

/// Returning to a section restores the same *item*, not the same line number.
/// Progressive reveal inserts rows above the one you were on all game long; a
/// remembered index would slide the cursor onto a neighbour every time
/// something new appeared.
///
/// @spec TUI-NAV-008
#[test]
fn returning_to_a_section_restores_the_item_even_after_reveals_insert_rows_above_it() {
    let landmark = ContentId::Structure(StructureId::ALL[5]);

    let mut before = fresh();
    before.revealed.insert(landmark);

    let mut after = before.clone();
    for s in StructureId::ALL.iter().take(5) {
        after.revealed.insert(ContentId::Structure(*s));
    }

    let index_of = |state: &GameState| {
        rows(state, Section::Meadow)
            .iter()
            .position(|r| r.key == RowKey::Content(landmark, ""))
            .expect("fixture: the landmark row must be listed")
    };
    assert!(
        index_of(&after) > index_of(&before),
        "fixture: the reveals must actually push the landmark row down the list"
    );

    // The player was on the landmark, then went elsewhere; meanwhile five rows
    // appeared above it.
    let mut view = focused(Section::Legacy, RowKey::Fixed("GreatMeadowMove"));
    view.focus[Section::Meadow.index()] = Some(RowKey::Content(landmark, ""));

    update(Action::SectionJump(Section::Meadow), &mut view, &after);

    assert_eq!(view.section, Some(Section::Meadow), "the jump did not change section");
    assert_eq!(
        view.focus[Section::Meadow.index()],
        Some(RowKey::Content(landmark, "")),
        "coming back landed on a different row than the one left behind; the cursor remembers \
         an item, never a line number"
    );
}

/// Cursor memory addresses rows by identity, so within a section every row the
/// cursor can land on needs an identity of its own. Two rows sharing one
/// `RowKey` cannot both be reached: the first match wins, and the second row
/// becomes unfocusable — and with it, uninspectable.
///
/// This is the row builder's key assignment (`nav/rows.rs`), not the reducer's.
/// Surface it; do not weaken it.
///
/// @spec TUI-NAV-008
#[test]
fn every_selectable_row_in_a_section_carries_a_distinct_identity() {
    let state = rich();

    for section in Section::ALL {
        let mut seen: Vec<RowKey> = Vec::new();
        for row in rows(&state, section).into_iter().filter(|r| r.selectable()) {
            assert!(
                !seen.contains(&row.key),
                "the {} section lists two selectable rows both identified as {:?}; the cursor \
                 remembers a row by its identity, so the second one can never be focused",
                section.title(),
                row.key
            );
            seen.push(row.key);
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-032 — where the cursor lands
// ---------------------------------------------------------------------------

/// A first visit lands on the first row there is something to do on. Landing on
/// a row that is only teased would open a screen whose focused action cannot be
/// taken.
///
/// @spec TUI-NAV-032
#[test]
fn a_first_visit_lands_on_the_first_actionable_row() {
    // A meadow where the first thing on the Ideas list is revealed but not yet
    // affordable, and something further down is ready to go.
    let mut state = fresh();
    // House rules now appear on the same gate the ideas do — once the meadow
    // can hold what buys them — so the fixture needs that storage for the
    // section to contain an actionable row at all. @spec SIM-ENG-032
    state.structures[StructureId::CardboardBox.ix()] = 1;
    state.revealed.insert(ContentId::Idea(IdeaId::ALL[0]));

    let list = rows(&state, Section::Ideas);
    assert!(
        matches!(list.first().map(|r| &r.key), Some(RowKey::Content(id, "")) if state.availability(*id) == Availability::Teased),
        "fixture: the first row must be a teased one, or the landing rule is not under test"
    );
    let first_available = list
        .iter()
        .find(|r| match r.key {
            RowKey::Content(id, "") => state.availability(id) == Availability::Available,
            _ => false,
        })
        .expect("fixture: something on the list must be actionable")
        .key
        .clone();

    let mut view = ViewState::default();
    update(Action::SectionJump(Section::Ideas), &mut view, &state);

    assert_eq!(
        view.focus[Section::Ideas.index()].as_ref(),
        Some(&first_available),
        "a first visit landed somewhere other than the first row the player can act on"
    );
}

/// When nothing on the list is actionable, the cursor takes the first row
/// anyway — a section is never entered with no cursor at all.
///
/// @spec TUI-NAV-032
#[test]
fn a_section_with_nothing_actionable_lands_on_its_first_row() {
    let state = fresh();
    let first = rows(&state, Section::Meadow)
        .first()
        .expect("fixture: a new meadow has rows")
        .key
        .clone();

    let mut view = ViewState::default();
    update(Action::SectionJump(Section::Meadow), &mut view, &state);

    assert_eq!(
        view.focus[Section::Meadow.index()].as_ref(),
        Some(&first),
        "entering a section with nothing affordable left the cursor somewhere unexpected"
    );
}

/// A remembered item that is no longer revealed — structures reset by a Move,
/// say — falls back to the landing rule rather than to a cursor pointing at
/// nothing.
///
/// @spec TUI-NAV-032
#[test]
fn a_forgotten_item_falls_back_to_the_landing_rule() {
    let state = rich();
    let listed: Vec<RowKey> = rows(&state, Section::Ideas)
        .into_iter()
        .filter(|r| r.selectable())
        .map(|r| r.key)
        .collect();

    let mut view = ViewState::default();
    // A memory of something that is not on this list at all.
    view.focus[Section::Ideas.index()] = Some(RowKey::Fixed("a row that went away"));

    update(Action::SectionJump(Section::Ideas), &mut view, &state);

    let landed = view.focus[Section::Ideas.index()]
        .as_ref()
        .expect("entering a section with a stale memory left no cursor at all");
    assert!(
        listed.contains(landed),
        "the cursor kept a memory of {landed:?}, which is no longer a row here; a remembered \
         item that has gone falls back to the landing rule"
    );
}

/// The cursor never lands on a group header. Headers are signposts: `[` and `]`
/// jump to them, but Enter on one has nothing to do.
///
/// @spec TUI-NAV-032
#[test]
fn the_cursor_never_lands_on_an_unselectable_row() {
    for (label, state) in [("a new meadow", fresh()), ("a maximal meadow", rich())] {
        for section in Section::ALL {
            let list = rows(&state, section);
            if list.iter().all(|r| !r.selectable()) {
                continue;
            }

            let mut view = ViewState::default();
            update(Action::SectionJump(section), &mut view, &state);

            let Some(landed) = view.focus[section.index()].clone() else {
                continue;
            };
            let row = list
                .iter()
                .find(|r| r.key == landed)
                .unwrap_or_else(|| panic!(
                    "in {label}, entering {} focused {landed:?}, which is not a row on that list",
                    section.title()
                ));
            assert!(
                row.selectable(),
                "in {label}, entering {} landed the cursor on {landed:?}, a row the cursor \
                 cannot act on",
                section.title()
            );
        }
    }
}
