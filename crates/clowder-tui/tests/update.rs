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
    KittenTrait, LockReason, Location, ResourceId, RiteId, RuleId, SiteId, StructureId,
};

use clowder_tui::nav::keys::resolve;
use clowder_tui::nav::rows::{rows, Row, RowKind};
use clowder_tui::nav::update::MAX_AFFORDABLE_CAP;
use clowder_tui::nav::{
    update, Action, ArmedRow, CountSemantics, Outcome, Overlay, PendingReport, RowKey, Section,
    ViewState,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
    let mut v = ViewState { section: Some(section), ..Default::default() };
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

/// The nap row, focused, in a meadow where a nap is open and affordable.
fn nap_view(state: &GameState) -> (RowKey, ViewState) {
    let (key, _) = find_row(state, Section::Dreams, |c| matches!(c, Command::DeepNap(_)));
    let view = focused(Section::Dreams, key.clone());
    (key, view)
}

/// `x`, the digits, then the prompt's Enter — one whole count-prompt acceptance.
fn accept_count(digits: &str, view: &mut ViewState, state: &GameState) -> Outcome {
    assert_eq!(update(Action::CountPrompt, view, state), Outcome::Handled);
    assert!(view.prompt.is_some(), "fixture: x on the nap row opened no count prompt");
    for c in digits.chars() {
        update(Action::Type(c), view, state);
    }
    update(Action::Activate, view, state)
}

/// Enter is a row's action ×1, and one unit of a nap is one year: a plain
/// Enter on the unarmed nap row arms a one-year nap — it neither opens the
/// prompt nor fires.
///
/// @spec TUI-NAV-036
#[test]
fn a_plain_enter_on_the_unarmed_counted_heavy_row_arms_one_unit() {
    let state = rich();
    let (key, mut view) = nap_view(&state);

    let out = update(Action::Activate, &mut view, &state);

    assert_eq!(out, Outcome::Handled, "a plain Enter on the unarmed nap row reached the core");
    assert!(view.prompt.is_none(), "a plain Enter on the nap row opened a count prompt");
    let armed = view.armed.as_ref().expect("a plain Enter on the nap row armed nothing");
    assert_eq!(armed.key, key, "the armed row is not the nap row");
    assert_eq!(
        armed.command,
        Command::DeepNap(1),
        "a plain Enter armed a nap of some length other than one year"
    );
}

/// Accepting a new nonzero count over an already-armed nap re-arms it with
/// that count and a fresh expiry, so the summary on screen is always the nap
/// the next Enter commits — and that next Enter commits exactly it.
///
/// @spec TUI-NAV-036
#[test]
fn a_new_count_on_an_armed_counted_row_re_arms_it_with_that_count_and_a_fresh_expiry() {
    let state = rich();
    let (key, mut view) = nap_view(&state);

    assert_eq!(accept_count("3", &mut view, &state), Outcome::Handled);
    assert_eq!(view.armed.as_ref().map(|a| a.command), Some(Command::DeepNap(3)));
    // Age the first arming so a renewed expiry is distinguishable from it.
    view.armed.as_mut().expect("armed").expires_at = Instant::now() + Duration::from_secs(1);

    std::thread::sleep(Duration::from_millis(300));
    let out = accept_count("7", &mut view, &state);

    assert_eq!(out, Outcome::Handled, "re-accepting a count over an armed nap fired it");
    let armed = view.armed.as_ref().expect("re-accepting a count left the nap row unarmed");
    assert_eq!(armed.key, key);
    assert_eq!(
        armed.command,
        Command::DeepNap(7),
        "the re-armed nap still carries the old count; the summary would describe a nap the \
         next Enter does not commit"
    );
    let left = armed.expires_at.saturating_duration_since(Instant::now());
    assert!(
        left >= Duration::from_secs(7),
        "the re-armed nap kept its old expiry ({left:?} left); a re-arming starts a fresh one"
    );

    let out = commit_deliberately(&mut view, &state);
    assert_eq!(
        out,
        Outcome::Apply(Command::DeepNap(7)),
        "the commit after a re-arming sent something other than the count it was re-armed with"
    );
}

/// An empty or zero count arms nothing, and leaves an arming already on the
/// row exactly as it stood — its count and its expiry both.
///
/// @spec TUI-NAV-036
#[test]
fn an_empty_or_zero_count_arms_nothing_and_leaves_any_arming_as_it_stood() {
    let state = rich();

    for digits in ["", "0", "00"] {
        let (_, mut view) = nap_view(&state);
        let out = accept_count(digits, &mut view, &state);
        assert_eq!(out, Outcome::Handled, "a count of {digits:?} fired the nap");
        assert!(view.armed.is_none(), "a count of {digits:?} armed the unarmed nap row");

        let (_, mut view) = nap_view(&state);
        assert_eq!(accept_count("5", &mut view, &state), Outcome::Handled);
        let before = view.armed.clone().expect("fixture: a count of 5 arms the nap");
        std::thread::sleep(Duration::from_millis(20));

        let out = accept_count(digits, &mut view, &state);
        assert_eq!(out, Outcome::Handled, "a count of {digits:?} over an armed nap fired it");
        assert_eq!(
            view.armed.as_ref(),
            Some(&before),
            "a count of {digits:?} changed the arming already on the nap row; it leaves any \
             arming as it stood"
        );
    }
}

/// The nap commit re-runs the real command with the armed count, so the core
/// re-validates it: a count the core will refuse by now still goes to it, and
/// the refusal is the core's to make.
///
/// @spec TUI-NAV-036
#[test]
fn the_counted_commit_re_runs_the_real_command_with_the_armed_count() {
    let state = rich();
    let (_, mut view) = nap_view(&state);
    assert_eq!(accept_count("4", &mut view, &state), Outcome::Handled);

    let mut broke = state.clone();
    broke.resources[ResourceId::Starshards.ix()] = 0.0;
    assert!(
        broke.clone().apply(Command::DeepNap(4)).is_err(),
        "fixture: the armed nap must be one the core now refuses"
    );

    let out = commit_deliberately(&mut view, &broke);
    assert_eq!(
        out,
        Outcome::Apply(Command::DeepNap(4)),
        "the nap commit was judged in the shell rather than re-run through the core"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-076 — what ends an arming, and what leaves it running
// ---------------------------------------------------------------------------

/// The expiry is about eight seconds of wall-clock time whatever the game
/// speed — the confirm is for the player's reading, not the sim's pace.
///
/// @spec TUI-NAV-076
#[test]
fn the_arming_expiry_is_wall_clock_time_at_every_game_speed() {
    for speed in [0.25, 0.5, 1.0, 2.0, 4.0, 8.0] {
        let mut state = rich();
        state.speed = speed;
        for (name, section, key, _) in heavy_rows(&state) {
            let mut view = focused(section, key);
            update(Action::Activate, &mut view, &state);
            let armed = view.armed.as_ref().unwrap_or_else(|| panic!("{name} did not arm"));
            let left = armed.expires_at.saturating_duration_since(Instant::now());
            assert!(
                left >= Duration::from_secs(7) && left <= Duration::from_secs(9),
                "at speed ×{speed} the {name} arming expires in {left:?}; it is about eight \
                 wall-clock seconds whatever the game speed"
            );
        }
    }
}

/// Every navigation key on the screen ends an arming, silently — whether or
/// not it moves the cursor or the section. The armed row here is the first
/// row of its list, so ↑ moves nothing; a digit for the section already
/// showing moves nothing either.
///
/// @spec TUI-NAV-076
#[test]
fn every_navigation_key_ends_an_arming_silently_whether_or_not_it_moves_anything() {
    let state = rich();
    let (row_key, command) = move_row(&state);
    assert_eq!(
        rows(&state, Section::Legacy).iter().position(|r| r.selectable()),
        rows(&state, Section::Legacy).iter().position(|r| r.key == row_key),
        "fixture: the armed row must be the first selectable row, so an ↑ moves nothing"
    );
    let revealed = clowder_tui::nav::rows::revealed_sections(&state);

    let mut nav_keys: Vec<KeyEvent> = vec![
        key(KeyCode::Up),
        key(KeyCode::Down),
        key(KeyCode::Char('k')),
        key(KeyCode::Char('j')),
        key(KeyCode::Tab),
        key(KeyCode::BackTab),
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
        key(KeyCode::Char('l')),
        key(KeyCode::Char('h')),
        key(KeyCode::Left),
        key(KeyCode::Right),
        key(KeyCode::Char('[')),
        key(KeyCode::Char(']')),
    ];
    nav_keys.extend(Section::ALL.iter().map(|s| key(KeyCode::Char(s.digit()))));

    for k in nav_keys {
        let mut view = focused(Section::Legacy, row_key.clone());
        update(Action::Activate, &mut view, &state);
        assert!(view.armed.is_some(), "fixture: the row must be armed before {k:?}");

        let action = resolve(k, &view);
        let out = update(action.clone(), &mut view, &state);

        assert!(!would_fire(&out, command), "{k:?} ({action:?}) committed the armed row");
        assert!(
            view.armed.is_none(),
            "{k:?} ({action:?}) left the row armed; any navigation key ends an arming, whether \
             or not it moves anything"
        );
        let unrevealed_digit = matches!(action, Action::SectionJump(s) if !revealed.contains(&s));
        if !unrevealed_digit {
            assert!(
                view.footer.is_none(),
                "{k:?} ({action:?}) ended the arming with a footer note {:?}; an arming ends \
                 silently",
                view.footer
            );
        }
    }
}

/// The palette's jump is navigation, so it ends an arming.
///
/// @spec TUI-NAV-076
#[test]
fn the_palettes_jump_ends_an_arming() {
    let state = rich();
    let (row_key, command) = move_row(&state);
    let mut view = focused(Section::Legacy, row_key);
    update(Action::Activate, &mut view, &state);
    assert!(view.armed.is_some(), "fixture: armed");

    press_char('/', &mut view, &state);
    assert!(matches!(view.overlay, Some(Overlay::Palette { .. })), "fixture: palette open");
    assert!(view.armed.is_some(), "opening the palette ended the arming");
    let query = "a";
    assert!(
        !clowder_tui::nav::palette::results(&state, query).is_empty(),
        "fixture: the query must aim the palette at something"
    );
    type_text(query, &mut view, &state);

    let out = press(key(KeyCode::Enter), &mut view, &state);

    assert!(!would_fire(&out, command), "the palette's jump committed the armed row");
    assert!(view.overlay.is_none(), "fixture: the palette's Enter did not jump");
    assert!(view.armed.is_none(), "the palette's jump left the row armed; the jump is navigation");
}

/// Esc with nothing above the armed row closes the arming, silently.
///
/// @spec TUI-NAV-076
#[test]
fn esc_closing_an_arming_ends_it_silently() {
    let state = rich();
    for (name, section, row_key, command) in heavy_rows(&state) {
        let mut view = focused(section, row_key);
        update(Action::Activate, &mut view, &state);
        assert!(view.armed.is_some(), "fixture: {name} armed");

        let out = press(key(KeyCode::Esc), &mut view, &state);

        assert!(!would_fire(&out, command), "Esc committed the armed {name} row");
        assert!(view.armed.is_none(), "Esc left the {name} row armed");
        assert!(view.footer.is_none(), "Esc on the armed {name} row left a footer note");
    }
}

/// A commit ends the arming whether the core accepts it or refuses it — so a
/// refused commit needs a fresh arming, and the next Enter is a first press.
///
/// @spec TUI-NAV-076
#[test]
fn a_commit_ends_the_arming_whether_the_core_accepts_or_refuses_it() {
    let state = rich();
    let (key, command) = move_row(&state);

    let mut lapsed = state.clone();
    lapsed.kittens.truncate(1);
    assert!(lapsed.clone().apply(command).is_err(), "fixture: the Move is refused now");
    assert!(state.clone().apply(command).is_ok(), "fixture: the Move is accepted here");

    for (which, world) in [("accepted", &state), ("refused", &lapsed)] {
        let mut view = focused(Section::Legacy, key.clone());
        update(Action::Activate, &mut view, world);
        let out = commit_deliberately(&mut view, world);
        assert_eq!(out, Outcome::Apply(command), "fixture: the {which} commit reached the core");
        assert!(view.armed.is_none(), "an {which} commit left the row armed");

        std::thread::sleep(Duration::from_millis(300));
        let out = update(Action::Activate, &mut view, world);
        assert!(
            !would_fire(&out, command),
            "the Enter after an {which} commit committed again; that press is a first press"
        );
        assert!(view.armed.is_some(), "the Enter after an {which} commit did not arm afresh");
    }
}

/// The arming of another row ends the first — through the count prompt as
/// well as through a plain Enter.
///
/// @spec TUI-NAV-076
#[test]
fn arming_another_row_ends_the_first_arming() {
    let state = rich();
    let (move_key, move_command) = move_row(&state);
    let (nap_key, _) = find_row(&state, Section::Dreams, |c| matches!(c, Command::DeepNap(_)));

    let mut view = focused(Section::Dreams, nap_key.clone());
    view.armed = Some(ArmedRow {
        key: move_key,
        command: move_command,
        expires_at: Instant::now() + Duration::from_secs(8),
    });
    assert_eq!(accept_count("2", &mut view, &state), Outcome::Handled);

    let armed = view.armed.as_ref().expect("the nap prompt's Enter armed nothing");
    assert_eq!(armed.key, nap_key, "arming the nap row left another row's arming standing");
    assert_eq!(armed.command, Command::DeepNap(2));
}

/// Nothing but the endings ends an arming. Toggling the inspector, opening the
/// Journal, help, the palette or a count prompt (and closing them again with
/// Esc), and the speed keys all leave the row armed with the same expiry
/// running.
///
/// @spec TUI-NAV-076
#[test]
fn opening_things_and_the_speed_keys_leave_an_arming_running() {
    let state = rich();
    let (nap_key, _) = find_row(&state, Section::Dreams, |c| matches!(c, Command::DeepNap(_)));

    let armed_view = || {
        let mut view = focused(Section::Dreams, nap_key.clone());
        update(Action::Activate, &mut view, &state);
        let armed = view.armed.clone().expect("fixture: the nap row arms");
        (view, armed)
    };

    // Each opener, then (where it opened a transient) the Esc that closes it.
    let openers: [(char, bool); 8] = [
        (' ', false),
        ('i', false),
        ('e', true),
        ('?', true),
        ('/', true),
        ('x', true),
        ('<', false),
        ('>', false),
    ];
    for (c, then_esc) in openers {
        let (mut view, before) = armed_view();
        std::thread::sleep(Duration::from_millis(5));

        press_char(c, &mut view, &state);
        assert_eq!(
            view.armed.as_ref(),
            Some(&before),
            "{c:?} changed or ended the arming; only navigation, Esc on the arming, a commit, \
             another arming, the expiry, or the row leaving its list may"
        );
        if c == 'x' {
            assert!(view.prompt.is_some(), "fixture: x on the armed nap row opened no prompt");
        }

        if then_esc {
            press(key(KeyCode::Esc), &mut view, &state);
            assert!(
                view.prompt.is_none() && view.overlay.is_none(),
                "fixture: Esc did not close what {c:?} opened"
            );
            assert_eq!(
                view.armed.as_ref(),
                Some(&before),
                "Esc closing what {c:?} opened also ended the arming beneath it"
            );
        }
    }
}

/// The cursor and group keys an open Journal or help overlay takes for its
/// own scrolling are the overlay's — their navigation meaning suspended — so
/// scrolling there ends nothing: the row beneath stays armed, its expiry
/// still running.
///
/// @spec TUI-NAV-076
#[test]
fn scrolling_an_open_journal_or_help_overlay_leaves_the_row_beneath_armed() {
    let state = rich();
    let (key_, _) = move_row(&state);

    let scroll_keys = [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Char('k'),
        KeyCode::Char('j'),
        KeyCode::Char('['),
        KeyCode::Char(']'),
    ];
    for overlay in [Overlay::Journal { anchor: 0 }, Overlay::Help] {
        for code in scroll_keys {
            let mut view = focused(Section::Legacy, key_.clone());
            update(Action::Activate, &mut view, &state);
            let before = view.armed.clone().expect("fixture: armed");
            view.overlay = Some(overlay.clone());

            press(key(code), &mut view, &state);

            assert_eq!(
                view.armed.as_ref(),
                Some(&before),
                "{code:?} inside {overlay:?} ended the arming beneath it; the overlay's scroll \
                 keys are the overlay's, not navigation"
            );
        }
    }
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
// TUI-NAV-046 / TUI-NAV-081 — the per-screen mnemonics
//
// A mnemonic is a lowercase letter the global table leaves unbound, standing
// for one row listed on its screen. These tests drive it as a keypress — the
// key resolved against the view, then folded — because whether a letter is
// live depends on the rows current state lists and on the item under the
// cursor, and a claim about "a letter that is not live is an unbound key"
// is a claim about the whole path from key to outcome, whichever half of it
// decides.
//
// The oracle below is the design's mnemonic table restated: a fixed
// mnemonic for each of the Meadow's gather, forage, and rummage rows, live
// while its row is listed; and the focused-item mnemonics — the focused
// kitten's promote and Elder rows on the Clowder screen, the focused colony's
// pen-pal row on the Friends screen — live while the cursor is on a row of
// that item.
// ---------------------------------------------------------------------------

/// Every key the global table binds among the printables: the reserved set
/// no mnemonic may take (Space included). Anything outside it and outside the
/// live mnemonics is an unbound key. `n` is reserved here too, though not a
/// mnemonic (TUI-NAV-046): it is the row-bound key TUI-NAV-094 states, whose
/// own tests cover it in full.
const GLOBAL_PRINTABLES: &str = "hjklixeoqnX0123456789[]+=-/<>? ";

/// The Meadow's fixed mnemonics, in their rows' list order.
const FIXED_MNEMONICS: [(char, &str); 3] =
    [('g', "GatherCatnip"), ('f', "Forage"), ('r', "Rummage")];

/// The mnemonics live in `section` with `focus` under the cursor, each with
/// the row it stands for — the design's table, as a function of state.
fn live_mnemonics(
    state: &GameState,
    section: Section,
    focus: Option<&RowKey>,
) -> Vec<(char, RowKey)> {
    let listed = rows(state, section);
    let is_listed = |k: &RowKey| listed.iter().any(|r| &r.key == k);
    let mut out = Vec::new();
    match section {
        Section::Meadow => {
            for (c, tag) in FIXED_MNEMONICS {
                let key = RowKey::Fixed(tag);
                if is_listed(&key) {
                    out.push((c, key));
                }
            }
        }
        Section::Clowder => {
            // A kitten is one row (TUI-NAV-093); `p` stands for that very
            // row. `n` is bound there too, but as the row-bound key
            // TUI-NAV-094 states, not as a mnemonic (TUI-NAV-046) — so it is
            // never in this list.
            if let Some(key @ RowKey::Kitten(..)) = focus {
                if is_listed(key) {
                    out.push(('p', key.clone()));
                }
            }
        }
        Section::Friends => {
            if let Some(RowKey::Colony(colony, _)) = focus {
                let key = RowKey::Colony(*colony, "penpal");
                if is_listed(&key) {
                    out.push(('p', key));
                }
            }
        }
        _ => {}
    }
    out
}

/// Every context a mnemonic can be pressed in, in a state: each revealed
/// section with each of its selectable rows under the cursor, and with no row
/// focused at all.
fn mnemonic_contexts(state: &GameState) -> Vec<(Section, Option<RowKey>)> {
    let mut out = Vec::new();
    for section in Section::ALL {
        let listed = rows(state, section);
        if listed.is_empty() {
            continue;
        }
        out.push((section, None));
        for r in listed.into_iter().filter(|r| r.selectable()) {
            out.push((section, Some(r.key)));
        }
    }
    out
}

/// A view on `section` with `focus` under the cursor — and an unread report
/// banner up and the inspector asked for, so "the press is not navigation and
/// leaves the banner and the layout as they stood" has something to leave.
fn mnemonic_view(section: Section, focus: Option<RowKey>) -> ViewState {
    let mut v = ViewState { section: Some(section), ..Default::default() };
    v.focus[section.index()] = focus;
    v.reports = vec![PendingReport { entry: 0, dismissed: false }];
    v.inspector_wanted = true;
    v
}

fn footer_text(view: &ViewState) -> Option<String> {
    view.footer.as_ref().map(|(m, _)| m.clone())
}

/// Everything the view holds, as text, less the Enter clock and the footer —
/// the footer is compared on its own, against what Enter on the row says.
fn observable_but_footer(view: &ViewState) -> String {
    let mut v = view.clone();
    v.last_repeat = None;
    v.footer = None;
    format!("{v:?}")
}

/// What Enter on `target`, in `view`'s section, does: the outcome, and the
/// footer it leaves. The oracle a live mnemonic is held to.
fn enter_on(view: &ViewState, target: &RowKey, state: &GameState) -> (Outcome, Option<String>) {
    let mut v = view.clone();
    let section = v.section.expect("a mnemonic view has a section");
    v.focus[section.index()] = Some(target.clone());
    let out = press(key(KeyCode::Enter), &mut v, state);
    (out, footer_text(&v))
}

fn describe(section: Section, focus: &Option<RowKey>) -> String {
    match focus {
        Some(k) => format!("on the {} screen with {k:?} focused", section.title()),
        None => format!("on the {} screen with no row focused", section.title()),
    }
}

/// The fixture the mnemonic sweeps stand on: a meadow where every mnemonic in
/// the table has its row listed somewhere, and none of them is heavy.
fn assert_mnemonic_fixture(state: &GameState) {
    let meadow = rows(state, Section::Meadow);
    for (c, tag) in FIXED_MNEMONICS {
        assert!(
            meadow.iter().any(|r| r.key == RowKey::Fixed(tag)),
            "fixture: the Meadow row `{c}` stands for is not listed"
        );
    }
    assert!(
        rows(state, Section::Clowder).iter().any(|r| matches!(r.key, RowKey::Kitten(..))),
        "fixture: no kitten rows are listed on the Clowder screen"
    );
    assert!(
        rows(state, Section::Friends).iter().any(|r| matches!(r.key, RowKey::Colony(..))),
        "fixture: no colony rows are listed on the Friends screen"
    );
    for (section, focus) in mnemonic_contexts(state) {
        for (c, target) in live_mnemonics(state, section, focus.as_ref()) {
            let row = rows(state, section)
                .into_iter()
                .find(|r| r.key == target)
                .expect("a live mnemonic's row is listed by construction");
            assert!(
                !row.heavy,
                "fixture: `{c}` {} stands for the heavy row {target:?}; a heavy row never \
                 carries a mnemonic",
                describe(section, &focus)
            );
        }
    }
}

/// The whole of the binding rule, swept over every printable key in every
/// context a revealed screen offers, in a meadow with every mnemonic's row
/// listed and in a new one where the rummage row is not yet: a key outside the
/// global table acts exactly where it is a live mnemonic — a lowercase letter
/// standing for one row listed on its screen, a fixed one while its row is
/// listed, a focused-item one while the cursor is on a row of its item — and
/// everywhere else it is an unbound key, ignored with nothing changed and no
/// footer note. An uppercase letter is never a mnemonic, so Caps Lock never
/// turns one into anything.
///
/// @spec TUI-NAV-046
#[test]
fn a_letter_acts_exactly_where_it_is_a_live_mnemonic_and_is_silent_everywhere_else() {
    let mut live_seen = 0u32;
    for (label, state) in [("a meadow far along", rich()), ("a new meadow", fresh())] {
        if label == "a meadow far along" {
            assert_mnemonic_fixture(&state);
        } else {
            assert!(
                !rows(&state, Section::Meadow).iter().any(|r| r.key == RowKey::Fixed("Rummage")),
                "fixture: a new meadow does not yet list the rummage row"
            );
        }

        for (section, focus) in mnemonic_contexts(&state) {
            let live = live_mnemonics(&state, section, focus.as_ref());
            for c in (' '..='~').filter(|c| !GLOBAL_PRINTABLES.contains(*c)) {
                // On the Ideas screen `d` is the done key, which TUI-NAV-046
                // names as one of the two letters a screen binds that are not
                // mnemonics; its own tests (TUI-NAV-115) cover it there.
                if c == 'd' && section == Section::Ideas {
                    continue;
                }
                let before = mnemonic_view(section, focus.clone());
                let mut view = before.clone();
                let out = press_char(c, &mut view, &state);
                let whr = format!("in {label}, {}", describe(section, &focus));

                match live.iter().find(|(m, _)| *m == c) {
                    Some((_, target)) => {
                        live_seen += 1;
                        let (enter_out, enter_footer) = enter_on(&before, target, &state);
                        assert_eq!(
                            out, enter_out,
                            "{whr}, the live mnemonic {c:?} answered {out:?}, but Enter on the \
                             row it stands for, {target:?}, answers {enter_out:?}; a mnemonic \
                             performs its row's primary action exactly as Enter on that row would"
                        );
                        assert_eq!(
                            footer_text(&view),
                            enter_footer,
                            "{whr}, the live mnemonic {c:?} left a different footer than Enter on \
                             {target:?} does"
                        );
                    }
                    None => {
                        assert_eq!(
                            out,
                            Outcome::Handled,
                            "{whr}, {c:?} is no live mnemonic and yet told the core {out:?}"
                        );
                        assert_eq!(
                            footer_text(&view),
                            None,
                            "{whr}, {c:?} is no live mnemonic and yet earned a footer note; a \
                             letter that is not live is an unbound key, ignored in silence"
                        );
                        assert_eq!(
                            observable_but_footer(&view),
                            observable_but_footer(&before),
                            "{whr}, {c:?} is no live mnemonic and yet changed the view"
                        );
                    }
                }
            }
        }
    }
    assert!(live_seen > 0, "fixture: the sweep never met a live mnemonic");
}

/// A mnemonic is bound only where the global key table is. An open field takes
/// its letter as text — the palette into its query, the count prompt as a
/// non-digit it ignores — and a full-screen overlay suspends it with the other
/// action keys. Quantified over every live mnemonic in every context of a
/// meadow where every one of them has its row, under each of the four things
/// that can hold the keyboard.
///
/// @spec TUI-NAV-046
#[test]
fn a_mnemonic_never_acts_while_a_field_or_full_screen_overlay_holds_the_keyboard() {
    let state = rich();
    assert_mnemonic_fixture(&state);
    let (prompt_row, _) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));

    type Holder = (&'static str, fn(&mut ViewState, RowKey));
    let holders: [Holder; 4] = [
        ("the palette", |v, _| v.overlay = Some(Overlay::Palette { query: String::new(), selected: 0 })),
        ("the count prompt", |v, row| {
            v.prompt = Some(clowder_tui::nav::CountPrompt { digits: String::new(), row: Some(row) })
        }),
        ("the Journal", |v, _| v.overlay = Some(Overlay::Journal { anchor: 0 })),
        ("the help overlay", |v, _| v.overlay = Some(Overlay::Help)),
    ];

    let mut checked = 0u32;
    for (section, focus) in mnemonic_contexts(&state) {
        for (c, target) in live_mnemonics(&state, section, focus.as_ref()) {
            for (holder, open) in holders {
                let mut before = mnemonic_view(section, focus.clone());
                open(&mut before, prompt_row.clone());
                let mut view = before.clone();
                let out = press_char(c, &mut view, &state);
                let whr = format!("{} with {holder} open", describe(section, &focus));

                assert_eq!(
                    out,
                    Outcome::Handled,
                    "{whr}, the mnemonic {c:?} (for {target:?}) told the core {out:?}; a \
                     mnemonic is bound only where the global key table applies"
                );
                let mut expected = before.clone();
                if let Some(Overlay::Palette { query, .. }) = expected.overlay.as_mut() {
                    query.push(c);
                }
                assert_eq!(
                    observable_but_footer(&view),
                    observable_but_footer(&expected),
                    "{whr}, the mnemonic {c:?} did something other than what {holder} makes of \
                     the letter: the palette takes it as query text, and the count prompt and \
                     a full-screen overlay leave it unbound"
                );
                assert_eq!(footer_text(&view), None, "{whr}, the letter {c:?} earned a footer note");
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "fixture: no live mnemonic was checked");
}

/// A mnemonic is an act, not navigation: it moves neither the cursor nor the
/// focused section, so the report banner stays up, the inspector stays asked
/// for, and nothing about where the player stands changes — the view after
/// the press is the view before it, but for what Enter on its row would have
/// left in the footer. Quantified over every live mnemonic in every context.
///
/// @spec TUI-NAV-081
#[test]
fn a_mnemonic_press_acts_in_place_and_leaves_cursor_section_banner_and_layout_as_they_stood() {
    let state = rich();
    assert_mnemonic_fixture(&state);

    let mut checked = 0u32;
    for (section, focus) in mnemonic_contexts(&state) {
        for (c, target) in live_mnemonics(&state, section, focus.as_ref()) {
            let before = mnemonic_view(section, focus.clone());
            let mut view = before.clone();
            let out = press_char(c, &mut view, &state);
            let whr = describe(section, &focus);

            let (enter_out, _) = enter_on(&before, &target, &state);
            assert_eq!(
                out, enter_out,
                "{whr}, the mnemonic {c:?} answered {out:?} where Enter on {target:?} answers \
                 {enter_out:?}"
            );
            assert!(
                commands_of(&out).is_some(),
                "fixture: {whr}, Enter on {target:?} asks the core for nothing, so this press \
                 proves nothing about acting in place"
            );
            assert_eq!(view.section, Some(section), "{whr}, the mnemonic {c:?} changed the focused section");
            assert_eq!(
                view.focus, before.focus,
                "{whr}, the mnemonic {c:?} moved the cursor; a mnemonic acts on its row in place"
            );
            assert_eq!(
                view.reports, before.reports,
                "{whr}, the mnemonic {c:?} dismissed the report banner; the press is an act, not \
                 navigation"
            );
            assert_eq!(
                view.inspector_wanted, before.inspector_wanted,
                "{whr}, the mnemonic {c:?} changed the layout"
            );
            assert_eq!(
                observable_but_footer(&view),
                observable_but_footer(&before),
                "{whr}, the mnemonic {c:?} changed the view beyond acting on {target:?}"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "fixture: no live mnemonic was checked");
}

/// One context per mnemonic in the table: each fixed letter with the cursor on
/// a row that is not its own, and each focused-item letter with the cursor on
/// its item's other row.
fn one_context_per_mnemonic(state: &GameState) -> Vec<(char, Section, RowKey, RowKey)> {
    let (build, _) = find_row(state, Section::Meadow, |c| matches!(c, Command::Build(_)));
    let mut out: Vec<(char, Section, RowKey, RowKey)> = FIXED_MNEMONICS
        .iter()
        .map(|(c, tag)| (*c, Section::Meadow, build.clone(), RowKey::Fixed(tag)))
        .collect();
    // A kitten is one row (TUI-NAV-093): its own `p` is always live only on
    // its own row, so there is no other row of the same item to focus it
    // from, and `n` is not a mnemonic at all (TUI-NAV-046, TUI-NAV-094) — so
    // neither belongs in this table.
    let colony = rows(state, Section::Friends)
        .into_iter()
        .find_map(|r| match r.key {
            RowKey::Colony(c, _) => Some(c),
            _ => None,
        })
        .expect("fixture: a colony is listed on the Friends screen");
    out.push(('p', Section::Friends, RowKey::Colony(colony, "trade"), RowKey::Colony(colony, "penpal")));
    for (c, section, focus, target) in &out {
        assert_eq!(
            live_mnemonics(state, *section, Some(focus)).iter().find(|(m, _)| m == c).map(|(_, t)| t),
            Some(target),
            "fixture: `{c}` is not live for {target:?} with {focus:?} focused"
        );
    }
    out
}

/// Hold a mnemonic for a second, one press every `pace` (or as fast as the
/// loop runs), and return the instants it acted at.
fn hold_mnemonic(
    c: char,
    section: Section,
    focus: &RowKey,
    target: &RowKey,
    pace: Option<Duration>,
    state: &GameState,
) -> (u32, Vec<Instant>) {
    let mut view = focused(section, focus.clone());
    let (primary, _) = enter_on(&view, target, state);
    let primary = commands_of(&primary).expect("fixture: Enter on the mnemonic's row asks for a command");
    let start = Instant::now();
    let mut presses = 0u32;
    let mut fired = Vec::new();
    while start.elapsed() < Duration::from_secs(1) {
        presses += 1;
        if would_fire(&press_char(c, &mut view, state), primary) {
            fired.push(Instant::now());
        }
        if let Some(p) = pace {
            std::thread::sleep(p);
        }
    }
    (presses, fired)
}

fn assert_no_faster_than_ten_a_second(fired: &[Instant], what: &str) {
    assert!(
        fired.len() >= 2,
        "{what}: a held mnemonic acted {} times in a second; held, it must still repeat, as a \
         held Enter does",
        fired.len()
    );
    assert!(
        fired.len() <= 11,
        "{what}: a held mnemonic acted {} times in a second; it acts no more than ten times a \
         second",
        fired.len()
    );
    for pair in fired.windows(2) {
        let gap = pair[1].duration_since(pair[0]);
        assert!(
            gap >= Duration::from_millis(95),
            "{what}: a held mnemonic acted twice {gap:?} apart; it acts no more than ten times a \
             second, the pace a held Enter keeps"
        );
    }
}

/// Held or auto-repeating, a mnemonic acts no more than ten times a second —
/// the pace a held Enter keeps — for every mnemonic in the table, under a
/// stream of presses far denser than any terminal's key repeat.
///
/// @spec TUI-NAV-081
#[test]
fn a_held_mnemonic_acts_no_more_than_ten_times_a_second() {
    let state = rich();
    assert_mnemonic_fixture(&state);
    for (c, section, focus, target) in one_context_per_mnemonic(&state) {
        let (presses, fired) = hold_mnemonic(c, section, &focus, &target, None, &state);
        assert!(
            presses > 100,
            "fixture: only {presses} presses in a second — the stream must outrun any terminal's \
             key repeat for the limit to be under test"
        );
        assert_no_faster_than_ten_a_second(
            &fired,
            &format!("`{c}` on the {} screen for {target:?}", section.title()),
        );
    }
}

/// The same pace whatever the terminal's key-repeat rate: a fast repeat and a
/// slower one both land at no more than ten acts a second, and a repeat
/// slower than that acts on every press.
///
/// @spec TUI-NAV-081
#[test]
fn a_held_mnemonics_pace_does_not_depend_on_the_key_repeat_rate() {
    let state = rich();
    assert_mnemonic_fixture(&state);
    let (c, section, focus, target) = one_context_per_mnemonic(&state)
        .into_iter()
        .next()
        .expect("fixture: the table has a mnemonic");
    for rate_ms in [15u64, 40] {
        let (presses, fired) =
            hold_mnemonic(c, section, &focus, &target, Some(Duration::from_millis(rate_ms)), &state);
        assert!(presses > 11, "fixture: only {presses} presses at one per {rate_ms}ms");
        assert_no_faster_than_ten_a_second(&fired, &format!("`{c}` repeating every {rate_ms}ms"));
    }
    let (presses, fired) =
        hold_mnemonic(c, section, &focus, &target, Some(Duration::from_millis(150)), &state);
    assert_eq!(
        fired.len() as u32,
        presses,
        "`{c}` repeating every 150ms acted {} times from {presses} presses; a repeat slower than \
         ten a second is slower than the cap, so every press acts",
        fired.len()
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-093 / TUI-NAV-094 — a kitten's one row, and its row-bound key
//
// A kitten is one row in the Clowder section: its Enter promotes it, the
// Clowder `p` stands for that same row, and appointing the kitten Elder is
// taken from that row by `n` alone — a key bound to the row without standing
// for a row of its own. These tests find a kitten's row by the kitten it is
// about (`RowKey::Kitten(ix, _)`), never by a tag, so they hold whatever the
// row's identity is spelled as.
//
// The core opens the command gate of AppointElder whenever the roster is not
// empty (SIM-ENG-045), so every listed kitten row stands under an open gate;
// no reachable meadow lists a kitten row with that gate shut. The sweeps below
// still read the gate per row rather than assuming it, so a later gate that
// can shut on a listed kitten is judged by the same claim.
// ---------------------------------------------------------------------------

/// Every kitten row the Clowder section lists, with the roster position of the
/// kitten it is about, in list order.
fn kitten_rows(state: &GameState) -> Vec<(usize, RowKey)> {
    rows(state, Section::Clowder)
        .into_iter()
        .filter_map(|r| match r.key {
            RowKey::Kitten(ix, _) => Some((ix, r.key)),
            _ => None,
        })
        .collect()
}

/// A meadow a few kittens old with nobody yet appointed Elder, beside `rich`'s
/// long roster with an Elder standing — so the sweeps meet both.
fn small_clowder() -> GameState {
    let mut s = fresh();
    s.kittens = ["Biscuit", "Clover", "Dumpling"].iter().map(|n| kitten(n, None)).collect();
    s.roster_high_water = s.kittens.len();
    s.elder = None;
    s
}

/// Every kitten row's Enter promotes the kitten the row is about, and the
/// Clowder `p` — a focused-item mnemonic standing for that very row — is live
/// on it and answers exactly as that Enter does, in place. Quantified over
/// every kitten row of a long roster with an Elder standing and of a small one
/// with none.
///
/// @spec TUI-NAV-093
#[test]
fn every_kitten_rows_enter_promotes_it_and_the_clowder_p_answers_as_that_enter_does() {
    let mut checked = 0u32;
    for (label, state) in [("a meadow far along", rich()), ("a small clowder", small_clowder())] {
        let kittens = kitten_rows(&state);
        assert!(!kittens.is_empty(), "fixture: {label} lists kitten rows");
        for (ix, key) in kittens {
            let before = mnemonic_view(Section::Clowder, Some(key.clone()));
            let whr = format!("in {label}, with the kitten row {key:?} focused");

            let mut on_enter = before.clone();
            let entered = press(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut on_enter, &state);
            assert_eq!(
                entered,
                Outcome::Apply(Command::Promote(ix)),
                "{whr}, Enter answered {entered:?}; a kitten's one row promotes the kitten it is \
                 about"
            );

            let mut on_p = before.clone();
            let pressed = press_char('p', &mut on_p, &state);
            assert_eq!(
                pressed, entered,
                "{whr}, `p` answered {pressed:?} where the row's Enter answers {entered:?}; the \
                 Clowder `p` stands for the kitten's own row and is live on it"
            );
            assert_eq!(footer_text(&on_p), None, "{whr}, `p` left a footer note");
            assert_eq!(
                observable_but_footer(&on_p),
                observable_but_footer(&before),
                "{whr}, `p` changed the view; a mnemonic acts on its row in place"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "fixture: no kitten row was checked");
}

/// While the cursor is on a kitten's row with the gate of AppointElder open for
/// its kitten and nothing holding the keyboard, `n` issues AppointElder for
/// that kitten, once, through the same path Enter's commands take — so the
/// core arbitrates and a refusal comes back as the footer — and moves neither
/// the cursor nor the focused section, leaving the report banner up and the
/// inspector call, which the narrow layout answers with its swap, as it stood.
/// Quantified over every kitten row of a long roster with an Elder standing
/// and of a small one with none.
///
/// @spec TUI-NAV-094
#[test]
fn n_on_every_kitten_row_appoints_that_kitten_once_and_in_place() {
    let mut checked = 0u32;
    for (label, state) in [("a meadow far along", rich()), ("a small clowder", small_clowder())] {
        let kittens = kitten_rows(&state);
        assert!(!kittens.is_empty(), "fixture: {label} lists kitten rows");
        for (ix, key) in kittens {
            assert!(
                state.command_gate_open(&Command::AppointElder(ix)),
                "fixture: in {label} the core reports the gate of AppointElder for kitten {ix} \
                 open"
            );
            let before = mnemonic_view(Section::Clowder, Some(key.clone()));
            let mut view = before.clone();
            let out = press_char('n', &mut view, &state);
            let whr = format!("in {label}, with the kitten row {key:?} focused");

            assert_eq!(
                out,
                Outcome::Apply(Command::AppointElder(ix)),
                "{whr}, `n` answered {out:?}; it issues AppointElder for the kitten the row is \
                 about, once, for the core to arbitrate"
            );
            assert_eq!(view.section, Some(Section::Clowder), "{whr}, `n` changed the focused section");
            assert_eq!(view.focus, before.focus, "{whr}, `n` moved the cursor; it is an act, not navigation");
            assert_eq!(
                view.reports, before.reports,
                "{whr}, `n` dismissed the report banner; the press is an act, not navigation"
            );
            assert_eq!(
                view.inspector_wanted, before.inspector_wanted,
                "{whr}, `n` changed the inspector call, and with it the narrow layout's swap"
            );
            assert_eq!(footer_text(&view), None, "{whr}, `n` left a footer note of its own");
            assert_eq!(
                observable_but_footer(&view),
                observable_but_footer(&before),
                "{whr}, `n` changed the view beyond issuing its command"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "fixture: no kitten row was checked");
}

/// Hold `n` on `key` for a second, one press every `pace` (or as fast as the
/// loop runs), and return how many presses there were and the instants the
/// press asked the core for AppointElder.
fn hold_n(state: &GameState, ix: usize, key: &RowKey, pace: Option<Duration>) -> (u32, Vec<Instant>) {
    let mut view = focused(Section::Clowder, key.clone());
    let start = Instant::now();
    let mut presses = 0u32;
    let mut fired = Vec::new();
    while start.elapsed() < Duration::from_secs(1) {
        presses += 1;
        if would_fire(&press_char('n', &mut view, state), Command::AppointElder(ix)) {
            fired.push(Instant::now());
        }
        if let Some(p) = pace {
            std::thread::sleep(p);
        }
    }
    (presses, fired)
}

/// Held or auto-repeating, `n` acts no more than ten times a second, whatever
/// the terminal's key-repeat rate: a stream far denser than any key repeat, a
/// fast repeat, and a slower one all land at ten or fewer, and a repeat slower
/// than the cap acts on every press.
///
/// @spec TUI-NAV-094
#[test]
fn a_held_n_acts_no_more_than_ten_times_a_second_whatever_the_repeat_rate() {
    let state = small_clowder();
    let (ix, key) = kitten_rows(&state).into_iter().next().expect("fixture: a kitten row is listed");

    for pace in [None, Some(Duration::from_millis(15)), Some(Duration::from_millis(40))] {
        let (presses, fired) = hold_n(&state, ix, &key, pace);
        let what = format!("`n` held on {key:?}, one press every {pace:?}");
        assert!(presses > 11, "fixture: {what} made only {presses} presses in a second");
        assert!(
            fired.len() >= 2,
            "{what} acted {} times in a second; held, it still repeats, as a held Enter does",
            fired.len()
        );
        assert!(
            fired.len() <= 11,
            "{what} acted {} times in a second; it acts no more than ten times a second",
            fired.len()
        );
        for pair in fired.windows(2) {
            let gap = pair[1].duration_since(pair[0]);
            assert!(
                gap >= Duration::from_millis(95),
                "{what} acted twice {gap:?} apart; it acts no more than ten times a second"
            );
        }
    }

    let (presses, fired) = hold_n(&state, ix, &key, Some(Duration::from_millis(150)));
    assert_eq!(
        fired.len() as u32,
        presses,
        "`n` repeating every 150ms acted {} times from {presses} presses; a repeat slower than \
         ten a second is slower than the cap, so every press acts",
        fired.len()
    );
}

/// `n` is bound only where the global key table is. An open field takes it as
/// its own — the palette as query text, the count prompt as a non-digit it
/// ignores — and a full-screen overlay suspends it with the other action keys,
/// so under each of the four things that can hold the keyboard it asks the
/// core for nothing. Quantified over every kitten row.
///
/// @spec TUI-NAV-094
#[test]
fn n_never_acts_while_a_field_or_full_screen_overlay_holds_the_keyboard() {
    let state = small_clowder();
    let kittens = kitten_rows(&state);
    assert!(!kittens.is_empty(), "fixture: kitten rows are listed");

    type Holder = (&'static str, fn(&mut ViewState, RowKey));
    let holders: [Holder; 4] = [
        ("the palette", |v, _| v.overlay = Some(Overlay::Palette { query: String::new(), selected: 0 })),
        ("the count prompt", |v, row| {
            v.prompt = Some(clowder_tui::nav::CountPrompt { digits: String::new(), row: Some(row) })
        }),
        ("the Journal", |v, _| v.overlay = Some(Overlay::Journal { anchor: 0 })),
        ("the help overlay", |v, _| v.overlay = Some(Overlay::Help)),
    ];

    for (ix, key) in kittens {
        for (holder, open) in holders {
            let mut before = mnemonic_view(Section::Clowder, Some(key.clone()));
            open(&mut before, key.clone());
            let mut view = before.clone();
            let out = press_char('n', &mut view, &state);
            let whr = format!("with the kitten row {key:?} focused and {holder} open");

            assert!(
                !would_fire(&out, Command::AppointElder(ix)),
                "{whr}, `n` issued AppointElder; the key is bound only where the global table is"
            );
            assert_eq!(out, Outcome::Handled, "{whr}, `n` told the core {out:?}");
            let mut expected = before.clone();
            if let Some(Overlay::Palette { query, .. }) = expected.overlay.as_mut() {
                query.push('n');
            }
            assert_eq!(
                observable_but_footer(&view),
                observable_but_footer(&expected),
                "{whr}, `n` did something other than what {holder} makes of the letter"
            );
            assert_eq!(footer_text(&view), None, "{whr}, `n` earned a footer note");
        }
    }
}

/// Everywhere but a kitten's row under an open gate, `n` is an unbound key:
/// ignored, asking the core for nothing, changing nothing, and leaving no
/// footer note. Quantified over every screen that lists rows, with each of its
/// selectable rows focused and with none, in a meadow far along and a new one;
/// over the Clowder section of a new meadow, which lists nothing, remembering
/// a kitten row or nothing; and over a Sky that lists nothing.
///
/// @spec TUI-NAV-094
#[test]
fn n_is_an_unbound_key_everywhere_but_a_kitten_row_whose_gate_is_open() {
    let mut contexts: Vec<(String, GameState, ViewState)> = Vec::new();
    for (label, state) in [("a meadow far along", rich()), ("a new meadow", fresh())] {
        for (section, focus) in mnemonic_contexts(&state) {
            let view = mnemonic_view(section, focus.clone());
            contexts.push((format!("in {label}, {}", describe(section, &focus)), state.clone(), view));
        }
    }
    let new_meadow = fresh();
    assert!(rows(&new_meadow, Section::Clowder).is_empty(), "fixture: a new meadow's Clowder lists nothing");
    for memory in [None, Some(RowKey::Kitten(0, "")), Some(RowKey::Kitten(0, "promote"))] {
        let view = mnemonic_view(Section::Clowder, memory.clone());
        contexts.push((
            format!("in a new meadow, on an empty Clowder remembering {memory:?}"),
            new_meadow.clone(),
            view,
        ));
    }
    let sky = sky_emptied();
    for (what, view) in standing_in_an_empty_sky() {
        contexts.push((format!("in an empty Sky, {what}"), sky.clone(), view));
    }

    let (mut silent, mut live) = (0u32, 0u32);
    for (whr, state, before) in contexts {
        let section = before.section.expect("every context has a section");
        let focus = before.focus[section.index()].clone();
        let on_live_kitten_row = section == Section::Clowder
            && kitten_rows(&state).iter().any(|(ix, k)| {
                Some(k) == focus.as_ref() && state.command_gate_open(&Command::AppointElder(*ix))
            });
        if on_live_kitten_row {
            live += 1;
            continue;
        }
        let mut view = before.clone();
        let out = press_char('n', &mut view, &state);
        assert_eq!(out, Outcome::Handled, "{whr}, `n` is unbound here and yet told the core {out:?}");
        assert_eq!(
            footer_text(&view),
            None,
            "{whr}, `n` is unbound here and yet earned a footer note; an unbound key is ignored \
             in silence"
        );
        assert_eq!(
            observable_but_footer(&view),
            observable_but_footer(&before),
            "{whr}, `n` is unbound here and yet changed the view"
        );
        silent += 1;
    }
    assert!(silent > 0, "fixture: no context where `n` is unbound was checked");
    assert!(live > 0, "fixture: the sweep met no kitten row, so it told nothing apart");
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

/// Enter is ×1 on every row, so on an unarmed count-taking row it takes one
/// unit: every Pounce and Trade row's Enter passes a count of one — one squad,
/// one caravan — and the unarmed nap row's Enter arms a nap of one year rather
/// than firing. Quantified over every count-taking row the game lists.
///
/// @spec TUI-NAV-031
#[test]
fn enter_on_every_unarmed_count_taking_row_takes_one_unit() {
    let state = rich();
    let (mut ordinary, mut heavy, mut trade) = (0, 0, 0);

    for (section, row) in every_row(&state).into_iter().filter(|(_, r)| is_count_taking(r)) {
        let command = row_command(&row).expect("a count-taking row has a command");
        let mut view = focused(section, row.key.clone());
        let out = update(Action::Activate, &mut view, &state);
        assert!(view.prompt.is_none(), "Enter on {:?} opened a count prompt", row.key);

        if row.heavy {
            heavy += 1;
            assert_eq!(out, Outcome::Handled, "Enter on the unarmed heavy {:?} fired", row.key);
            let armed = view
                .armed
                .as_ref()
                .unwrap_or_else(|| panic!("Enter on the unarmed heavy {:?} armed nothing", row.key));
            assert_eq!(armed.key, row.key, "Enter on {:?} armed a different row", row.key);
            assert_eq!(
                armed.command,
                with_count(command, 1),
                "Enter on the unarmed {:?} armed {:?}; one unit is a count of one",
                row.key,
                armed.command
            );
        } else {
            ordinary += 1;
            if matches!(command, Command::Trade(..)) {
                trade += 1;
            }
            assert_eq!(
                out,
                Outcome::Apply(with_count(command, 1)),
                "Enter on {:?} did not pass a count of one; Enter is the row's action ×1",
                row.key
            );
            assert!(view.armed.is_none(), "Enter on the ordinary {:?} armed it", row.key);
        }
    }

    assert!(ordinary > trade, "fixture: the Pounce row must be listed");
    assert!(trade > 0, "fixture: a trade row must be listed");
    assert!(heavy > 0, "fixture: the nap row must be listed");
}

/// An empty or zero count closes the prompt with no command and no footer
/// note — the prompt closing is its answer, as it is for Esc — on every row
/// the prompt opens on. On the nap row an arming already standing is left
/// exactly as it stood. Where the prompt's row has gone, that same Enter
/// reports the row's going as any Enter on a vanished row does.
///
/// @spec TUI-NAV-031
#[test]
fn an_empty_or_zero_count_closes_the_prompt_with_no_command_and_no_footer_on_every_row() {
    let state = rich();
    let gone = fresh();
    let (mut seen, mut armed_rows, mut vanished) = (0, 0, 0);

    for (section, row) in every_row(&state).into_iter().filter(|(_, r)| opens_prompt(&state, r)) {
        seen += 1;
        for typed in ["", "0", "00"] {
            let mut view = focused(section, row.key.clone());
            if row.heavy {
                // Arm the row first, so the cancel has an arming to leave be.
                update(Action::Activate, &mut view, &state);
                assert!(view.armed.is_some(), "fixture: Enter on the heavy {:?} did not arm", row.key);
                armed_rows += 1;
            }
            let before = view.armed.clone();
            update(Action::CountPrompt, &mut view, &state);
            assert!(view.prompt.is_some(), "fixture: x on {:?} opened no prompt", row.key);
            for c in typed.chars() {
                update(Action::Type(c), &mut view, &state);
            }
            let out = update(Action::Activate, &mut view, &state);

            assert_eq!(
                commands_of(&out),
                None,
                "a count of {typed:?} on {:?} asked the core for {out:?}",
                row.key
            );
            assert!(view.prompt.is_none(), "a count of {typed:?} left the prompt over {:?} open", row.key);
            assert_eq!(
                footer_text(&view),
                None,
                "a count of {typed:?} on {:?} left a footer note; the prompt closing is its answer",
                row.key
            );
            assert_eq!(
                view.armed, before,
                "a count of {typed:?} on {:?} changed the arming; a cancel leaves it as it stood",
                row.key
            );
        }

        if !rows(&gone, section).iter().any(|r| r.key == row.key) {
            vanished += 1;
            for typed in ["", "0"] {
                let mut view = open_and_type(section, &row, typed, &state);
                let out = update(Action::Activate, &mut view, &gone);
                assert_eq!(
                    commands_of(&out),
                    None,
                    "a count of {typed:?} on the vanished {:?} asked the core for something",
                    row.key
                );
                assert!(view.prompt.is_none(), "the prompt over the vanished {:?} stayed open", row.key);
                assert_footer_says_something_plain(
                    &view,
                    &format!("a count of {typed:?} on a prompt whose row {:?} has gone", row.key),
                );
            }
        }
    }

    assert!(seen > 0, "fixture: no row opens the count prompt");
    assert!(armed_rows > 0, "fixture: the heavy nap row opens the count prompt");
    assert!(vanished > 0, "fixture: no prompt row leaves its list in a fresh meadow");
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
// Which rows take a count, quantified over every row the game lists
//
// The claims below are about *every* row of a kind, so they walk every
// selectable row of every section rather than one example. A row's class is
// read from its tag (TUI-NAV-031) and from the core's ownership report — never
// from a list of commands — so the tests stay the requirement whichever
// command a later cascade retags.
// ---------------------------------------------------------------------------

/// Every selectable row of every section, with the section it is in.
fn every_row(state: &GameState) -> Vec<(Section, Row)> {
    Section::ALL
        .into_iter()
        .flat_map(|section| {
            rows(state, section)
                .into_iter()
                .filter(|r| r.selectable())
                .map(move |r| (section, r))
        })
        .collect()
}

/// Whether the core reports the row's content item as already the player's.
fn owned(state: &GameState, row: &Row) -> bool {
    match &row.key {
        RowKey::Content(id, _) => state.lock_reason(*id) == Some(LockReason::AlreadyYours),
        _ => false,
    }
}

/// Whether `x` opens the count prompt on this row: a row tagged count-taking or
/// repeat-N, or a count row, that the core does not report as already the
/// player's. @spec TUI-NAV-011
fn opens_prompt(state: &GameState, row: &Row) -> bool {
    !owned(state, row)
        && match &row.kind {
            RowKind::Act { semantics, .. } => {
                matches!(semantics, CountSemantics::CountTaking | CountSemantics::RepeatN)
            }
            RowKind::Count { .. } => true,
            RowKind::Display | RowKind::Group => false,
        }
}

/// Whether the row is count-taking.
fn is_count_taking(row: &Row) -> bool {
    matches!(row.kind, RowKind::Act { semantics: CountSemantics::CountTaking, .. })
}

/// What a count prompt's Enter with a count of `n` asks of the core on a row
/// that does not arm: the count passed once to a count-taking command, or a
/// repeat-N command or a count row's assign called `n` times. @spec TUI-NAV-070
fn accepted(row: &Row, n: u32) -> Outcome {
    match &row.kind {
        RowKind::Act { command, semantics: CountSemantics::CountTaking } => {
            Outcome::Apply(with_count(*command, n))
        }
        RowKind::Act { command, semantics: CountSemantics::RepeatN } => Outcome::Repeat(*command, n),
        RowKind::Count { increment, .. } => Outcome::Repeat(*increment, n),
        other => panic!("fixture: {:?} is not a row the count prompt opens on ({other:?})", row.key),
    }
}

/// One unit of the row's action: what each call of an `X` run carries.
/// @spec TUI-NAV-012
fn one_unit(row: &Row) -> Command {
    match &row.kind {
        RowKind::Act { command, semantics: CountSemantics::CountTaking } => with_count(*command, 1),
        RowKind::Act { command, .. } => *command,
        RowKind::Count { increment, .. } => *increment,
        other => panic!("fixture: {:?} has no action to scale ({other:?})", row.key),
    }
}

/// The command a row would put through the core on Enter, if it has one.
fn row_command(row: &Row) -> Option<Command> {
    match &row.kind {
        RowKind::Act { command, .. } => Some(*command),
        RowKind::Count { increment, .. } => Some(*increment),
        RowKind::Display | RowKind::Group => None,
    }
}

/// Open the count prompt on `row` the way a player does, and type `digits`.
fn open_and_type(
    section: Section,
    row: &Row,
    digits: &str,
    state: &GameState,
) -> ViewState {
    let mut view = focused(section, row.key.clone());
    let out = update(Action::CountPrompt, &mut view, state);
    assert_eq!(out, Outcome::Handled, "x on {:?} asked the core for something", row.key);
    assert!(view.prompt.is_some(), "x on {:?} opened no count prompt", row.key);
    for c in digits.chars() {
        update(Action::Type(c), &mut view, state);
    }
    view
}

/// `rich()`, with one heavy rule already adopted — so an owned heavy row is
/// on the list alongside the owned one-shot `rich()` already carries.
fn rich_with_an_adopted_rule() -> GameState {
    let base = rich();
    for r in RuleId::ALL {
        let mut s = base.clone();
        s.rules.insert(r);
        let id = ContentId::Rule(r);
        let listed = rows(&s, Section::Ideas)
            .iter()
            .any(|row| row.key == RowKey::Content(id, "") && row.heavy);
        if listed && s.lock_reason(id) == Some(LockReason::AlreadyYours) {
            return s;
        }
    }
    panic!("fixture: no rule can be adopted and still be listed as an owned heavy row");
}

/// `x` opens the count prompt on every row whose command takes a count or is
/// repeated N times, and on every count row — the heavy DeepNap row among them
/// — unless the core reports the row's item as already the player's; on every
/// other row it opens nothing. Wherever it opens, Esc closes it with no effect:
/// nothing fires, nothing arms, and the cursor stays where it was.
///
/// @spec TUI-NAV-011
#[test]
fn x_opens_the_count_prompt_on_exactly_the_rows_that_take_a_count() {
    let state = rich_with_an_adopted_rule();
    let (mut repeat_n, mut count_taking, mut count_rows, mut heavy) = (0, 0, 0, 0);

    for (section, row) in every_row(&state) {
        let mut view = focused(section, row.key.clone());
        // A finished Ideas row is reached only through the open done group
        // (TUI-NAV-113).
        unfold_focus(&state, &mut view);
        let out = update(Action::CountPrompt, &mut view, &state);
        assert_eq!(
            commands_of(&out),
            None,
            "x on {:?} put a command through the core; x only opens a prompt",
            row.key
        );

        if !opens_prompt(&state, &row) {
            assert!(
                view.prompt.is_none(),
                "x opened a count prompt on {:?}, a row that takes no count (or is already the \
                 player's)",
                row.key
            );
            continue;
        }

        assert!(
            view.prompt.is_some(),
            "x on {:?} opened no count prompt; every row tagged count-taking or repeat-N, and \
             every count row, takes one",
            row.key
        );
        assert_eq!(
            view.prompt.as_ref().map(|p| p.digits.as_str()),
            Some(""),
            "the count prompt on {:?} opened with something already typed",
            row.key
        );
        assert!(view.armed.is_none(), "x on {:?} armed the row", row.key);
        match &row.kind {
            RowKind::Count { .. } => count_rows += 1,
            _ if row.heavy => heavy += 1,
            _ if is_count_taking(&row) => count_taking += 1,
            _ => repeat_n += 1,
        }

        // Esc closes it, and nothing else changes.
        update(Action::Type('5'), &mut view, &state);
        let out = update(Action::Dismiss, &mut view, &state);
        assert_eq!(out, Outcome::Handled, "Esc on the count prompt over {:?} acted", row.key);
        assert!(view.prompt.is_none(), "Esc left the count prompt over {:?} open", row.key);
        assert!(view.armed.is_none(), "Esc on the count prompt over {:?} armed it", row.key);
        assert!(
            view.footer.is_none(),
            "Esc on the count prompt over {:?} left a footer note; closing is no effect",
            row.key
        );
        assert_eq!(
            view.focus[section.index()].as_ref(),
            Some(&row.key),
            "Esc on the count prompt moved the cursor off {:?}",
            row.key
        );
    }

    assert!(repeat_n > 0, "fixture: no repeat-N row was listed");
    assert!(count_taking > 0, "fixture: no ordinary count-taking row was listed");
    assert!(count_rows > 0, "fixture: no count row was listed");
    assert!(heavy > 0, "fixture: the heavy DeepNap row was not listed");
}

/// The prompt's Enter with a nonzero count does what the row takes a count as:
/// passes it once to a count-taking command, calls a repeat-N command or a
/// count row's assign that many times — the core arbitrating each call — and on
/// DeepNap, arms the row with the counted command instead of firing.
///
/// @spec TUI-NAV-070
#[test]
fn the_count_prompts_enter_does_what_each_row_takes_a_count_as() {
    let state = rich();
    let mut seen = 0;

    for (section, row) in every_row(&state).into_iter().filter(|(_, r)| opens_prompt(&state, r)) {
        seen += 1;
        let mut view = open_and_type(section, &row, "6", &state);
        let out = update(Action::Activate, &mut view, &state);
        assert!(view.prompt.is_none(), "the count prompt over {:?} stayed open", row.key);

        if row.heavy {
            assert_eq!(
                out,
                Outcome::Handled,
                "the prompt's Enter on the heavy {:?} fired; it arms, and a distinct Enter commits",
                row.key
            );
            let armed = view.armed.as_ref().unwrap_or_else(|| {
                panic!("the prompt's Enter on the heavy {:?} armed nothing", row.key)
            });
            assert_eq!(armed.key, row.key, "the prompt's Enter armed a different row");
            assert_eq!(
                Some(armed.command),
                row_command(&row).map(|c| with_count(c, 6)),
                "the armed {:?} does not carry the typed count",
                row.key
            );
        } else {
            assert_eq!(
                out,
                accepted(&row, 6),
                "×6 on {:?} did not ask the core for what that row takes a count as",
                row.key
            );
            assert!(view.armed.is_none(), "×6 on the ordinary {:?} armed it", row.key);
        }
    }
    assert!(seen > 0, "fixture: no row opens the count prompt");
}

/// The count prompt makes no affordability pre-check. With the stores empty,
/// every row it opens on — Teased and unaffordable ones included — opens it
/// exactly as a rich meadow's does, and its Enter attempts the count, leaving
/// the core to accept or reject it.
///
/// @spec TUI-NAV-071
#[test]
fn the_count_prompt_opens_and_attempts_on_rows_the_stock_cannot_cover() {
    let mut state = rich();
    for r in ResourceId::ALL {
        state.resources[r.ix()] = 0.0;
    }

    let teased = every_row(&state).into_iter().any(|(_, r)| match &r.key {
        RowKey::Content(id, _) => state.availability(*id) == Availability::Teased,
        _ => false,
    });
    assert!(teased, "fixture: with the stores empty, some listed row should be Teased");

    let mut seen = 0;
    for (section, row) in every_row(&state).into_iter().filter(|(_, r)| opens_prompt(&state, r)) {
        seen += 1;
        let mut view = focused(section, row.key.clone());
        update(Action::CountPrompt, &mut view, &state);
        assert!(
            view.prompt.is_some(),
            "x on {:?} opened no count prompt with the stores empty; the prompt makes no \
             affordability pre-check",
            row.key
        );
        assert!(
            view.footer.is_none(),
            "x on the unaffordable {:?} left a footer note; the core, not the shell, says what is \
             short — and only once the count is attempted",
            row.key
        );
        update(Action::Type('3'), &mut view, &state);
        let out = update(Action::Activate, &mut view, &state);
        if row.heavy {
            assert!(
                view.armed.as_ref().is_some_and(|a| a.key == row.key),
                "the prompt's Enter on the unaffordable heavy {:?} did not arm it; arming \
                 reserves nothing and pre-checks nothing",
                row.key
            );
        } else {
            assert_eq!(
                out,
                accepted(&row, 3),
                "×3 on the unaffordable {:?} was not attempted; the core is the single arbiter",
                row.key
            );
        }
    }
    assert!(seen > 0, "fixture: no row opens the count prompt");
}

/// An open prompt belongs to the row it was opened on, by that row's identity.
/// Whatever the focus reads by the time Enter is pressed — modelled here by
/// moving it to another row of the same section beneath the open prompt — the
/// prompt's Enter acts on its own row, never on another.
///
/// @spec TUI-NAV-072
#[test]
fn an_open_count_prompt_acts_only_on_the_row_it_was_opened_on() {
    let state = rich();
    let mut seen = 0;

    for (section, row) in every_row(&state).into_iter().filter(|(_, r)| opens_prompt(&state, r)) {
        let Some(other) = rows(&state, section)
            .into_iter()
            .find(|r| r.selectable() && r.key != row.key && row_command(r) != row_command(&row))
        else {
            continue;
        };
        seen += 1;

        let mut view = open_and_type(section, &row, "4", &state);
        view.focus[section.index()] = Some(other.key.clone());
        let out = update(Action::Activate, &mut view, &state);

        if let Some(theirs) = row_command(&other) {
            assert!(
                !would_fire(&out, theirs) && !would_fire(&out, with_count(theirs, 4)),
                "the prompt opened on {:?} acted on {:?} instead",
                row.key,
                other.key
            );
        }
        if row.heavy {
            let armed = view.armed.as_ref().unwrap_or_else(|| {
                panic!("the prompt opened on the heavy {:?} armed nothing", row.key)
            });
            assert_eq!(
                armed.key, row.key,
                "the prompt opened on {:?} armed {:?}; it belongs to its own row",
                row.key, armed.key
            );
        } else {
            assert_eq!(
                out,
                accepted(&row, 4),
                "the prompt opened on {:?} did not act on that row once the focus read {:?}",
                row.key,
                other.key
            );
        }
    }
    assert!(seen > 0, "fixture: no section holds a prompt row beside another row");
}

/// The sim keeps ticking under an open prompt, and a section's rows are
/// re-derived from state every frame, so the prompt's row can leave its list
/// before Enter is pressed. Then the prompt closes with no effect — nothing
/// fires, nothing arms — and a footer note says the row has gone. Modelled by
/// pressing Enter against a meadow in which the row is not listed.
///
/// @spec TUI-NAV-072
#[test]
fn a_count_prompt_whose_row_has_left_its_list_closes_with_no_effect_and_says_so() {
    let state = rich();
    let gone = fresh();
    let (mut repeat_n, mut count_taking, mut count_rows, mut heavy) = (0, 0, 0, 0);

    for (section, row) in every_row(&state).into_iter().filter(|(_, r)| opens_prompt(&state, r)) {
        if rows(&gone, section).iter().any(|r| r.key == row.key) {
            continue;
        }
        match &row.kind {
            RowKind::Count { .. } => count_rows += 1,
            _ if row.heavy => heavy += 1,
            _ if is_count_taking(&row) => count_taking += 1,
            _ => repeat_n += 1,
        }

        let mut view = open_and_type(section, &row, "4", &state);
        let out = update(Action::Activate, &mut view, &gone);

        assert_eq!(
            out,
            Outcome::Handled,
            "Enter on a prompt whose row {:?} is no longer listed asked the core for something",
            row.key
        );
        assert!(view.prompt.is_none(), "the prompt over the vanished {:?} stayed open", row.key);
        assert!(view.armed.is_none(), "the prompt over the vanished {:?} armed a row", row.key);
        assert_footer_says_something_plain(
            &view,
            &format!("Enter on a prompt whose row {:?} has left its list", row.key),
        );
    }

    assert!(repeat_n > 0, "fixture: no repeat-N row leaves its list in a fresh meadow");
    assert!(count_taking > 0, "fixture: no count-taking row leaves its list in a fresh meadow");
    assert!(count_rows > 0, "fixture: no count row leaves its list in a fresh meadow");
    assert!(heavy > 0, "fixture: the DeepNap row does not leave its list in a fresh meadow");
}

/// `X` runs on every row that is not heavy, not already the player's, and
/// takes a count or is repeated N times, and on every count row — one unit per
/// call: a repeat-N command as it stands, a count-taking command with a count
/// of one, a count row's assign — repeatedly, not as a single call.
///
/// @spec TUI-NAV-012
#[test]
fn x_max_runs_every_row_that_takes_a_count_one_unit_per_call() {
    let state = rich();
    let (mut count_taking, mut seen) = (0, 0);

    for (section, row) in every_row(&state)
        .into_iter()
        .filter(|(_, r)| opens_prompt(&state, r) && !r.heavy)
    {
        seen += 1;
        if is_count_taking(&row) {
            count_taking += 1;
        }
        let mut view = focused(section, row.key.clone());
        let out = update(Action::MaxAffordable, &mut view, &state);

        assert_eq!(
            commands_of(&out),
            Some(one_unit(&row)),
            "X on {:?} yielded {out:?}; each call of an X run carries one unit of the row's \
             action — a count-taking command with a count of one",
            row.key
        );
        assert!(
            !matches!(out, Outcome::Apply(_)),
            "X on {:?} asked for a single call; X applies the action repeatedly until the core \
             rejects it or the cap is reached",
            row.key
        );
        assert!(view.prompt.is_none() && view.armed.is_none(), "X on {:?} opened or armed", row.key);
    }
    assert!(seen > 0, "fixture: no row X runs on is listed");
    assert!(count_taking > 0, "fixture: no count-taking row X runs on is listed");
}

/// Whether `X` runs on this row: a row `x` opens the prompt on that is not
/// heavy. @spec TUI-NAV-012
fn x_runs_on(state: &GameState, row: &Row) -> bool {
    opens_prompt(state, row) && !row.heavy
}

/// The shell-side cap is ten thousand calls: every row `X` runs on asks for
/// its one unit exactly that many times, and the constant the prompt's count
/// is bounded by is that same number.
///
/// @spec TUI-NAV-012
#[test]
fn x_max_asks_for_exactly_ten_thousand_calls_on_every_row_it_runs_on() {
    assert_eq!(MAX_AFFORDABLE_CAP, 10_000, "the shell-side cap is ten thousand calls");
    let state = rich();
    let mut seen = 0;

    for (section, row) in every_row(&state).into_iter().filter(|(_, r)| x_runs_on(&state, r)) {
        seen += 1;
        let mut view = focused(section, row.key.clone());
        let out = update(Action::MaxAffordable, &mut view, &state);
        assert_eq!(
            out,
            Outcome::Repeat(one_unit(&row), 10_000),
            "X on {:?} did not ask for its one unit ten thousand times",
            row.key
        );
    }
    assert!(seen > 0, "fixture: no row X runs on is listed");
}

/// `X` makes no affordability pre-check, as Enter and `x` make none: with the
/// stores empty, every row it runs on — Teased and unaffordable ones included
/// — asks for the same run a rich meadow's does, with no footer of the shell's
/// own; the core arbitrates each call.
///
/// @spec TUI-NAV-012
#[test]
fn x_max_runs_on_rows_the_stock_cannot_cover() {
    let mut state = rich();
    for r in ResourceId::ALL {
        state.resources[r.ix()] = 0.0;
    }
    let teased = every_row(&state).into_iter().any(|(_, r)| match &r.key {
        RowKey::Content(id, _) => x_runs_on(&state, &r) && state.availability(*id) == Availability::Teased,
        _ => false,
    });
    assert!(teased, "fixture: with the stores empty, some row X runs on should be Teased");

    let mut seen = 0;
    for (section, row) in every_row(&state).into_iter().filter(|(_, r)| x_runs_on(&state, r)) {
        seen += 1;
        let mut view = focused(section, row.key.clone());
        let out = update(Action::MaxAffordable, &mut view, &state);
        assert_eq!(
            out,
            Outcome::Repeat(one_unit(&row), MAX_AFFORDABLE_CAP),
            "X on the unaffordable {:?} was not attempted; X makes no affordability pre-check",
            row.key
        );
        assert_eq!(
            footer_text(&view),
            None,
            "X on the unaffordable {:?} left a footer note of the shell's own; the core says \
             what is short once the run is attempted",
            row.key
        );
    }
    assert!(seen > 0, "fixture: no row X runs on is listed");
}

/// One row of each kind `X` runs on: a repeat-N row, a count-taking row, and a
/// count row.
fn one_x_row_of_each_kind(state: &GameState) -> Vec<(Section, Row)> {
    let rows: Vec<(Section, Row)> =
        every_row(state).into_iter().filter(|(_, r)| x_runs_on(state, r)).collect();
    let repeat_n = rows
        .iter()
        .find(|(_, r)| matches!(r.kind, RowKind::Act { semantics: CountSemantics::RepeatN, .. }))
        .cloned()
        .expect("fixture: a repeat-N row X runs on");
    let count_taking =
        rows.iter().find(|(_, r)| is_count_taking(r)).cloned().expect("fixture: a count-taking row X runs on");
    let count_row = rows
        .iter()
        .find(|(_, r)| matches!(r.kind, RowKind::Count { .. }))
        .cloned()
        .expect("fixture: a count row X runs on");
    vec![repeat_n, count_taking, count_row]
}

/// Hold `X` on `row` for a second, one press every `pace` (or as fast as the
/// loop runs), and return how many presses there were and the instants a run
/// was asked for. Every press is checked on the way: a run is a whole run, and
/// a dropped press leaves no footer note.
fn hold_x(section: Section, row: &Row, pace: Option<Duration>, state: &GameState) -> (u32, Vec<Instant>) {
    let mut view = focused(section, row.key.clone());
    let whole = Outcome::Repeat(one_unit(row), MAX_AFFORDABLE_CAP);
    let start = Instant::now();
    let mut presses = 0u32;
    let mut ran = Vec::new();
    while start.elapsed() < Duration::from_secs(1) {
        presses += 1;
        let out = update(Action::MaxAffordable, &mut view, state);
        if commands_of(&out).is_some() {
            assert_eq!(out, whole, "a held X on {:?} made a run that was not a whole run", row.key);
            ran.push(Instant::now());
        } else {
            assert_eq!(out, Outcome::Handled, "a held X on {:?} yielded {out:?}", row.key);
            assert_eq!(
                footer_text(&view),
                None,
                "a held X dropped on {:?} left a footer note; a dropped repeat is silent",
                row.key
            );
        }
        assert!(view.prompt.is_none() && view.armed.is_none(), "a held X on {:?} opened or armed", row.key);
        if let Some(p) = pace {
            std::thread::sleep(p);
        }
    }
    (presses, ran)
}

fn assert_x_no_faster_than_ten_a_second(ran: &[Instant], what: &str) {
    assert!(
        ran.len() >= 2,
        "{what}: a held X ran {} times in a second; held, it must still repeat, as a held Enter \
         does",
        ran.len()
    );
    assert!(
        ran.len() <= 11,
        "{what}: a held X ran {} times in a second; it runs no more than ten times a second",
        ran.len()
    );
    for pair in ran.windows(2) {
        let gap = pair[1].duration_since(pair[0]);
        assert!(
            gap >= Duration::from_millis(95),
            "{what}: a held X ran twice {gap:?} apart; an X within a tenth of a second of the last \
             run is dropped"
        );
    }
}

/// Held, `X` makes no more than ten runs a second — the pace a held Enter
/// keeps — on every kind of row it runs on, under a stream of presses far
/// denser than any terminal's key repeat. Each run it makes is a whole run;
/// every press it drops leaves no footer note.
///
/// @spec TUI-NAV-012
#[test]
fn a_held_x_makes_no_more_than_ten_runs_a_second() {
    let state = rich();
    for (section, row) in one_x_row_of_each_kind(&state) {
        let (presses, ran) = hold_x(section, &row, None, &state);
        assert!(
            presses > 100,
            "fixture: only {presses} presses in a second — the stream must outrun any terminal's \
             key repeat for the pace to be under test"
        );
        assert_x_no_faster_than_ten_a_second(&ran, &format!("X on {:?}", row.key));
    }
}

/// The same pace whatever the terminal's key-repeat rate: a fast repeat and a
/// slower one both land at no more than ten runs a second, and a repeat slower
/// than that runs on every press.
///
/// @spec TUI-NAV-012
#[test]
fn a_held_xs_pace_does_not_depend_on_the_key_repeat_rate() {
    let state = rich();
    let (section, row) = one_x_row_of_each_kind(&state).into_iter().next().expect("fixture");
    for rate_ms in [15u64, 40] {
        let (presses, ran) = hold_x(section, &row, Some(Duration::from_millis(rate_ms)), &state);
        assert!(presses > 11, "fixture: only {presses} presses at one per {rate_ms}ms");
        assert_x_no_faster_than_ten_a_second(&ran, &format!("X repeating every {rate_ms}ms"));
    }
    let (presses, ran) = hold_x(section, &row, Some(Duration::from_millis(150)), &state);
    assert_eq!(
        ran.len() as u32,
        presses,
        "X repeating every 150ms ran {} times from {presses} presses; a repeat slower than ten a \
         second is slower than the pace, so every press runs",
        ran.len()
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

/// Every row `x` or `X` does not act on says why, and the three reasons are
/// three different notes:
///
/// * a row whose item the core reports as already the player's answers Enter,
///   `x` and `X` alike with a note that it is theirs — checked before anything
///   else the key would do, so an owned heavy row never arms, and `X` on it
///   does not fall through to the heavy-row note;
/// * a row tagged single, or a display-only row with no command, answers `x`
///   and `X` with a note that there is nothing to count;
/// * `X` on a heavy row of any tag, DeepNap included, answers instead that the
///   row takes a deliberate Enter, not `X` — a note that there is nothing to
///   count would be untrue of DeepNap.
///
/// Quantified over every row the game lists in a meadow holding an owned
/// one-shot and an owned heavy row.
///
/// @spec TUI-NAV-041
#[test]
fn x_and_x_max_say_why_on_every_row_they_do_not_act_on() {
    let state = rich_with_an_adopted_rule();
    let mut owned_notes: BTreeSet<String> = BTreeSet::new();
    let mut nothing_notes: BTreeSet<String> = BTreeSet::new();
    let mut deliberate_notes: BTreeSet<String> = BTreeSet::new();
    let (mut owned_heavy, mut display, mut heavy_single, mut heavy_counted) = (0, 0, 0, 0);

    let note = |view: &ViewState| view.footer.as_ref().map(|(m, _)| m.clone()).unwrap_or_default();

    for (section, row) in every_row(&state) {
        let what = format!("{:?}", row.key);

        if owned(&state, &row) {
            if row.heavy {
                owned_heavy += 1;
            }
            for action in [Action::Activate, Action::CountPrompt, Action::MaxAffordable] {
                let mut view = focused(section, row.key.clone());
                // A finished Ideas row is reached only through the open done group
                // (TUI-NAV-113).
                unfold_focus(&state, &mut view);
                let out = update(action.clone(), &mut view, &state);
                assert_eq!(
                    commands_of(&out),
                    None,
                    "{action:?} on {what}, which the core reports as already the player's, put a \
                     command through the core"
                );
                assert!(view.armed.is_none(), "{action:?} on the owned {what} armed it");
                assert!(view.prompt.is_none(), "{action:?} on the owned {what} opened a prompt");
                assert_footer_says_something_plain(&view, &format!("{action:?} on the owned {what}"));
                owned_notes.insert(note(&view));
            }
            continue;
        }

        let nothing_to_count = matches!(
            row.kind,
            RowKind::Act { semantics: CountSemantics::Single, .. } | RowKind::Display
        );
        if matches!(row.kind, RowKind::Display) {
            display += 1;
        }

        if nothing_to_count {
            let mut view = focused(section, row.key.clone());
            // A finished Ideas row is reached only through the open done group
            // (TUI-NAV-113).
            unfold_focus(&state, &mut view);
            let out = update(Action::CountPrompt, &mut view, &state);
            assert_eq!(commands_of(&out), None, "x on {what}, which has nothing to count, acted");
            assert!(view.prompt.is_none(), "x on {what} opened a count prompt");
            assert!(view.armed.is_none(), "x on {what} armed it");
            assert_footer_says_something_plain(&view, &format!("x on {what}"));
            nothing_notes.insert(note(&view));
        }

        let mut view = focused(section, row.key.clone());
        // A finished Ideas row is reached only through the open done group
        // (TUI-NAV-113).
        unfold_focus(&state, &mut view);
        let out = update(Action::MaxAffordable, &mut view, &state);
        if row.heavy {
            if nothing_to_count {
                heavy_single += 1;
            } else {
                heavy_counted += 1;
            }
            assert_eq!(commands_of(&out), None, "X on the heavy {what} put a command through");
            assert!(view.armed.is_none(), "X on the heavy {what} armed it");
            assert!(view.prompt.is_none(), "X on the heavy {what} opened a prompt");
            assert_footer_says_something_plain(&view, &format!("X on the heavy {what}"));
            let said = note(&view);
            assert!(
                said.contains("Enter"),
                "X on the heavy {what} said {said:?}; the note is that the row takes a \
                 deliberate Enter, not X"
            );
            deliberate_notes.insert(said);
        } else if nothing_to_count {
            assert_eq!(commands_of(&out), None, "X on {what}, which has nothing to count, acted");
            assert!(view.prompt.is_none() && view.armed.is_none(), "X on {what} opened or armed");
            assert_footer_says_something_plain(&view, &format!("X on {what}"));
            nothing_notes.insert(note(&view));
        }
    }

    assert!(owned_heavy > 0, "fixture: no owned heavy row is listed");
    assert!(display > 0, "fixture: no display-only row is listed");
    assert!(heavy_single > 0, "fixture: no heavy single row is listed");
    assert!(heavy_counted > 0, "fixture: the heavy DeepNap row is not listed");

    for (a_name, a, b_name, b) in [
        ("already the player's", &owned_notes, "nothing to count", &nothing_notes),
        ("already the player's", &owned_notes, "takes a deliberate Enter", &deliberate_notes),
        ("nothing to count", &nothing_notes, "takes a deliberate Enter", &deliberate_notes),
    ] {
        let shared: Vec<&String> = a.intersection(b).collect();
        assert!(
            shared.is_empty(),
            "the note for {a_name:?} and the note for {b_name:?} read the same ({shared:?}); \
             three different reasons are three different notes"
        );
    }
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
// TUI-NAV-085 — a focused section with nothing listed
// ---------------------------------------------------------------------------

/// A young meadow whose Sky lists nothing: every site already reached, and the
/// sky's summoning row gated shut. Nothing else about it matters here.
fn sky_emptied() -> GameState {
    let mut s = fresh();
    s.ideas.insert(IdeaId::SkywardLonging);
    s.sites_reached = SiteId::ALL.into_iter().collect();
    assert!(
        rows(&s, Section::Sky).iter().all(|r| !r.selectable()),
        "fixture: the Sky lists no selectable row"
    );
    s
}

/// The views a player can be standing in with the Sky in focus and nothing
/// listed there: the cursor remembering the row that left, or remembering
/// nothing; a report banner showing or not; the inspector called for or not.
/// No field or full-screen overlay holds the keyboard in any of them.
fn standing_in_an_empty_sky() -> Vec<(String, ViewState)> {
    let mut out = Vec::new();
    for memory in [Some(RowKey::Site(SiteId::ALL[0])), None] {
        for banner in [false, true] {
            for inspector in [false, true] {
                let mut v = ViewState { section: Some(Section::Sky), ..Default::default() };
                v.focus[Section::Sky.index()] = memory.clone();
                if banner {
                    v.reports = vec![PendingReport { entry: 0, dismissed: false }];
                }
                v.inspector_wanted = inspector;
                out.push((
                    format!("memory={memory:?} banner={banner} inspector={inspector}"),
                    v,
                ));
            }
        }
    }
    out
}

/// The view as a string with its footer set aside: everything a key could
/// change besides the note it leaves.
fn all_but_the_footer(view: &ViewState) -> String {
    let mut v = view.clone();
    v.footer = None;
    format!("{v:?}")
}

/// With the section in focus listing nothing and no field or full-screen
/// overlay holding the keyboard, every row key — Enter, x, X, + and =, - and
/// Backspace — finds no row, asks the core for nothing, changes nothing, and
/// leaves one footer note, the same for each key: a bound key that cannot act
/// now says so rather than reading as a lost keypress.
///
/// @spec TUI-NAV-085
#[test]
fn with_nothing_listed_every_row_key_leaves_one_note_and_does_nothing_else() {
    let state = sky_emptied();
    let row_keys: [(&str, KeyEvent); 7] = [
        ("Enter", key(KeyCode::Enter)),
        ("x", key(KeyCode::Char('x'))),
        ("X", KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT)),
        ("+", key(KeyCode::Char('+'))),
        ("=", key(KeyCode::Char('='))),
        ("-", key(KeyCode::Char('-'))),
        ("Backspace", key(KeyCode::Backspace)),
    ];

    let mut notes: BTreeSet<String> = BTreeSet::new();
    for (context, view) in standing_in_an_empty_sky() {
        for (name, k) in row_keys {
            let mut v = view.clone();
            let before = all_but_the_footer(&v);
            let out = press(k, &mut v, &state);

            assert_eq!(
                out,
                Outcome::Handled,
                "{name} in an empty focused section ({context}) asked the core for {out:?}; there \
                 is no row for it to act on"
            );
            assert_eq!(
                all_but_the_footer(&v),
                before,
                "{name} in an empty focused section ({context}) changed the view; it finds no row \
                 and does nothing but leave its note"
            );
            assert_footer_says_something_plain(
                &v,
                &format!("{name} in an empty focused section ({context})"),
            );
            let (note, _) = v.footer.clone().expect("checked just above");
            notes.insert(note);
        }
    }
    assert_eq!(
        notes.len(),
        1,
        "the row keys in an empty focused section left different notes, {notes:?}; they find the \
         same nothing and say the same thing"
    );
}

/// The emptied section in focus keeps its tab, so its own digit answers as
/// any tabbed section's does — a visit to the section in focus, with no
/// refusal note — and, being a navigation key, it retires the showing report
/// banner as the Meadow's own digit does in the Meadow.
///
/// @spec TUI-NAV-085
#[test]
fn the_empty_focused_sections_own_digit_is_a_visit_not_a_refusal() {
    let state = sky_emptied();
    for (context, view) in standing_in_an_empty_sky() {
        let mut v = view.clone();
        let out = press_char(Section::Sky.digit(), &mut v, &state);

        assert_eq!(out, Outcome::Handled, "the Sky's digit asked the core for {out:?} ({context})");
        assert_eq!(
            v.section,
            Some(Section::Sky),
            "the Sky's own digit, pressed in the emptied Sky, left it for {:?} ({context})",
            v.section
        );
        assert!(
            v.footer.is_none(),
            "the Sky's own digit, pressed while the Sky holds focus with its tab shown, answered \
             with the no-tab note {:?} ({context}); a section in focus has its tab, and its digit \
             is a visit",
            v.footer.as_ref().map(|(m, _)| m)
        );

        // The Meadow's digit in the Meadow: the same press on a section that
        // lists rows, for the banner's answer to be measured against.
        let mut home = view.clone();
        home.section = Some(Section::Meadow);
        press_char(Section::Meadow.digit(), &mut home, &state);
        assert_eq!(
            format!("{:?}", v.reports),
            format!("{:?}", home.reports),
            "the Sky's own digit left the report queue differently from any tabbed section's own \
             digit ({context})"
        );
    }
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
    // The first row the cursor can land on: a group header above it is a
    // signpost, not a row (TUI-NAV-099).
    assert!(
        matches!(
            list.iter().find(|r| r.selectable()).map(|r| &r.key),
            Some(RowKey::Content(id, "")) if state.availability(*id) == Availability::Teased
        ),
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
    // The first row is the first one the cursor can land on: a group header
    // above it is a signpost, not a row (TUI-NAV-099).
    let first = rows(&state, Section::Meadow)
        .into_iter()
        .find(|r| r.selectable())
        .expect("fixture: a new meadow has rows")
        .key;

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

/// Where a section's list opens with a group header, the row TUI-NAV-032 calls
/// the section's first is the first row beneath that header: the header is a
/// line the cursor never lands on. Quantified over every several-group section
/// that lists anything in a new meadow and in a maximal one — each must open
/// with its first group's header, and a first visit must land beneath it, on
/// the first row the player can act on or, failing that, the first row.
///
/// @spec TUI-NAV-099
#[test]
fn a_first_visit_lands_beneath_the_header_that_opens_a_sections_list() {
    let mut checked = 0;
    for (label, state) in [("a new meadow", fresh()), ("a maximal meadow", rich())] {
        for section in Section::ALL {
            if section == Section::Crafts {
                continue; // one group by design: no header to land beneath
            }
            let list = rows(&state, section);
            if !list.iter().any(|r| r.selectable()) {
                continue;
            }
            assert!(
                matches!(list[0].kind, RowKind::Group),
                "in {label}, {}'s list opens with {:?} rather than its first group's header",
                section.title(),
                list[0].key
            );

            let mut view = ViewState::default();
            update(Action::SectionJump(section), &mut view, &state);
            let landed = view.focus[section.index()]
                .clone()
                .unwrap_or_else(|| panic!("in {label}, entering {} left no cursor", section.title()));
            let at = list
                .iter()
                .position(|r| r.key == landed)
                .unwrap_or_else(|| panic!("in {label}, entering {} focused {landed:?}, not a row of it", section.title()));
            assert!(
                at >= 1 && list[at].selectable(),
                "in {label}, entering {} landed on the header line itself",
                section.title()
            );
            checked += 1;
        }
    }

    // The fallback: a new meadow's home lists nothing but its verbs, so its
    // first row — the one a visit lands on — is the line directly beneath the
    // header that opens it.
    let state = fresh();
    let list = rows(&state, Section::Meadow);
    let mut view = ViewState::default();
    update(Action::SectionJump(Section::Meadow), &mut view, &state);
    assert_eq!(
        view.focus[Section::Meadow.index()].as_ref(),
        list.get(1).map(|r| &r.key),
        "a first visit to a new meadow's home must land on the row directly beneath its header"
    );
    assert!(checked >= 2, "fixture: too few sections listed anything to check");
}

// ---------------------------------------------------------------------------
// The two fields — TUI-NAV-037, TUI-NAV-038, TUI-NAV-058, TUI-NAV-059,
// TUI-NAV-060, TUI-NAV-061, TUI-NAV-064, TUI-NAV-065
//
// A field (the count prompt, or the palette) holds the keyboard. These tests
// drive it the way a player does — a key resolved against the view, then folded
// in — because the claims are about what a *keypress* does while a field is
// open, and a test that fed the reducer hand-picked actions could pass with the
// key map and the reducer disagreeing about what the key meant.
//
// The palette's ranking and its result set are `tests/palette.rs`'s to pin; the
// tests here need only that a structure's own name aims the palette at
// *something*, and read where the jump landed rather than predicting it.
// ---------------------------------------------------------------------------

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// One keypress as the player makes it: resolved against the view, then folded.
fn press(k: KeyEvent, view: &mut ViewState, state: &GameState) -> Outcome {
    let action = resolve(k, view);
    update(action, view, state)
}

fn press_char(c: char, view: &mut ViewState, state: &GameState) -> Outcome {
    press(key(KeyCode::Char(c)), view, state)
}

fn type_text(text: &str, view: &mut ViewState, state: &GameState) {
    for c in text.chars() {
        press_char(c, view, state);
    }
}

fn palette_query(view: &ViewState) -> Option<String> {
    match &view.overlay {
        Some(Overlay::Palette { query, .. }) => Some(query.clone()),
        _ => None,
    }
}

fn prompt_digits(view: &ViewState) -> Option<String> {
    view.prompt.as_ref().map(|p| p.digits.clone())
}

/// Everything the view holds, as text, less the time of the last Enter. That
/// one field is bookkeeping for telling a held key from a fresh press — no
/// player can see it — so "nothing happened" is asserted over everything else.
fn observable(view: &ViewState) -> String {
    let mut v = view.clone();
    v.last_repeat = None;
    format!("{v:?}")
}

/// A name the palette is certain to find: a revealed structure's own.
fn structure_name(ix: usize) -> String {
    clowder_core::content::structure_def(StructureId::ALL[ix]).name.to_string()
}

/// A query no name in the game contains.
const NO_MATCH: &str = "~~~~";

/// The same command, carrying a typed count where it takes one.
fn with_count(command: Command, n: u32) -> Command {
    match command {
        Command::Pounce(_) => Command::Pounce(n),
        Command::Trade(colony, _) => Command::Trade(colony, n),
        Command::DeepNap(_) => Command::DeepNap(n),
        other => other,
    }
}

/// A row the count prompt opens on.
struct Countable {
    what: &'static str,
    section: Section,
    key: RowKey,
    /// What a plain Enter on the row asks for — the row's primary action.
    primary: Command,
    count_taking: bool,
    heavy: bool,
}

/// Every kind of row `x` opens a count prompt on: repeat-N rows, count-taking
/// rows (the heavy one included), and `+`/`-` count rows — so a claim about
/// "the count prompt" is quantified over every row it can sit on.
fn countable_rows(state: &GameState) -> Vec<Countable> {
    let mut out = Vec::new();
    for (what, section, count_taking, heavy, pred) in [
        ("a repeat-N build row", Section::Meadow, false, false, (|c: &Command| matches!(c, Command::Build(_))) as fn(&Command) -> bool),
        ("a repeat-N craft row", Section::Crafts, false, false, |c: &Command| matches!(c, Command::Craft(_))),
        ("a count-taking row", Section::Clowder, true, false, |c: &Command| matches!(c, Command::Pounce(_))),
        ("a count-taking trade row", Section::Friends, true, false, |c: &Command| matches!(c, Command::Trade(..))),
        ("the heavy count-taking nap row", Section::Dreams, true, true, |c: &Command| matches!(c, Command::DeepNap(_))),
    ] {
        let (key, primary) = find_row(state, section, pred);
        out.push(Countable { what, section, key, primary, count_taking, heavy });
    }
    for (what, section) in [("a job count row", Section::Clowder), ("a tinkerer count row", Section::Crafts)] {
        let (key, increment, _) = find_count_row(state, section);
        out.push(Countable { what, section, key, primary: increment, count_taking: false, heavy: false });
    }
    out
}

/// Hold Enter: a stream of presses far denser than any terminal's key repeat,
/// every one of which must be spent on nothing.
fn hold_enter(view: &mut ViewState, state: &GameState, what: &str) {
    let start = Instant::now();
    let mut presses = 0u32;
    while start.elapsed() < Duration::from_millis(400) {
        presses += 1;
        let out = press(key(KeyCode::Enter), view, state);
        assert_eq!(
            out,
            Outcome::Handled,
            "{what}: the Enter that accepted the field was still held, and auto-repeat press \
             {presses} ({:?} in) told the core {out:?}. Key-repeat is a machine typing, not a \
             player deciding — a held Enter after a field must not start acting on the row the \
             field returned to",
            start.elapsed()
        );
    }
    assert!(
        presses > 50,
        "fixture: only {presses} presses in 400ms — too sparse to model a held key"
    );
}

/// A distinct Enter: a real pause with no key in it, then one press. Escalates
/// rather than guessing the threshold, as `commit_deliberately` does.
fn act_deliberately(view: &mut ViewState, state: &GameState) -> Outcome {
    for _ in 0..4 {
        std::thread::sleep(Duration::from_millis(300));
        let out = press(key(KeyCode::Enter), view, state);
        if out != Outcome::Handled {
            return out;
        }
    }
    Outcome::Handled
}

/// The palette's query is the player's typing, verbatim: every printable
/// character lands in it — letters, digits, symbols, and Space — and none of
/// them reaches its global meaning or tells the core anything.
///
/// @spec TUI-NAV-037
#[test]
fn the_palette_takes_every_printable_character_into_its_query() {
    let state = rich();
    let (row, _) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));

    for c in ' '..='~' {
        let mut view = focused(Section::Meadow, row.clone());
        press_char('/', &mut view, &state);
        assert_eq!(palette_query(&view).as_deref(), Some(""), "fixture: / opens an empty palette");

        let out = press_char(c, &mut view, &state);
        assert_eq!(out, Outcome::Handled, "typing {c:?} into the palette told the core {out:?}");
        assert_eq!(
            palette_query(&view),
            Some(c.to_string()),
            "{c:?} typed into the palette did not land in its query; the palette takes every \
             printable character, digits and Space included, as query text"
        );
        assert_eq!(view.section, Some(Section::Meadow), "{c:?} typed into the palette moved the section");
        assert_eq!(view.focus[Section::Meadow.index()].as_ref(), Some(&row), "{c:?} moved the cursor");
        assert!(view.prompt.is_none(), "{c:?} typed into the palette opened a count prompt");
        assert!(view.footer.is_none(), "{c:?} typed into the palette earned a footer note");
    }

    let everything: String = (' '..='~').collect();
    let mut view = focused(Section::Meadow, row);
    press_char('/', &mut view, &state);
    type_text(&everything, &mut view, &state);
    assert_eq!(
        palette_query(&view),
        Some(everything),
        "the query must be exactly what was typed, in order, spaces and all"
    );
}

/// The count prompt's field is a number: the ten digits type into it, and
/// every other printable key — Space included — does nothing at all. Not a
/// character in the count, and not a footer note either: a key with nothing to
/// mean in its context is silent (TUI-NAV-042), and a count is digits only.
///
/// A digit that would carry the count past the shell-side cap is the one
/// exception (TUI-NAV-068): it is refused with a footer note rather than
/// silently ignored, so once the accumulating count reaches the cap the rest
/// of this run's digits are checked against that instead.
///
/// @spec TUI-NAV-037, TUI-NAV-068
#[test]
fn the_count_prompt_takes_the_digits_and_silently_ignores_every_other_printable() {
    let state = rich();
    for row in countable_rows(&state) {
        let mut view = focused(row.section, row.key.clone());
        press_char('x', &mut view, &state);
        assert_eq!(
            prompt_digits(&view).as_deref(),
            Some(""),
            "fixture: x on {} opens an empty count prompt",
            row.what
        );

        let mut expected = String::new();
        // Once a digit is refused for carrying the count past the cap, the
        // refusal's footer note stands (nothing past this point clears it)
        // — an ignored non-digit key does not clear a footer any more than
        // a taken digit does — so every remaining key in this single
        // ascending pass over the printable range is checked against that,
        // not against silence. @spec TUI-NAV-068
        let mut past_cap = false;
        for c in ' '..='~' {
            let out = press_char(c, &mut view, &state);
            if c.is_ascii_digit() && !past_cap {
                let candidate = format!("{expected}{c}");
                if candidate.parse::<u64>().is_ok_and(|n| n <= MAX_AFFORDABLE_CAP as u64) {
                    expected = candidate;
                } else {
                    past_cap = true;
                }
            }
            assert_eq!(out, Outcome::Handled, "on {}, {c:?} in the count prompt told the core {out:?}", row.what);
            assert_eq!(
                prompt_digits(&view),
                Some(expected.clone()),
                "on {}, after {c:?} the count read {:?}; the count prompt takes the digits 0–9 \
                 up to the cap and ignores every other printable character and Space",
                row.what,
                prompt_digits(&view)
            );
            if past_cap {
                assert!(
                    view.footer.is_some(),
                    "on {}, past the cap, {c:?} in the count prompt left no footer note \
                     (TUI-NAV-068)",
                    row.what
                );
            } else {
                assert!(
                    view.footer.is_none(),
                    "on {}, {c:?} in the count prompt earned a footer note; a key with nothing to \
                     mean there is silent",
                    row.what
                );
            }
            assert!(view.overlay.is_none(), "on {}, {c:?} in the count prompt opened an overlay", row.what);
            assert_eq!(view.section, Some(row.section), "on {}, {c:?} in the count prompt moved the section", row.what);
            assert_eq!(view.focus[row.section.index()].as_ref(), Some(&row.key), "on {}, {c:?} moved the cursor", row.what);
        }
    }
}

/// The count prompt's Enter reaches the row beneath only as the field directs
/// — ×N, or nothing for an empty or zero count — and never as the row's own
/// primary action.
///
/// @spec TUI-NAV-037
#[test]
fn the_count_prompts_enter_reaches_the_row_only_as_its_count() {
    let state = rich();
    for row in countable_rows(&state) {
        for typed in ["", "0", "00", "3", "12"] {
            let mut view = focused(row.section, row.key.clone());
            press_char('x', &mut view, &state);
            type_text(typed, &mut view, &state);
            let out = press(key(KeyCode::Enter), &mut view, &state);

            let n: u32 = typed.parse().unwrap_or(0);
            let expected = if n == 0 || (row.count_taking && row.heavy) {
                Outcome::Handled
            } else if row.count_taking {
                Outcome::Apply(with_count(row.primary, n))
            } else {
                Outcome::Repeat(row.primary, n)
            };
            assert_eq!(
                out, expected,
                "on {} with {typed:?} typed, the count prompt's Enter asked for {out:?}; the \
                 field's Enter reaches the row only as its own acceptance directs — ×N, or \
                 nothing on an empty or zero count",
                row.what
            );
        }
    }
}

/// An armed heavy row can sit beneath an open field — arm, then `x` on the nap
/// row, or arm and then `/`. The field's Enter goes to the field, so it can
/// never be the second Enter that commits the row beneath. The armed row's
/// last press is set well in the past, so a commit here would count as
/// deliberate: only the field's claim on Enter stands between it and the core.
///
/// @spec TUI-NAV-037
#[test]
fn enter_in_a_field_never_commits_an_armed_row_beneath_it() {
    let state = rich();
    let aimed = structure_name(0);

    for (name, section, key_, command) in heavy_rows(&state) {
        for query in ["", "   ", NO_MATCH, aimed.as_str()] {
            let mut view = focused(section, key_.clone());
            update(Action::Activate, &mut view, &state);
            assert!(view.armed.is_some(), "fixture: the first Enter arms the {name} row");
            press_char('/', &mut view, &state);
            assert!(palette_query(&view).is_some(), "fixture: / opens the palette over an armed row");
            type_text(query, &mut view, &state);
            view.last_repeat = None;

            let out = press(key(KeyCode::Enter), &mut view, &state);
            assert!(
                !matches!(out, Outcome::Apply(_) | Outcome::Repeat(..)),
                "with the palette open over the armed {name} row and {query:?} typed, Enter \
                 asked the core for {out:?} (the armed command is {command:?}); the palette's \
                 Enter is the palette's, and never commits the row beneath it"
            );
        }
    }

    // The nap row is the one heavy row `x` opens a count prompt on.
    let (nap, _) = find_row(&state, Section::Dreams, |c| matches!(c, Command::DeepNap(_)));
    for typed in ["", "0", "3"] {
        let mut view = focused(Section::Dreams, nap.clone());
        update(Action::Activate, &mut view, &state);
        assert!(view.armed.is_some(), "fixture: the first Enter arms the nap row");
        press_char('x', &mut view, &state);
        assert!(view.prompt.is_some(), "fixture: x opens a count prompt over the armed nap row");
        type_text(typed, &mut view, &state);
        view.last_repeat = None;

        let out = press(key(KeyCode::Enter), &mut view, &state);
        assert_eq!(
            out,
            Outcome::Handled,
            "with a count prompt over the armed nap row and {typed:?} typed, Enter asked the core \
             for {out:?}; the count prompt's Enter sets the count and arms, and never commits"
        );
    }
}

/// Esc closes the field and nothing happened: the view is exactly what it was
/// before the field opened — including an armed row beneath, which Esc on the
/// field must not take with it, since the field is the topmost transient.
///
/// @spec TUI-NAV-037, TUI-NAV-038, TUI-NAV-030
#[test]
fn esc_closes_a_field_and_leaves_everything_as_it_was() {
    let state = rich();
    let (build, _) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));

    let mut beneath: Vec<(String, ViewState)> =
        vec![("an ordinary row".to_string(), focused(Section::Meadow, build))];
    for (name, section, key_, _) in heavy_rows(&state) {
        let mut v = focused(section, key_);
        update(Action::Activate, &mut v, &state);
        assert!(v.armed.is_some(), "fixture: the first Enter arms the {name} row");
        beneath.push((format!("the armed {name} row"), v));
    }

    for (what, before) in &beneath {
        let mut view = before.clone();
        press_char('/', &mut view, &state);
        type_text("ab 1q", &mut view, &state);
        let out = press(key(KeyCode::Esc), &mut view, &state);
        assert_eq!(out, Outcome::Handled, "Esc on the palette over {what} told the core {out:?}");
        assert_eq!(
            observable(&view),
            observable(before),
            "Esc on the palette over {what} did not leave the view as it was before the palette \
             opened; Esc cancels a field with no effect"
        );
    }

    for row in countable_rows(&state) {
        let before = focused(row.section, row.key.clone());
        let mut view = before.clone();
        press_char('x', &mut view, &state);
        type_text("12", &mut view, &state);
        let out = press(key(KeyCode::Esc), &mut view, &state);
        assert_eq!(out, Outcome::Handled, "Esc on the count prompt over {} told the core {out:?}", row.what);
        assert_eq!(
            observable(&view),
            observable(&before),
            "Esc on the count prompt over {} did not leave the view as it was",
            row.what
        );
    }

    let (nap, _) = find_row(&state, Section::Dreams, |c| matches!(c, Command::DeepNap(_)));
    let mut before = focused(Section::Dreams, nap);
    update(Action::Activate, &mut before, &state);
    let mut view = before.clone();
    press_char('x', &mut view, &state);
    type_text("3", &mut view, &state);
    press(key(KeyCode::Esc), &mut view, &state);
    assert_eq!(
        observable(&view),
        observable(&before),
        "Esc on a count prompt over the armed nap row did not leave the armed row as it was"
    );
}

/// Backspace inside a field edits the field: it deletes the last character,
/// does nothing on an empty field, and never fires its global meaning — which
/// on a count row is "unassign one". The count row is the fixture on purpose:
/// it is the one row where a leaked Backspace would change the game.
///
/// @spec TUI-NAV-058
#[test]
fn backspace_in_a_field_deletes_its_last_character_and_never_decrements() {
    let state = rich();
    let (count_key, _, decrement) = find_count_row(&state, Section::Clowder);

    let mut view = focused(Section::Clowder, count_key.clone());
    press_char('x', &mut view, &state);
    type_text("123", &mut view, &state);
    assert_eq!(prompt_digits(&view).as_deref(), Some("123"), "fixture: the count reads 123");
    for expected in ["12", "1", "", ""] {
        let out = press(key(KeyCode::Backspace), &mut view, &state);
        assert!(!would_fire(&out, decrement), "Backspace in the count prompt unassigned a worker");
        assert_eq!(out, Outcome::Handled, "Backspace in the count prompt told the core {out:?}");
        assert_eq!(
            prompt_digits(&view).as_deref(),
            Some(expected),
            "Backspace in the count prompt should leave {expected:?}; it deletes the last \
             character, and on an empty field does nothing (the prompt stays open)"
        );
    }

    let mut view = focused(Section::Clowder, count_key.clone());
    press_char('/', &mut view, &state);
    type_text("ab c", &mut view, &state);
    assert_eq!(
        palette_query(&view).as_deref(),
        Some("ab c"),
        "the typed query did not reach the palette (TUI-NAV-037), so there is nothing to erase"
    );
    for expected in ["ab ", "ab", "a", "", ""] {
        let out = press(key(KeyCode::Backspace), &mut view, &state);
        assert!(!would_fire(&out, decrement), "Backspace in the palette unassigned a worker");
        assert_eq!(out, Outcome::Handled, "Backspace in the palette told the core {out:?}");
        assert_eq!(
            palette_query(&view).as_deref(),
            Some(expected),
            "Backspace in the palette should leave {expected:?}; it deletes the last character, \
             and on an empty field does nothing (the palette stays open)"
        );
    }

    for field in ['x', '/'] {
        let mut view = focused(Section::Clowder, count_key.clone());
        press_char(field, &mut view, &state);
        let before = observable(&view);
        let out = press(key(KeyCode::Backspace), &mut view, &state);
        assert_eq!(out, Outcome::Handled);
        assert_eq!(
            observable(&view),
            before,
            "Backspace on an empty field (opened with {field:?}) changed something; on an empty \
             field it does nothing"
        );
    }
}

/// The field holds the keyboard, so the keys that would move the cursor or the
/// section do nothing while it is open: the row the field will act on stays
/// the row under it.
///
/// @spec TUI-NAV-059
#[test]
fn arrows_tab_and_backtab_move_nothing_beneath_a_field() {
    let state = rich();
    let builds: Vec<RowKey> = rows(&state, Section::Meadow)
        .into_iter()
        .filter(|r| matches!(r.kind, RowKind::Act { command: Command::Build(_), .. }))
        .map(|r| r.key)
        .collect();
    assert!(builds.len() > 4, "fixture: the Meadow lists rows above and below the cursor");
    let middle = builds[builds.len() / 2].clone();

    for (field, opener, typed) in [("the count prompt", 'x', "4"), ("the palette", '/', "ab")] {
        let mut view = focused(Section::Meadow, middle.clone());
        press_char(opener, &mut view, &state);
        type_text(typed, &mut view, &state);
        let text = |v: &ViewState| prompt_digits(v).or_else(|| palette_query(v));
        let before = text(&view);
        assert!(before.is_some(), "fixture: {field} is open");

        for code in [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right, KeyCode::Tab, KeyCode::BackTab] {
            let out = press(key(code), &mut view, &state);
            assert_eq!(out, Outcome::Handled, "{code:?} with {field} open told the core {out:?}");
            assert_eq!(view.section, Some(Section::Meadow), "{code:?} with {field} open moved the section");
            assert_eq!(
                view.focus[Section::Meadow.index()].as_ref(),
                Some(&middle),
                "{code:?} with {field} open moved the list cursor beneath it"
            );
            assert_eq!(text(&view), before, "{code:?} with {field} open changed or closed the field");
            assert!(view.footer.is_none(), "{code:?} with {field} open earned a footer note");
        }
    }
}

/// The count prompt's Enter always closes it — applying the count, or
/// cancelling on an empty or zero one. It never leaves the player in a field
/// that Enter could not get them out of.
///
/// @spec TUI-NAV-060
#[test]
fn the_count_prompts_enter_always_closes_it() {
    let state = rich();
    for row in countable_rows(&state) {
        for typed in ["", "0", "00", "4", "12"] {
            let mut view = focused(row.section, row.key.clone());
            press_char('x', &mut view, &state);
            type_text(typed, &mut view, &state);
            press(key(KeyCode::Enter), &mut view, &state);
            assert!(
                view.prompt.is_none(),
                "on {} with {typed:?} typed, the count prompt's Enter left it open",
                row.what
            );
        }
    }
}

/// The palette's Enter closes it only when there is a result to jump to. With
/// no query, a blank one, or one that matches nothing, the palette stays open —
/// the player's way out is Esc.
///
/// @spec TUI-NAV-060, TUI-NAV-064
#[test]
fn the_palettes_enter_closes_it_only_when_there_is_a_result_to_jump_to() {
    let state = rich();
    let (build, _) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));

    for ix in 0..5 {
        let aimed = structure_name(ix);
        let mut view = focused(Section::Meadow, build.clone());
        press_char('/', &mut view, &state);
        type_text(&aimed, &mut view, &state);
        press(key(KeyCode::Enter), &mut view, &state);
        assert!(
            view.overlay.is_none(),
            "Enter with the query {aimed:?} — a revealed structure's own name — left the palette \
             open; with a result to jump to, Enter closes it"
        );
    }

    for query in ["", " ", "   ", NO_MATCH] {
        let mut view = focused(Section::Meadow, build.clone());
        press_char('/', &mut view, &state);
        type_text(query, &mut view, &state);
        press(key(KeyCode::Enter), &mut view, &state);
        assert_eq!(
            palette_query(&view).as_deref(),
            Some(query),
            "Enter with the query {query:?} closed the palette or changed its query; with no \
             result to jump to it stays open, as typed"
        );
    }
}

/// With nothing aimed at — no query, only spaces, or no match — the palette's
/// Enter does nothing at all. Not a jump the player did not aim, not the row
/// beneath's action, not a commit of an armed row: nothing.
///
/// @spec TUI-NAV-064
#[test]
fn the_palettes_enter_with_nothing_aimed_at_does_nothing() {
    let state = rich();
    let (build, _) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));
    let (count_key, _, _) = find_count_row(&state, Section::Clowder);

    let mut beneath: Vec<(String, ViewState)> = vec![
        ("an ordinary row".to_string(), focused(Section::Meadow, build)),
        ("a count row".to_string(), focused(Section::Clowder, count_key)),
    ];
    for (name, section, key_, _) in heavy_rows(&state) {
        let mut v = focused(section, key_);
        update(Action::Activate, &mut v, &state);
        beneath.push((format!("the armed {name} row"), v));
    }

    for (what, context) in &beneath {
        for query in ["", " ", "    ", NO_MATCH, "#%^~"] {
            let mut view = context.clone();
            press_char('/', &mut view, &state);
            type_text(query, &mut view, &state);
            view.last_repeat = None;
            let before = observable(&view);

            let out = press(key(KeyCode::Enter), &mut view, &state);
            assert_eq!(
                out,
                Outcome::Handled,
                "over {what}, Enter in the palette with {query:?} asked the core for {out:?}; \
                 with nothing aimed at, Enter does nothing"
            );
            assert_eq!(
                observable(&view),
                before,
                "over {what}, Enter in the palette with {query:?} changed the view; with nothing \
                 aimed at, the palette stays open with its query as typed and nothing else moves"
            );
        }
    }
}

/// The Enter that accepts a field is spent on the field. If the key is still
/// held, its auto-repeat must not start acting on the row the field returns to
/// — only a distinct Enter does, judged as a heavy row's commit is.
///
/// Quantified over every row kind the count prompt opens on, and over the row
/// a palette jump lands on.
///
/// @spec TUI-NAV-061
#[test]
fn a_held_enter_after_a_field_accepts_never_acts_on_the_row_beneath() {
    let state = rich();

    for row in countable_rows(&state) {
        let mut view = focused(row.section, row.key.clone());
        press_char('x', &mut view, &state);
        type_text("3", &mut view, &state);
        press(key(KeyCode::Enter), &mut view, &state);
        assert!(view.prompt.is_none(), "fixture: the count prompt's Enter accepted the count");

        hold_enter(&mut view, &state, &format!("after ×3 on {}", row.what));

        let expected = if row.heavy {
            Outcome::Apply(with_count(row.primary, 3))
        } else {
            Outcome::Apply(row.primary)
        };
        let out = act_deliberately(&mut view, &state);
        assert_eq!(
            out, expected,
            "after ×3 on {} and a held Enter, a distinct Enter must act on the row again",
            row.what
        );
    }

    // Start on a count row so the row beneath the palette is one Enter would
    // act on, and jump to a structure's row.
    let (count_key, _, _) = find_count_row(&state, Section::Clowder);
    let mut view = focused(Section::Clowder, count_key);
    press_char('/', &mut view, &state);
    type_text(&structure_name(3), &mut view, &state);
    let out = press(key(KeyCode::Enter), &mut view, &state);
    assert_eq!(out, Outcome::Handled, "the palette's Enter asked the core for {out:?}");
    assert!(view.overlay.is_none(), "the palette's Enter did not jump (TUI-NAV-038)");

    let section = view.section.expect("the jump landed in a section");
    let landed_key = view.focus[section.index()].clone().expect("the jump focused a row");
    let landed = rows(&state, section)
        .into_iter()
        .find(|r| r.key == landed_key)
        .expect("the jump focused a row that is on the list");
    assert!(!landed.heavy, "fixture: the jump lands on an ordinary row");
    let primary = match landed.kind {
        RowKind::Act { command, .. } => command,
        RowKind::Count { increment, .. } => increment,
        other => panic!("fixture: the jump landed on a row Enter does nothing on ({other:?})"),
    };

    hold_enter(&mut view, &state, "after a palette jump");
    let out = act_deliberately(&mut view, &state);
    assert_eq!(
        out,
        Outcome::Apply(primary),
        "after a palette jump and a held Enter, a distinct Enter must act on the landed row"
    );
}

/// The palette's jump is navigation, with everything navigation does: it
/// disarms an armed row, dismisses the report banner, and counts as a visit to
/// the section it lands in. It moves the cursor and not the layout, so whether
/// the player had asked for the inspector is untouched.
///
/// @spec TUI-NAV-065
#[test]
fn the_palettes_jump_is_navigation_and_leaves_the_layout_alone() {
    let state = rich();
    let aimed = structure_name(2);

    for inspector_wanted in [false, true] {
        let (move_key, _) = move_row(&state);
        let mut view = focused(Section::Legacy, move_key);
        view.inspector_wanted = inspector_wanted;
        view.reports = vec![PendingReport { entry: 0, dismissed: false }];
        update(Action::Activate, &mut view, &state);
        assert!(view.armed.is_some(), "fixture: the Move row is armed");

        press_char('/', &mut view, &state);
        type_text(&aimed, &mut view, &state);
        press(key(KeyCode::Enter), &mut view, &state);
        assert!(view.overlay.is_none(), "the palette's Enter did not jump (TUI-NAV-038)");
        let landed = view.section.expect("the jump landed in a section");
        assert_ne!(landed, Section::Legacy, "fixture: a structure's name lands outside Legacy");

        assert!(
            view.armed.is_none(),
            "the palette's jump left the Move row armed; the jump is navigation, and any \
             navigation disarms"
        );
        assert!(
            view.reports.iter().all(|r| r.dismissed),
            "the palette's jump left the report banner up; the jump is navigation, and the first \
             navigation dismisses the banner"
        );
        assert!(
            view.seen[landed.index()].visited,
            "the palette's jump did not count as a visit to {}",
            landed.title()
        );
        assert_eq!(
            view.seen[landed.index()].revealed_at_last_visit,
            rows(&state, landed).len(),
            "the palette's jump visited {} without catching its watermark up to what it lists",
            landed.title()
        );
        assert_eq!(
            view.inspector_wanted, inspector_wanted,
            "the palette's jump changed whether the inspector is asked for; it moves the cursor, \
             not the layout"
        );
    }
}

/// `/` opens the palette from a screen with no prompt or overlay open — an
/// armed row is neither — and nowhere else: inside a count prompt it is just a
/// non-digit, inside the Journal or help it is unbound, and inside the palette
/// it is a character of the query.
///
/// @spec TUI-NAV-038
#[test]
fn slash_opens_the_palette_only_where_no_prompt_or_overlay_is_open() {
    let state = rich();
    let (build, _) = find_row(&state, Section::Meadow, |c| matches!(c, Command::Build(_)));

    let mut view = focused(Section::Meadow, build.clone());
    let out = press_char('/', &mut view, &state);
    assert_eq!(out, Outcome::Handled);
    assert_eq!(palette_query(&view).as_deref(), Some(""), "/ on a bare screen must open an empty palette");

    let (move_key, _) = move_row(&state);
    let mut view = focused(Section::Legacy, move_key);
    update(Action::Activate, &mut view, &state);
    press_char('/', &mut view, &state);
    assert_eq!(palette_query(&view).as_deref(), Some(""), "/ beside an armed row must open the palette");

    let mut view = focused(Section::Meadow, build.clone());
    press_char('x', &mut view, &state);
    press_char('/', &mut view, &state);
    assert!(view.overlay.is_none(), "/ inside a count prompt opened the palette");
    assert_eq!(prompt_digits(&view).as_deref(), Some(""), "/ inside a count prompt changed or closed it");

    for overlay in [Overlay::Journal { anchor: 0 }, Overlay::Help] {
        let mut view = focused(Section::Meadow, build.clone());
        view.overlay = Some(overlay.clone());
        press_char('/', &mut view, &state);
        assert_eq!(view.overlay, Some(overlay.clone()), "/ inside {overlay:?} replaced it");
    }

    let mut view = focused(Section::Meadow, build);
    press_char('/', &mut view, &state);
    press_char('/', &mut view, &state);
    assert_eq!(
        palette_query(&view).as_deref(),
        Some("/"),
        "/ inside the palette must be a character of the query, not a re-open"
    );
}

// ---------------------------------------------------------------------------
// The done group (Ideas) — TUI-NAV-112, TUI-NAV-113, TUI-NAV-115, TUI-NAV-116,
// TUI-NAV-117, TUI-NAV-118, TUI-NAV-119, TUI-NAV-120, TUI-NAV-121, TUI-NAV-123
//
// The fold is the shell's own view state, read here only through what it
// changes: `drawn_rows`, the rows the list draws and the cursor visits. A row
// is finished exactly where the core's lock reason names its item already the
// player's (TUI-NAV-110), and nothing else here decides it.
//
// Where a claim turns on a command the core accepts — a row finishing under
// the cursor — it is driven through a real `App::step`, so wherever the shell
// settles the cursor after a command (in the reducer, or in the frame's own
// per-frame pass) is what is observed. No time passes in a step unless a test
// says so.
// ---------------------------------------------------------------------------

use clowder_tui::nav::rows::drawn_rows;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator};
use crossterm::event::Event;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

/// Whether the core names `row`'s item already the player's.
fn finished(state: &GameState, row: &Row) -> bool {
    owned(state, row)
}

/// Whether the core would take `row`'s item right now: it reports no lock
/// reason at all.
fn actionable(state: &GameState, row: &Row) -> bool {
    match &row.key {
        RowKey::Content(id, _) => state.lock_reason(*id).is_none(),
        _ => false,
    }
}

/// The Ideas section's rows as the list draws them under `view`'s fold.
fn drawn(state: &GameState, view: &ViewState) -> Vec<Row> {
    drawn_rows(state, Section::Ideas, view)
}

/// The drawn Ideas rows the cursor can land on, in order.
fn drawn_selectable(state: &GameState, view: &ViewState) -> Vec<RowKey> {
    drawn(state, view).into_iter().filter(|r| r.selectable()).map(|r| r.key).collect()
}

/// Whether the done group is drawn open: some finished row is drawn.
fn is_open(state: &GameState, view: &ViewState) -> bool {
    drawn(state, view).iter().any(|r| finished(state, r))
}

/// The cursor's remembered row on the Ideas screen.
fn ideas_cursor(view: &ViewState) -> Option<RowKey> {
    view.focus[Section::Ideas.index()].clone()
}

/// A pause long enough that the next press is a distinct one, by any reading
/// of TUI-NAV-051's gate.
fn gap() {
    std::thread::sleep(Duration::from_millis(250));
}

/// `d` pressed, then the repeat clock cleared, so an Enter right after it in
/// the same test is a distinct press of its own.
fn press_d(view: &mut ViewState, state: &GameState) -> Outcome {
    let out = press_char('d', view, state);
    view.last_repeat = None;
    out
}

/// Where `view`'s focused row is listed but folded into the done group, open
/// the group by its key, keeping the focus on that row — the one way a player
/// reaches such a row (TUI-NAV-113).
fn unfold_focus(state: &GameState, view: &mut ViewState) {
    let Some(section) = view.section else { return };
    let Some(key) = view.focus[section.index()].clone() else { return };
    if drawn_rows(state, section, view).iter().any(|r| r.key == key)
        || !rows(state, section).iter().any(|r| r.key == key)
    {
        return;
    }
    press_d(view, state);
    view.focus[section.index()] = Some(key);
}

/// A sitting opened on `state` and then the Ideas screen entered by its digit.
fn in_ideas(state: &GameState) -> ViewState {
    let mut v = ViewState::default();
    clowder_tui::nav::opening(&mut v, state);
    press_char('3', &mut v, state);
    v
}

/// The Ideas section's three kinds, in the order its groups stand.
fn ideas_rank(row: &Row) -> usize {
    match &row.key {
        RowKey::Content(ContentId::Idea(_), _) => 0,
        RowKey::Content(ContentId::Refinement(_), _) => 1,
        RowKey::Content(ContentId::Rule(_), _) => 2,
        other => panic!("{other:?} is not an Ideas row"),
    }
}

/// A row's place in the Ideas section's list: the place it holds in its own
/// kind's group — its kind's rank, then the order the core lists that kind.
fn ideas_place(key: &RowKey) -> (usize, usize) {
    fn at<T: PartialEq>(all: &[T], x: &T) -> usize {
        all.iter().position(|y| y == x).expect("in its kind's table")
    }
    match key {
        RowKey::Content(ContentId::Idea(i), _) => (0, at(&IdeaId::ALL, i)),
        RowKey::Content(ContentId::Refinement(r), _) => (1, at(&clowder_core::RefinementId::ALL, r)),
        RowKey::Content(ContentId::Rule(r), _) => (2, at(&RuleId::ALL, r)),
        other => panic!("{other:?} is not an Ideas row"),
    }
}

/// The row that closes up into `gone`'s place in `after`'s drawn list: the
/// first drawn row following that place, else the last drawn row before it,
/// else none (TUI-NAV-117).
fn closes_up(after: &[RowKey], gone: &RowKey) -> Option<RowKey> {
    let p = ideas_place(gone);
    after
        .iter()
        .find(|k| ideas_place(k) > p)
        .or_else(|| after.iter().rev().find(|k| ideas_place(k) < p))
        .cloned()
}

/// Where TUI-NAV-032 lands a cursor among `drawn`: the first row whose item
/// the core reports Available, else the first row.
fn landing_among(state: &GameState, drawn: &[RowKey]) -> Option<RowKey> {
    drawn
        .iter()
        .find(|k| match k {
            RowKey::Content(id, _) => {
                state.availability(*id) == Availability::Available
                    && state.lock_reason(*id) != Some(LockReason::AlreadyYours)
            }
            _ => false,
        })
        .or_else(|| drawn.first())
        .cloned()
}

/// Make `row`'s item the player's, directly.
fn finish(state: &mut GameState, row: &Row) {
    match &row.key {
        RowKey::Content(ContentId::Idea(i), _) => {
            state.ideas.insert(*i);
        }
        RowKey::Content(ContentId::Refinement(r), _) => {
            state.refinements.insert(*r);
        }
        RowKey::Content(ContentId::Rule(r), _) => {
            state.rules.insert(*r);
        }
        other => panic!("{other:?} is not an Ideas row"),
    }
}

/// The footer note a row key leaves where there is no row to act on
/// (TUI-NAV-085), read from a focused section that lists nothing.
fn nothing_to_act_on_note() -> String {
    let state = sky_emptied();
    let (_, mut v) = standing_in_an_empty_sky().remove(0);
    press(key(KeyCode::Enter), &mut v, &state);
    footer_text(&v).expect("fixture: Enter in an emptied section leaves a note")
}

/// A meadow mid-game: the open meadow with every other idea that no other idea
/// needs left unresearched, every other refinement on offer bought, and one
/// rule adopted from each of two of the four pairs — so each of the Ideas
/// section's three kinds lists several rows finished and several not, the
/// unfinished ones the core would take at once.
fn mid_game() -> GameState {
    use clowder_core::content::{idea_def, refinement_def, rule_def};
    use clowder_core::RefinementId;
    let mut s = rich();
    let leaves: Vec<IdeaId> = IdeaId::ALL
        .iter()
        .copied()
        .filter(|i| !IdeaId::ALL.iter().any(|j| idea_def(*j).prereqs.contains(i)))
        .collect();
    for (n, i) in leaves.iter().enumerate() {
        if n % 2 == 0 {
            s.ideas.remove(i);
        }
    }
    let mut n = 0;
    for r in RefinementId::ALL {
        if s.ideas.contains(&refinement_def(r).gate) {
            if n % 2 == 0 {
                s.refinements.insert(r);
            }
            n += 1;
        }
    }
    let mut paired: Vec<RuleId> = Vec::new();
    let mut pair = 0;
    for r in RuleId::ALL {
        if paired.contains(&r) {
            continue;
        }
        paired.push(r);
        paired.push(rule_def(r).blocks);
        if pair % 2 == 0 {
            s.rules.insert(r);
        }
        pair += 1;
    }
    let list = rows(&s, Section::Ideas);
    for rank in 0..3 {
        let of_kind: Vec<&Row> = list.iter().filter(|r| r.selectable() && ideas_rank(r) == rank).collect();
        let done = of_kind.iter().filter(|r| finished(&s, r)).count();
        let ready = of_kind.iter().filter(|r| actionable(&s, r)).count();
        assert!(
            done >= 2 && ready >= 2,
            "fixture: mid-game, Ideas kind {rank} lists {done} finished and {ready} ready rows; \
             several of each are needed"
        );
    }
    s
}

/// The open meadow with every idea researched, every refinement bought, and
/// one rule of every pair adopted: every row the Ideas section lists is
/// finished.
fn all_ideas_done() -> GameState {
    use clowder_core::content::rule_def;
    let mut s = rich();
    s.refinements = clowder_core::RefinementId::ALL.iter().copied().collect();
    let mut paired: Vec<RuleId> = Vec::new();
    for r in RuleId::ALL {
        if !paired.contains(&r) {
            paired.push(r);
            paired.push(rule_def(r).blocks);
            s.rules.insert(r);
        }
    }
    let list = rows(&s, Section::Ideas);
    assert!(list.iter().any(|r| r.selectable()), "fixture: the Ideas section lists rows");
    assert!(
        list.iter().filter(|r| r.selectable()).all(|r| finished(&s, r)),
        "fixture: every row the Ideas section lists is finished"
    );
    s
}

/// The open meadow before anything on the Ideas screen is done: rows listed,
/// none finished, so no done group is listed.
fn no_ideas_done() -> GameState {
    let mut s = rich();
    s.ideas = BTreeSet::new();
    s.refinements = BTreeSet::new();
    s.rules = BTreeSet::new();
    let list = rows(&s, Section::Ideas);
    assert!(list.iter().any(|r| r.selectable()), "fixture: the Ideas section lists rows");
    assert!(!list.iter().any(|r| finished(&s, r)), "fixture: no Ideas row is finished");
    s
}

/// A unique save path under the build's own scratch directory.
fn dg_scratch(name: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("clowder-done-{}-{name}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

fn still() -> Reading {
    Reading { monotonic: Duration::ZERO, wall: Duration::ZERO }
}

fn ev(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn ev_char(c: char) -> Event {
    ev(KeyCode::Char(c))
}

fn step(a: &mut App, t: &mut Terminal<TestBackend>, input: &[Event]) {
    a.step(t, still(), input).expect("a step against a TestBackend does not fail");
}

/// A session opened on `state` at a three-pane size, its first frame drawn,
/// and the Ideas screen entered by its digit.
fn ideas_session(state: GameState, name: &str) -> (App, Terminal<TestBackend>) {
    let mut view = ViewState::default();
    clowder_tui::nav::opening(&mut view, &state);
    let mut a = App {
        state,
        view,
        clock: TickAccumulator::default(),
        save_path: dg_scratch(name),
        last_save: Instant::now(),
        prev_layout: None,
        quitting: false,
        save_failed: false,
    };
    let mut t = Terminal::new(TestBackend::new(130, 40)).expect("TestBackend is infallible");
    step(&mut a, &mut t, &[]);
    step(&mut a, &mut t, &[ev_char('3')]);
    assert_eq!(a.view.section, Some(Section::Ideas), "fixture: 3 entered the Ideas screen");
    (a, t)
}

/// `d` pressed in a session, then the repeat clock cleared.
fn session_d(a: &mut App, t: &mut Terminal<TestBackend>) {
    step(a, t, &[ev_char('d')]);
    a.view.last_repeat = None;
}

// ---- TUI-NAV-112 -----------------------------------------------------------

/// Every sitting opens with the done group folded — a new game and a load
/// alike — and nothing of its state reaches the save: a save written with the
/// group open loads as the same game a save written with it folded does, and
/// a sitting opened from it starts folded again.
///
/// @spec TUI-NAV-112
#[test]
fn every_sitting_opens_with_the_done_group_folded_and_the_save_holds_nothing_of_it() {
    let state = mid_game();
    let loaded = clowder_core::load_string(&clowder_core::save_string(&state, 0), 0)
        .expect("fixture: the meadow round-trips through a save")
        .state;
    for (label, s) in [("a new game", &state), ("a load", &loaded)] {
        let v = in_ideas(s);
        assert!(
            rows(s, Section::Ideas).iter().any(|r| finished(s, r)),
            "fixture: {label} lists finished Ideas rows"
        );
        assert!(!is_open(s, &v), "{label} opened with the done group open; every sitting opens it folded");
    }

    let (mut a, mut t) = ideas_session(state, "save");
    let path = a.save_path.clone();
    a.save().expect("fixture: the save is written");
    let folded_save = std::fs::read_to_string(&path).expect("fixture: the save is read back");
    session_d(&mut a, &mut t);
    assert!(is_open(&a.state, &a.view), "d did not open the done group (TUI-NAV-115)");
    a.save().expect("fixture: the save is written");
    let open_save = std::fs::read_to_string(&path).expect("fixture: the save is read back");
    let as_game = |text: &str| {
        let g = clowder_core::load_string(text, 0).expect("the save loads").state;
        clowder_core::save_string(&g, 0)
    };
    assert_eq!(
        as_game(&open_save),
        as_game(&folded_save),
        "a save written with the done group open loads as a different game from one written with \
         it folded; the shell saves nothing of the fold"
    );
    let reopened = clowder_core::load_string(&open_save, 0).expect("the save loads").state;
    assert!(
        !is_open(&reopened, &in_ideas(&reopened)),
        "a sitting opened from a save written with the group open opened it open"
    );
}

/// Within a sitting the group changes only on `d`: leaving the section and
/// returning — by a section cycle or by digits — `Esc`, a resize, and the
/// simulation advancing each leave it as it stands, open or folded.
///
/// @spec TUI-NAV-112
#[test]
fn within_a_sitting_the_done_group_stays_as_the_player_left_it() {
    let (mut a, mut t) = ideas_session(mid_game(), "stays");
    for want_open in [true, false] {
        gap();
        session_d(&mut a, &mut t);
        assert_eq!(is_open(&a.state, &a.view), want_open, "d did not set the done group open={want_open} (TUI-NAV-115)");

        let check = |a: &App, what: &str| {
            assert_eq!(
                is_open(&a.state, &a.view),
                want_open,
                "{what} changed the done group (it was open={want_open}); within a sitting only d, \
                 the palette's jump, or the group leaving the list changes it"
            );
        };
        step(&mut a, &mut t, &[ev(KeyCode::Tab)]);
        check(&a, "a section cycle away");
        step(&mut a, &mut t, &[ev_char('3')]);
        check(&a, "returning by the section's digit");
        step(&mut a, &mut t, &[ev_char('1'), ev_char('3')]);
        check(&a, "leaving for home and returning");
        step(&mut a, &mut t, &[ev(KeyCode::BackTab), ev(KeyCode::Tab)]);
        check(&a, "a cycle back and forth");
        step(&mut a, &mut t, &[ev(KeyCode::Esc)]);
        check(&a, "Esc");
        for (w, h) in [(80u16, 24u16), (99, 30), (200, 60), (60, 20), (130, 40)] {
            t.backend_mut().resize(w, h);
            step(&mut a, &mut t, &[Event::Resize(w, h)]);
            check(&a, &format!("a resize to {w}x{h}"));
        }
        let ticks = a.state.ticks;
        a.step(&mut t, Reading { monotonic: Duration::from_secs(3), wall: Duration::from_secs(3) }, &[])
            .expect("a step against a TestBackend does not fail");
        assert!(a.state.ticks > ticks, "fixture: the simulation advanced");
        check(&a, "the simulation advancing");
        assert_eq!(a.view.section, Some(Section::Ideas), "fixture: the sweep ends on the Ideas screen");
    }
}

/// A done group that stops being listed — nothing on the Ideas screen
/// finished, as after a Move — is folded when it is next listed, whether the
/// player stood on the Ideas screen through the gap or elsewhere.
///
/// @spec TUI-NAV-112
#[test]
fn a_done_group_that_stops_being_listed_comes_back_folded() {
    for stay in [true, false] {
        let (mut a, mut t) = ideas_session(mid_game(), "relisted");
        session_d(&mut a, &mut t);
        assert!(is_open(&a.state, &a.view), "d did not open the done group (TUI-NAV-115)");
        if !stay {
            step(&mut a, &mut t, &[ev_char('1')]);
        }
        let kept = a.state.clone();
        a.state = no_ideas_done();
        step(&mut a, &mut t, &[]);
        a.state = kept;
        step(&mut a, &mut t, &[]);
        if !stay {
            step(&mut a, &mut t, &[ev_char('3')]);
        }
        assert!(
            !is_open(&a.state, &a.view),
            "a done group that left the list came back open (player on the Ideas screen throughout: \
             {stay}); it comes back folded"
        );
    }
}

// ---- TUI-NAV-113 -----------------------------------------------------------

/// A meadow whose Ideas rows still to take are all out of reach while some of
/// its folded rows are ones the core reports Available: the landing rule's
/// first Available row in the whole list would be a folded one.
fn ready_rows_all_folded() -> GameState {
    use clowder_core::content::{idea_def, refinement_def};
    let mut s = all_ideas_done();
    let leaf = IdeaId::ALL
        .iter()
        .copied()
        .find(|i| {
            !IdeaId::ALL.iter().any(|j| idea_def(*j).prereqs.contains(i))
                && !clowder_core::RefinementId::ALL.iter().any(|r| refinement_def(*r).gate == *i)
        })
        .expect("fixture: some idea gates nothing");
    s.ideas.remove(&leaf);
    s.resources[ResourceId::Curiosity.ix()] = 0.0;
    let list = rows(&s, Section::Ideas);
    let unfinished: Vec<&Row> = list.iter().filter(|r| r.selectable() && !finished(&s, r)).collect();
    assert!(
        !unfinished.is_empty()
            && unfinished.iter().all(|r| match r.key {
                RowKey::Content(id, _) => s.availability(id) != Availability::Available,
                _ => false,
            }),
        "fixture: every unfinished Ideas row is out of reach"
    );
    assert!(
        list.iter().any(|r| finished(&s, r)
            && matches!(r.key, RowKey::Content(id, _) if s.availability(id) == Availability::Available)),
        "fixture: some finished Ideas row is one the core reports Available"
    );
    s
}

/// While the done group is folded the cursor visits none of its rows: from
/// every row drawn, ↓, ↑, j, k, `]` and `[` pressed again and again land only
/// on drawn rows, a sweep down visits exactly the drawn rows in order, and a
/// first visit's landing chooses among drawn rows only. While it is open the
/// cursor visits them: a sweep down visits every listed row in order, and `]`
/// from the last row above the done header jumps to the group's first row.
///
/// @spec TUI-NAV-113
#[test]
fn the_cursor_never_lands_on_a_folded_row_and_reaches_every_open_one() {
    let state = mid_game();
    let v = in_ideas(&state);
    let folded = drawn_selectable(&state, &v);
    let moves = [
        key(KeyCode::Down),
        key(KeyCode::Up),
        key(KeyCode::Char('j')),
        key(KeyCode::Char('k')),
        key(KeyCode::Char(']')),
        key(KeyCode::Char('[')),
    ];
    for start in &folded {
        for k in moves {
            let mut w = v.clone();
            w.focus[Section::Ideas.index()] = Some(start.clone());
            for n in 0..folded.len() + 2 {
                press(k, &mut w, &state);
                let at = ideas_cursor(&w);
                assert!(
                    at.as_ref().is_some_and(|c| folded.contains(c)),
                    "from {start:?}, {:?} pressed {} time(s) left the cursor on {at:?}, which is not a \
                     drawn row; while the group is folded the cursor visits none of its rows",
                    k.code,
                    n + 1
                );
            }
        }
    }

    let sweep = |mut w: ViewState, len: usize| -> Vec<RowKey> {
        let mut seen = vec![ideas_cursor(&w).expect("a cursor")];
        for _ in 0..len {
            press(key(KeyCode::Down), &mut w, &state);
            let at = ideas_cursor(&w).expect("a cursor");
            if seen.last() != Some(&at) {
                seen.push(at);
            }
        }
        seen
    };
    let mut w = v.clone();
    w.focus[Section::Ideas.index()] = folded.first().cloned();
    assert_eq!(sweep(w, folded.len() + 2), folded, "a sweep down the folded list does not visit exactly the drawn rows");

    let mut o = v.clone();
    press_d(&mut o, &state);
    let listed: Vec<RowKey> =
        rows(&state, Section::Ideas).into_iter().filter(|r| r.selectable()).map(|r| r.key).collect();
    o.focus[Section::Ideas.index()] = listed.first().cloned();
    assert_eq!(
        sweep(o.clone(), listed.len() + 2),
        listed,
        "a sweep down the open list does not visit every listed row, the done group's included"
    );
    let last_above = folded.last().cloned().expect("fixture: rows are drawn above the done header");
    let first_done = listed
        .iter()
        .find(|k| rows(&state, Section::Ideas).iter().any(|r| &r.key == *k && finished(&state, r)))
        .cloned()
        .expect("fixture: the done group holds rows");
    o.focus[Section::Ideas.index()] = Some(last_above);
    press(key(KeyCode::Char(']')), &mut o, &state);
    assert_eq!(ideas_cursor(&o), Some(first_done), "] from the last row above the open done group does not jump to its first row");

    let poor = ready_rows_all_folded();
    let entered = in_ideas(&poor);
    assert_eq!(
        ideas_cursor(&entered),
        drawn_selectable(&poor, &entered).first().cloned(),
        "a first visit landed on {:?}; with nothing drawn Available it lands on the first drawn row, \
         never on a folded one the core reports Available",
        ideas_cursor(&entered)
    );
}

/// While the done group is open its rows' keys answer as any row's do: Enter,
/// x and X on a finished row answer exactly as they do on any row whose item
/// is already the player's — a bought arch's on the Wonders screen.
///
/// @spec TUI-NAV-113
#[test]
fn an_open_done_rows_keys_answer_as_any_owned_rows_do() {
    let state = mid_game();
    let (arch, _) = find_row(&state, Section::Wonders, |c| matches!(c, Command::BuyArch(ArchId::EverdreamArch)));
    let mut o = in_ideas(&state);
    press_d(&mut o, &state);
    let done: Vec<RowKey> = rows(&state, Section::Ideas)
        .into_iter()
        .filter(|r| finished(&state, r))
        .map(|r| r.key)
        .collect();
    assert!(!done.is_empty(), "fixture: the done group holds rows");
    for k in [key(KeyCode::Enter), key(KeyCode::Char('x')), key(KeyCode::Char('X'))] {
        let mut owned_view = focused(Section::Wonders, arch.clone());
        let owned_out = press(k, &mut owned_view, &state);
        for row in &done {
            let mut w = o.clone();
            w.focus[Section::Ideas.index()] = Some(row.clone());
            let out = press(k, &mut w, &state);
            assert_eq!(out, owned_out, "{:?} on the open done row {row:?} answered {out:?}", k.code);
            assert_eq!(
                footer_text(&w),
                footer_text(&owned_view),
                "{:?} on the open done row {row:?} left a different note than on an owned row",
                k.code
            );
            assert!(w.armed.is_none() && w.prompt.is_none(), "{:?} on {row:?} armed or prompted", k.code);
        }
    }
}

// ---- TUI-NAV-115 -----------------------------------------------------------

/// `d` opens a folded done group and folds an open one, each distinct press
/// once, and a press is no command and no navigation: it asks the core for
/// nothing, leaves an armed row armed, leaves the report banner unread, counts
/// as no visit, leaves the cursor and the section where they were, and leaves
/// the layout — the narrow-mode swap called for — as it stood.
///
/// @spec TUI-NAV-115
#[test]
fn d_opens_and_folds_the_done_group_and_is_no_command_and_no_navigation() {
    let state = mid_game();
    let mut v = in_ideas(&state);
    let rule = drawn(&state, &v)
        .into_iter()
        .find(|r| r.heavy && actionable(&state, r))
        .expect("fixture: a rule is drawn to arm");
    v.focus[Section::Ideas.index()] = Some(rule.key.clone());
    press(key(KeyCode::Enter), &mut v, &state);
    assert!(v.armed.is_some(), "fixture: Enter armed the rule");
    v.reports = vec![PendingReport { entry: 0, dismissed: false }];
    v.inspector_wanted = true;
    v.last_repeat = None;

    for want_open in [true, false, true] {
        let before = v.clone();
        let out = press_char('d', &mut v, &state);
        assert_eq!(out, Outcome::Handled, "d asked the core for {out:?}; it issues no command");
        assert_eq!(is_open(&state, &v), want_open, "d did not turn the done group open={want_open}");
        assert_eq!(v.armed, before.armed, "d ended or changed the arming");
        assert_eq!(v.reports, before.reports, "d retired the report banner");
        assert_eq!(v.seen, before.seen, "d counted as a visit");
        assert_eq!(v.section, before.section, "d moved the focused section");
        assert_eq!(v.focus, before.focus, "d moved a cursor");
        assert_eq!(v.inspector_wanted, before.inspector_wanted, "d changed the layout");
        assert!(v.footer.is_none(), "d left a footer note");
        gap();
    }
}

/// A held `d` changes the group once: a run of presses each too soon after the
/// last to be a distinct one changes it on the first alone, from folded and
/// from open, and the next distinct press changes it again.
///
/// @spec TUI-NAV-115
#[test]
fn a_held_d_changes_the_done_group_once() {
    let state = mid_game();
    let mut v = in_ideas(&state);
    for want_open in [true, false] {
        for _ in 0..12 {
            press_char('d', &mut v, &state);
            std::thread::sleep(Duration::from_millis(30));
        }
        assert_eq!(
            is_open(&state, &v),
            want_open,
            "a held d left the group open={}; a key held down changes it once",
            !want_open
        );
        gap();
    }
}

/// Wherever `d` is not live it is an unbound key, ignored with nothing changed
/// and no footer note: on every other screen with every drawn row and none
/// under the cursor, on an Ideas screen with nothing finished, and as
/// uppercase `D`. An open field takes it as the field takes a letter — the
/// palette into its query, the count prompt not at all — and a full-screen
/// overlay suspends it; in each the group is as it was once they close.
///
/// @spec TUI-NAV-115
#[test]
fn d_is_ignored_wherever_it_is_not_live_and_held_by_whatever_holds_the_keyboard() {
    let state = mid_game();
    for section in Section::ALL.into_iter().filter(|s| *s != Section::Ideas) {
        let listed = rows(&state, section);
        if listed.is_empty() {
            continue;
        }
        let mut contexts: Vec<Option<RowKey>> = vec![None];
        contexts.extend(listed.iter().filter(|r| r.selectable()).map(|r| Some(r.key.clone())));
        for focus in contexts {
            let mut before = ViewState { section: Some(section), ..Default::default() };
            before.focus[section.index()] = focus.clone();
            let mut w = before.clone();
            let out = press_char('d', &mut w, &state);
            assert_eq!(out, Outcome::Handled, "{}: d told the core {out:?}", describe(section, &focus));
            assert_eq!(observable(&w), observable(&before), "{}: d changed the view", describe(section, &focus));
            assert!(w.footer.is_none(), "{}: d left a footer note", describe(section, &focus));
        }
    }

    let none_done = no_ideas_done();
    let before = in_ideas(&none_done);
    let mut w = before.clone();
    assert_eq!(press_char('d', &mut w, &none_done), Outcome::Handled);
    assert_eq!(observable(&w), observable(&before), "d changed the view on an Ideas screen with nothing finished");
    assert!(w.footer.is_none(), "d left a note on an Ideas screen with nothing finished");

    let before = in_ideas(&state);
    let mut w = before.clone();
    press(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT), &mut w, &state);
    assert_eq!(observable(&w), observable(&before), "uppercase D changed the view; it is unbound");
    assert!(w.footer.is_none(), "uppercase D left a note");

    // A field takes it.
    let mut w = in_ideas(&state);
    press_char('/', &mut w, &state);
    press_char('d', &mut w, &state);
    assert_eq!(palette_query(&w).as_deref(), Some("d"), "the palette did not take d into its query");
    clowder_tui::nav::dismiss(&mut w, false);
    assert!(!is_open(&state, &w), "d typed into the palette changed the done group");

    let mut w = in_ideas(&state);
    let refinement = drawn(&state, &w)
        .into_iter()
        .find(|r| opens_prompt(&state, r))
        .expect("fixture: a row on the Ideas screen opens the count prompt");
    w.focus[Section::Ideas.index()] = Some(refinement.key.clone());
    press_char('x', &mut w, &state);
    assert!(w.prompt.is_some(), "fixture: x opened the count prompt");
    press_char('d', &mut w, &state);
    assert_eq!(prompt_digits(&w).as_deref(), Some(""), "the count prompt took d");
    clowder_tui::nav::dismiss(&mut w, false);
    assert!(!is_open(&state, &w), "d pressed into the count prompt changed the done group");

    // A full-screen overlay suspends it.
    for opener in ['?', 'e'] {
        let mut w = in_ideas(&state);
        press_char(opener, &mut w, &state);
        let overlay = w.overlay.clone();
        assert!(overlay.is_some(), "fixture: {opener} opened a full-screen overlay");
        press_char('d', &mut w, &state);
        assert_eq!(w.overlay, overlay, "d under the overlay changed it");
        clowder_tui::nav::dismiss(&mut w, false);
        assert!(!is_open(&state, &w), "d under a full-screen overlay changed the done group");
    }
}

// ---- TUI-NAV-116 -----------------------------------------------------------

/// Opening the group leaves the cursor on the row it rests on, whichever drawn
/// row that is; where it rests on no row, every row being folded, it lands as
/// a first visit does among the rows the open group draws.
///
/// @spec TUI-NAV-116
#[test]
fn opening_the_done_group_leaves_the_cursor_on_its_row() {
    let state = mid_game();
    let v = in_ideas(&state);
    for start in drawn_selectable(&state, &v) {
        let mut w = v.clone();
        w.focus[Section::Ideas.index()] = Some(start.clone());
        press_d(&mut w, &state);
        assert!(is_open(&state, &w), "d did not open the done group (TUI-NAV-115)");
        assert_eq!(ideas_cursor(&w), Some(start.clone()), "opening the done group moved the cursor off {start:?}");
    }

    let all = all_ideas_done();
    let mut w = in_ideas(&all);
    assert!(drawn_selectable(&all, &w).is_empty(), "with every row finished and the group folded, a row is still drawn (TUI-NAV-113)");
    press_d(&mut w, &all);
    let open = drawn_selectable(&all, &w);
    assert_eq!(
        ideas_cursor(&w),
        landing_among(&all, &open),
        "opening the group with the cursor on no row did not land it as a first visit does"
    );
}

/// Folding the group with the cursor on one of its rows moves the cursor to
/// the last row drawn above the done header; where no row is drawn above it,
/// the cursor rests on no row, and a row key says there is nothing to act on.
///
/// @spec TUI-NAV-116
#[test]
fn folding_the_done_group_from_one_of_its_rows_moves_the_cursor_above_its_header() {
    for (label, state) in [("mid-game", mid_game()), ("every row done", all_ideas_done())] {
        let mut template = in_ideas(&state);
        let above = drawn_selectable(&state, &template).last().cloned();
        press_d(&mut template, &state);
        let done: Vec<RowKey> = rows(&state, Section::Ideas)
            .into_iter()
            .filter(|r| finished(&state, r))
            .map(|r| r.key)
            .collect();
        gap();
        for row in &done {
            let mut w = template.clone();
            w.focus[Section::Ideas.index()] = Some(row.clone());
            press_char('d', &mut w, &state);
            assert!(!is_open(&state, &w), "d did not fold the done group (TUI-NAV-115)");
            match &above {
                Some(k) => assert_eq!(
                    ideas_cursor(&w).as_ref(),
                    Some(k),
                    "in {label}, folding from {row:?} left the cursor on {:?}, not the last row drawn \
                     above the done header",
                    ideas_cursor(&w)
                ),
                None => {
                    w.last_repeat = None;
                    let out = press(key(KeyCode::Enter), &mut w, &state);
                    assert_eq!(out, Outcome::Handled, "in {label}, Enter after folding from {row:?} acted");
                    assert_eq!(
                        footer_text(&w),
                        Some(nothing_to_act_on_note()),
                        "in {label}, with no row drawn above the header, folding from {row:?} left \
                         the cursor on a row"
                    );
                }
            }
        }
    }
}

// ---- TUI-NAV-117 -----------------------------------------------------------

/// With the group folded, a row that finishes under the cursor hands the
/// cursor to the row that closes up into its place — the first drawn row after
/// that place, else the last drawn row before it, else none — for an idea, a
/// refinement and a rule finished by the player's own Enter. The move is not
/// navigation: the report banner stays unread and no visit is counted.
///
/// @spec TUI-NAV-117
#[test]
fn a_row_finishing_under_the_cursor_hands_it_to_the_row_that_closes_up() {
    let base = mid_game();
    let start = in_ideas(&base);
    let ready: Vec<Row> = drawn(&base, &start).into_iter().filter(|r| actionable(&base, r)).collect();
    let (mut after_seen, mut before_seen) = (0, 0);
    for row in &ready {
        let (mut a, mut t) = ideas_session(base.clone(), "closes-up");
        a.view.focus[Section::Ideas.index()] = Some(row.key.clone());
        a.view.reports = vec![PendingReport { entry: 0, dismissed: false }];
        let seen = a.view.seen;
        step(&mut a, &mut t, &[ev(KeyCode::Enter)]);
        if row.heavy {
            gap();
            step(&mut a, &mut t, &[ev(KeyCode::Enter)]);
        }
        assert!(finished(&a.state, row), "fixture: the core took {:?}", row.key);
        let expected = closes_up(&drawn_selectable(&a.state, &a.view), &row.key);
        assert_eq!(
            ideas_cursor(&a.view),
            expected,
            "{:?} finished under the cursor with the group folded; the cursor should take the row \
             that closes up into its place",
            row.key
        );
        match &expected {
            Some(k) if ideas_place(k) > ideas_place(&row.key) => after_seen += 1,
            Some(_) => before_seen += 1,
            None => {}
        }
        assert_eq!(a.view.reports, vec![PendingReport { entry: 0, dismissed: false }], "the move retired the banner");
        assert_eq!(a.view.seen, seen, "the move counted as a visit");
    }
    assert!(after_seen > 0, "fixture: no finished row was followed by a drawn row");
    assert!(before_seen > 0, "fixture: no finished row was the last drawn row");

    // Neither: the one row still to take finishes, and nothing is left drawn.
    let mut last = all_ideas_done();
    let only = clowder_core::RefinementId::ALL
        .iter()
        .copied()
        .find(|r| {
            let mut s = last.clone();
            s.refinements.remove(r);
            s.lock_reason(ContentId::Refinement(*r)).is_none()
        })
        .expect("fixture: some refinement can be bought again once un-bought");
    last.refinements.remove(&only);
    let (mut a, mut t) = ideas_session(last, "neither");
    let key_ = RowKey::Content(ContentId::Refinement(only), "");
    assert_eq!(ideas_cursor(&a.view), Some(key_.clone()), "fixture: the cursor is on the one row left to take");
    step(&mut a, &mut t, &[ev(KeyCode::Enter)]);
    assert!(a.state.refinements.contains(&only), "fixture: the core took it");
    let mut w = a.view.clone();
    w.last_repeat = None;
    w.field_guard = false;
    assert_eq!(update(Action::Activate, &mut w, &a.state), Outcome::Handled);
    assert_eq!(
        footer_text(&w),
        Some(nothing_to_act_on_note()),
        "with no drawn row left after it finished, the cursor should rest on no row"
    );
}

/// With the group folded, returning to the Ideas screen when the row the
/// cursor remembered there has finished meanwhile places the cursor where that
/// row closes up, by the same rule — for every row the cursor could have
/// remembered.
///
/// @spec TUI-NAV-117
#[test]
fn returning_to_a_remembered_row_that_finished_meanwhile_places_the_cursor_where_it_closes_up() {
    let base = mid_game();
    let start = in_ideas(&base);
    let mut checked = 0;
    for row in drawn(&base, &start).into_iter().filter(|r| r.selectable()) {
        let mut v = start.clone();
        v.focus[Section::Ideas.index()] = Some(row.key.clone());
        press_char('1', &mut v, &base);
        let mut after = base.clone();
        finish(&mut after, &row);
        if !finished(&after, &row) {
            continue;
        }
        press_char('3', &mut v, &after);
        let expected = closes_up(&drawn_selectable(&after, &v), &row.key);
        assert_eq!(
            ideas_cursor(&v),
            expected,
            "the remembered {:?} finished while the Ideas screen was out of focus; returning should \
             place the cursor where it closes up",
            row.key
        );
        checked += 1;
    }
    assert!(checked > 10, "fixture: too few remembered rows were checked");
}

// ---- TUI-NAV-118 -----------------------------------------------------------

/// With the group open, a row that finishes under the cursor keeps the cursor:
/// it stays on the row, now drawn in the done group.
///
/// @spec TUI-NAV-118
#[test]
fn with_the_group_open_a_row_finishing_under_the_cursor_keeps_it() {
    let base = mid_game();
    let start = in_ideas(&base);
    let ready: Vec<Row> =
        drawn(&base, &start).into_iter().filter(|r| actionable(&base, r) && !r.heavy).collect();
    assert!(ready.len() > 4, "fixture: several rows are ready to take");
    for row in &ready {
        let (mut a, mut t) = ideas_session(base.clone(), "open-keeps");
        session_d(&mut a, &mut t);
        assert!(is_open(&a.state, &a.view), "d did not open the done group (TUI-NAV-115)");
        a.view.focus[Section::Ideas.index()] = Some(row.key.clone());
        step(&mut a, &mut t, &[ev(KeyCode::Enter)]);
        assert!(finished(&a.state, row), "fixture: the core took {:?}", row.key);
        assert_eq!(ideas_cursor(&a.view), Some(row.key.clone()), "the cursor left {:?} as it finished", row.key);
        let list = drawn(&a.state, &a.view);
        let header = list.iter().rposition(|r| matches!(r.kind, RowKind::Group)).expect("a header");
        assert!(
            list[header + 1..].iter().any(|r| r.key == row.key),
            "{:?} finished with the group open but is not drawn in the done group",
            row.key
        );
    }
}

// ---- TUI-NAV-119 -----------------------------------------------------------

/// Enter held for `held` steps, `pace` apart.
fn hold_enter_steps(a: &mut App, t: &mut Terminal<TestBackend>, held: usize, pace: Duration) {
    for _ in 0..held {
        step(a, t, &[ev(KeyCode::Enter)]);
        std::thread::sleep(pace);
    }
}

/// When a row folds away from under the cursor, a held or auto-repeating
/// continuation of the press acts on nothing the cursor lands on — no idea
/// researched, no refinement bought, no rule armed — and the next distinct
/// press does act there.
///
/// @spec TUI-NAV-119
#[test]
fn a_held_enter_never_acts_on_the_row_a_folded_row_hands_the_cursor_to() {
    let base = mid_game();
    let start = in_ideas(&base);
    let drawn_rows_now = drawn(&base, &start);
    for rank in 0..3 {
        let first = drawn_rows_now
            .iter()
            .find(|r| r.selectable() && ideas_rank(r) == rank && actionable(&base, r))
            .unwrap_or_else(|| panic!("fixture: kind {rank} has a row ready to take"))
            .clone();
        let (mut a, mut t) = ideas_session(base.clone(), "held");
        a.view.focus[Section::Ideas.index()] = Some(first.key.clone());
        step(&mut a, &mut t, &[ev(KeyCode::Enter)]);
        if first.heavy {
            gap();
            step(&mut a, &mut t, &[ev(KeyCode::Enter)]);
        }
        assert!(finished(&a.state, &first), "fixture: the core took {:?}", first.key);
        let next = ideas_cursor(&a.view).expect("the cursor rests on no row after the row folded away (TUI-NAV-117)");
        let next_row = rows(&a.state, Section::Ideas).into_iter().find(|r| r.key == next).expect("listed");
        assert!(
            actionable(&a.state, &next_row) && ideas_rank(&next_row) == rank,
            "when {:?} folded away the cursor did not move to the next row of its kind ready to take \
             (TUI-NAV-117)",
            first.key
        );

        hold_enter_steps(&mut a, &mut t, 15, Duration::from_millis(30));
        assert!(
            !finished(&a.state, &next_row),
            "a held Enter took {next:?}, the row the cursor landed on when {:?} folded away",
            first.key
        );
        assert!(a.view.armed.is_none(), "a held Enter armed {next:?} after {:?} folded away", first.key);

        gap();
        step(&mut a, &mut t, &[ev(KeyCode::Enter)]);
        let acted = finished(&a.state, &next_row) || a.view.armed.as_ref().is_some_and(|x| x.key == next);
        assert!(acted, "a distinct Enter after the hold did not act on {next:?}");
    }
}

/// The same fence holds against a held X on a row that finishes under it.
///
/// @spec TUI-NAV-119
#[test]
fn a_held_x_never_acts_on_the_row_a_folded_row_hands_the_cursor_to() {
    let base = mid_game();
    let start = in_ideas(&base);
    let drawn_rows_now = drawn(&base, &start);
    for rank in 0..3 {
        let Some(first) = drawn_rows_now
            .iter()
            .find(|r| r.selectable() && ideas_rank(r) == rank && actionable(&base, r) && !r.heavy)
            .cloned()
        else {
            continue;
        };
        let (mut a, mut t) = ideas_session(base.clone(), "heldx");
        a.view.focus[Section::Ideas.index()] = Some(first.key.clone());
        step(&mut a, &mut t, &[ev(KeyCode::Char('X'))]);
        if !finished(&a.state, &first) {
            continue;
        }
        let Some(next) = ideas_cursor(&a.view) else { continue };
        let next_row = rows(&a.state, Section::Ideas).into_iter().find(|r| r.key == next).expect("listed");
        for _ in 0..15 {
            step(&mut a, &mut t, &[ev(KeyCode::Char('X'))]);
            std::thread::sleep(Duration::from_millis(30));
        }
        assert!(
            !finished(&a.state, &next_row),
            "a held X took {next:?}, the row the cursor landed on when {:?} folded away",
            first.key
        );
    }
}

// ---- TUI-NAV-120 -----------------------------------------------------------

/// While every row the Ideas section lists is folded, the list draws the done
/// header alone, the section keeps its tab, and every row key — Enter, x, X,
/// the count keys and Backspace — finds no row: it asks the core for nothing,
/// changes nothing, and leaves the one note a section with nothing listed
/// leaves (TUI-NAV-085).
///
/// @spec TUI-NAV-120
#[test]
fn with_every_row_folded_the_header_stands_alone_and_every_row_key_finds_nothing() {
    let state = all_ideas_done();
    let v = in_ideas(&state);
    let list = drawn(&state, &v);
    assert!(
        list.len() == 1 && matches!(list[0].kind, RowKind::Group),
        "with every row folded the list draws {} line(s), not the done header alone",
        list.len()
    );
    for focus in [Some(Section::Ideas), Some(Section::Meadow)] {
        assert!(
            clowder_tui::nav::rows::tab_sections(&state, focus).contains(&Section::Ideas),
            "the Ideas section lost its tab with focus on {focus:?}"
        );
    }
    let note = nothing_to_act_on_note();
    for k in [
        key(KeyCode::Enter),
        key(KeyCode::Char('x')),
        key(KeyCode::Char('X')),
        key(KeyCode::Char('+')),
        key(KeyCode::Char('=')),
        key(KeyCode::Char('-')),
        key(KeyCode::Backspace),
    ] {
        let mut w = v.clone();
        let out = press(k, &mut w, &state);
        assert_eq!(out, Outcome::Handled, "{:?} with every row folded told the core {out:?}", k.code);
        assert_eq!(footer_text(&w), Some(note.clone()), "{:?} with every row folded left another note", k.code);
        let (mut a, mut b) = (w.clone(), v.clone());
        a.last_repeat = None;
        b.last_repeat = None;
        assert_eq!(all_but_the_footer(&a), all_but_the_footer(&b), "{:?} with every row folded changed the view", k.code);
    }
}

/// When a row the cursor can visit is next drawn beside an all-folded list,
/// the cursor lands on the first drawn row the core reports Available, else
/// the first drawn row — as on entering a section with no row remembered.
///
/// @spec TUI-NAV-120
#[test]
fn when_a_row_is_next_drawn_the_cursor_lands_on_the_first_available_one() {
    use clowder_core::content::{idea_def, refinement_def};
    let (mut a, mut t) = ideas_session(all_ideas_done(), "next-drawn");
    assert!(drawn_selectable(&a.state, &a.view).is_empty(), "with every row finished and the group folded, a row is still drawn (TUI-NAV-113)");

    let mut s = all_ideas_done();
    let leaf = IdeaId::ALL
        .iter()
        .copied()
        .find(|i| {
            !IdeaId::ALL.iter().any(|j| idea_def(*j).prereqs.contains(i))
                && !clowder_core::RefinementId::ALL.iter().any(|r| refinement_def(*r).gate == *i)
        })
        .expect("fixture: some idea gates nothing");
    s.ideas.remove(&leaf);
    s.resources[ResourceId::Curiosity.ix()] = 0.0;
    let back = *s.refinements.iter().next().expect("fixture: a refinement is held");
    s.refinements.remove(&back);
    a.state = s;
    step(&mut a, &mut t, &[]);
    let now = drawn_selectable(&a.state, &a.view);
    assert!(now.len() >= 2, "fixture: two rows are drawn again");
    assert!(
        matches!(&now[0], RowKey::Content(id, _) if a.state.availability(*id) != Availability::Available),
        "fixture: the first drawn row is not Available, so the landing rule is under test"
    );
    assert_eq!(
        ideas_cursor(&a.view),
        landing_among(&a.state, &now),
        "a row returned beside an all-folded list and the cursor did not land on the first Available one"
    );
}

// ---- TUI-NAV-121 -----------------------------------------------------------

/// The palette's jump to an item whose row is folded opens the done group and
/// focuses that row; the jump is otherwise the palette's jump — it closes the
/// palette, lands in the Ideas section as a visit, and retires the banner —
/// and the group stays open after it. Quantified over every finished item the
/// palette's query can aim at alone.
///
/// @spec TUI-NAV-121
#[test]
fn the_palettes_jump_to_a_folded_row_opens_the_done_group_and_focuses_it() {
    let state = mid_game();
    let finished_rows: Vec<RowKey> = rows(&state, Section::Ideas)
        .into_iter()
        .filter(|r| finished(&state, r))
        .map(|r| r.key)
        .collect();
    let all = clowder_tui::nav::palette::results(&state, "");
    let mut jumped = 0;
    for target in &finished_rows {
        let name = all.iter().find(|r| &r.key == target).expect("a finished row is a palette result").name.clone();
        if clowder_tui::nav::palette::results(&state, &name).first().map(|r| &r.key) != Some(target) {
            continue;
        }
        let mut v = ViewState::default();
        clowder_tui::nav::opening(&mut v, &state);
        v.reports = vec![PendingReport { entry: 0, dismissed: false }];
        v.overlay = Some(Overlay::Palette { query: name.clone(), selected: 0 });
        let out = press(key(KeyCode::Enter), &mut v, &state);
        assert_eq!(out, Outcome::Handled);
        assert!(v.overlay.is_none(), "the jump to {target:?} left the palette open");
        assert_eq!(v.section, Some(Section::Ideas), "the jump to {target:?} did not land on the Ideas screen");
        assert_eq!(ideas_cursor(&v).as_ref(), Some(target), "the jump did not focus {target:?}");
        assert!(is_open(&state, &v), "the jump to the folded {target:?} did not open the done group");
        assert!(drawn_selectable(&state, &v).contains(target), "the jump focused {target:?}, which is not drawn");
        assert!(v.reports[0].dismissed, "the jump did not retire the banner");
        assert!(v.seen[Section::Ideas.index()].visited, "the jump did not count as a visit");

        v.field_guard = false;
        press_char('1', &mut v, &state);
        press_char('3', &mut v, &state);
        assert!(is_open(&state, &v), "the group the jump opened did not stay open");
        jumped += 1;
    }
    assert!(jumped * 2 > finished_rows.len(), "fixture: the palette could aim at too few finished items alone");
}

// ---- TUI-NAV-123 -----------------------------------------------------------

/// `d` is no screen's mnemonic and no row-bound key: no screen spends it on a
/// mnemonic, no state and cursor make it a live one, and on every screen,
/// with every drawn row and none under the cursor, it asks the core for
/// nothing — and off the Ideas screen changes nothing at all.
///
/// @spec TUI-NAV-123
#[test]
fn d_is_no_screens_mnemonic_nor_row_bound_key_and_asks_the_core_for_nothing() {
    use clowder_tui::nav::mnemonics;
    for section in Section::ALL {
        assert!(
            !mnemonics::section_letters(section).contains(&'d'),
            "the {} screen spends d on a mnemonic",
            section.title()
        );
    }
    let mut pressed = 0;
    for (label, state) in [("mid-game", mid_game()), ("far along", rich()), ("new", fresh()), ("all done", all_ideas_done())] {
        for section in Section::ALL {
            let mut base = ViewState { section: Some(section), ..Default::default() };
            let mut views = vec![base.clone()];
            for k in drawn_rows(&state, section, &base).into_iter().filter(|r| r.selectable()).map(|r| r.key) {
                base.focus[section.index()] = Some(k);
                views.push(base.clone());
            }
            if section == Section::Ideas {
                let mut open = ViewState { section: Some(section), ..Default::default() };
                press_d(&mut open, &state);
                for k in drawn_rows(&state, section, &open).into_iter().filter(|r| r.selectable()).map(|r| r.key) {
                    let mut w = open.clone();
                    w.focus[section.index()] = Some(k);
                    views.push(w);
                }
            }
            for before in views {
                let focus = before.focus[section.index()].clone();
                assert!(
                    !mnemonics::live(&state, section, focus.as_ref()).iter().any(|(c, _, _)| *c == 'd'),
                    "in {label}, {} d is a live mnemonic",
                    describe(section, &focus)
                );
                let mut w = before.clone();
                let out = press_char('d', &mut w, &state);
                assert!(
                    !matches!(out, Outcome::Apply(_) | Outcome::Repeat(..)),
                    "in {label}, {} d told the core {out:?}",
                    describe(section, &focus)
                );
                if section != Section::Ideas {
                    assert_eq!(observable(&w), observable(&before), "in {label}, {} d changed the view", describe(section, &focus));
                }
                pressed += 1;
            }
        }
    }
    assert!(pressed > 100, "fixture: too few contexts swept");
}
