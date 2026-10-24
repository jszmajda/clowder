//! Kitten profiles: what each kitten looks like and loves.
//!
//! A profile is drawn once, at arrival, as a fixed function of the kitten's
//! name and generation, and stored on the kitten. It takes no draw from the
//! state RNG, so the meadow's fixed draw schedule never moves for it, and a
//! save written before profiles can draw its kittens' profiles on load.

use crate::content;
use crate::rng::Pcg32;
use crate::state::{EyeLook, Favourite, FavouriteKind, Fur, FurColour, FurPattern, Pose, Profile};

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
/// generation, in the fixed order pattern, slot colours, eyes, how many
/// favourites, their kinds, their entries, pose. The fur and eyes come first,
/// so how favourites and poses are drawn never moves them.
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

    // Four to six kinds, none twice, then held in kind order.
    let count = 4 + rng.index(3);
    let mut left: Vec<FavouriteKind> = FavouriteKind::ALL.to_vec();
    let mut kinds: Vec<FavouriteKind> = (0..count)
        .map(|_| left.remove(rng.index(left.len())))
        .collect();
    kinds.sort();
    let favourites = kinds
        .into_iter()
        .map(|kind| {
            let entries = content::favourite_kind_def(kind).entries;
            Favourite {
                kind,
                entry: entries[rng.index(entries.len())].to_string(),
            }
        })
        .collect();

    let pose = Pose::ALL[rng.index(Pose::ALL.len())];
    Profile {
        fur: Fur { pattern, colours },
        eyes,
        favourites,
        pose,
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
