//! Kitten profiles: what each kitten looks like and loves.
//!
//! A profile is drawn once, at arrival, as a fixed function of the kitten's
//! name and generation, and stored on the kitten. It takes no draw from the
//! state RNG, so the meadow's fixed draw schedule never moves for it, and a
//! save written before profiles can draw its kittens' profiles on load.

use crate::content;
use crate::rng::Pcg32;
use crate::state::{EyeLook, Fur, FurColour, FurPattern, Profile};

/// 64-bit FNV-1a. Hand-rolled because the standard hasher's output may change
/// between Rust releases, and a profile drawn on load must match the one an
/// arrival would have drawn on any build.
fn fnv1a(bytes: impl IntoIterator<Item = u8>) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes
        .into_iter()
        .fold(OFFSET, |h, b| (h ^ b as u64).wrapping_mul(PRIME))
}

/// The profile a kitten named `name`, arriving in `generation`, draws: from a
/// generator local to the draw, seeded by hashing the name and the
/// generation, in the fixed order pattern, slot colours, eyes, favourites.
/// @spec SIM-KIT-032
pub fn draw_profile(name: &str, generation: u32) -> Profile {
    let seed = fnv1a(name.bytes().chain(generation.to_le_bytes()));
    let mut rng = Pcg32::new(seed);

    let pattern = FurPattern::ALL[rng.index(FurPattern::ALL.len())];
    let mut colours: Vec<FurColour> = Vec::new();
    for slot in content::fur_pattern_def(pattern).slots {
        let left: Vec<FurColour> = slot
            .iter()
            .copied()
            .filter(|c| !colours.contains(c))
            .collect();
        colours.push(left[rng.index(left.len())]);
    }
    let eyes = EyeLook::ALL[rng.index(EyeLook::ALL.len())];
    let mut pick = |table: &[&str]| table[rng.index(table.len())].to_string();
    Profile {
        fur: Fur { pattern, colours },
        eyes,
        food: pick(&content::FAVOURITE_FOODS),
        toy: pick(&content::FAVOURITE_TOYS),
        nap: pick(&content::FAVOURITE_NAPS),
        show: pick(&content::FAVOURITE_SHOWS),
        pastime: pick(&content::FAVOURITE_PASTIMES),
        quirk: pick(&content::FAVOURITE_QUIRKS),
    }
}

/// A fur in words: its pattern's words, each slot mark replaced by the name
/// of the colour that slot took. @spec SIM-KIT-039
pub fn fur_words(fur: &Fur) -> String {
    let mut words = content::fur_pattern_def(fur.pattern).words.to_string();
    for (k, c) in fur.colours.iter().enumerate() {
        words = words.replace(&format!("{{{k}}}"), content::fur_colour_name(*c));
    }
    words
}
