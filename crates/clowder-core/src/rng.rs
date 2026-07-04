use serde::{Deserialize, Serialize};

/// PCG-XSH-RR 32-bit generator with 64-bit state, hand-rolled so the RNG
/// serializes stably with the game state across crate versions.
// @spec SIM-ENG-008
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

const PCG_MULT: u64 = 6364136223846793005;
const PCG_STREAM: u64 = 1442695040888963407;

impl Pcg32 {
    pub fn new(seed: u64) -> Self {
        let inc = (PCG_STREAM << 1) | 1;
        let mut rng = Pcg32 { state: 0, inc };
        rng.state = rng.state.wrapping_mul(PCG_MULT).wrapping_add(rng.inc);
        rng.state = rng.state.wrapping_add(seed);
        rng.state = rng.state.wrapping_mul(PCG_MULT).wrapping_add(rng.inc);
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(PCG_MULT).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// True with probability `p` (0.0..=1.0).
    pub fn chance(&mut self, p: f64) -> bool {
        (self.next_u32() as f64) < p * (u32::MAX as f64 + 1.0)
    }

    /// Uniform index in 0..n. Modulo bias is negligible at game scales.
    pub fn index(&mut self, n: usize) -> usize {
        (self.next_u32() as usize) % n.max(1)
    }
}
