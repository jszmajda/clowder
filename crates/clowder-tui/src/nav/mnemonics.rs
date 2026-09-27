//! The screen-mnemonic table (TUI-NAV-046): a lowercase letter that stands for
//! one row listed on its screen — a row the cursor and Enter already reach, so
//! the letter gates nothing. A *fixed* mnemonic stands for one fixed row of
//! its screen and is live while that row is listed; a *focused-item*
//! mnemonic stands for a row of the item the cursor is on and is live only
//! while the cursor sits on one of that item's own rows.
//!
//! Two callers read this table: `update` decides whether a pressed letter
//! performs its row's action in place of the cursor, and the hint bar decides
//! which live letters to name and where.

use clowder_core::GameState;

use super::rows::{rows, RowKey};
use super::section::Section;

/// Whether a mnemonic stands for one fixed row of its screen, or for a row of
/// whichever item the cursor sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MnemonicKind {
    Fixed,
    FocusedItem,
}

/// The Meadow's fixed mnemonics, in their rows' list order — the design's
/// table.
const FIXED: [(char, &str); 3] = [('g', "GatherCatnip"), ('f', "Forage"), ('r', "Rummage")];

/// Which letters the design spends on `section` at all — a fact of the
/// design's table, fixed at compile time, not of whether a row is listed
/// right now. Key resolution reads this (not game state) to keep a letter
/// the design never touches on a screen an unbound key there, the same as on
/// a screen with no mnemonics at all. @spec TUI-NAV-046
pub fn section_letters(section: Section) -> &'static [char] {
    match section {
        Section::Meadow => &['g', 'f', 'r'],
        // `n` is bound on a kitten's row too, but as the row-bound key
        // TUI-NAV-094 states, not as a mnemonic under this table's rule
        // (TUI-NAV-046) — `resolve` binds it on its own, never through this
        // list.
        Section::Clowder => &['p'],
        Section::Friends => &['p'],
        _ => &[],
    }
}

/// The mnemonics live on `section` with `focus` under the cursor, each with
/// the row it stands for and its kind — fixed ones in their rows' list order.
/// @spec TUI-NAV-046
pub fn live(
    state: &GameState,
    section: Section,
    focus: Option<&RowKey>,
) -> Vec<(char, RowKey, MnemonicKind)> {
    let listed = rows(state, section);
    let is_listed = |k: &RowKey| listed.iter().any(|r| &r.key == k);
    let mut out = Vec::new();
    match section {
        Section::Meadow => {
            for (c, tag) in FIXED {
                let key = RowKey::Fixed(tag);
                if is_listed(&key) {
                    out.push((c, key, MnemonicKind::Fixed));
                }
            }
        }
        Section::Clowder => {
            // A kitten is one row (TUI-NAV-093); `p` stands for that very
            // row, live only while the cursor is on it.
            if let Some(key @ RowKey::Kitten(..)) = focus {
                if is_listed(key) {
                    out.push(('p', key.clone(), MnemonicKind::FocusedItem));
                }
            }
        }
        Section::Friends => {
            if let Some(RowKey::Colony(colony, _)) = focus {
                let key = RowKey::Colony(*colony, "penpal");
                if is_listed(&key) {
                    out.push(('p', key, MnemonicKind::FocusedItem));
                }
            }
        }
        _ => {}
    }
    out
}

/// The row the live mnemonic `c` stands for on `section` with `focus` under
/// the cursor, or `None` if `c` names no live mnemonic there — an unbound
/// letter, silently ignored. @spec TUI-NAV-046
pub fn target(state: &GameState, section: Section, focus: Option<&RowKey>, c: char) -> Option<RowKey> {
    live(state, section, focus).into_iter().find(|(m, _, _)| *m == c).map(|(_, key, _)| key)
}
