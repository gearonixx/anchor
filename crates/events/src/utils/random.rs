// deterministic seeds

// [ai] AI-generated, not reviewed yet

use chrono::Datelike;

use crate::types::LocalDate;
use crate::events::Event;

const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
const MIX_MULTIPLIER_A: u64 = 0xBF58_476D_1CE4_E5B9;
const MIX_MULTIPLIER_B: u64 = 0x94D0_49BB_1331_11EB;
const F64_MANTISSA_BITS: u32 = 53;

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn from_parts(parts: &[u64]) -> Self {
        let state = parts
            .iter()
            .fold(0, |acc, &part| mix(acc ^ mix(part.wrapping_add(GOLDEN_GAMMA))));
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN_GAMMA);
        mix(self.state)
    }

    fn next_f64(&mut self) -> f64 {
        let top_bits = self.next_u64() >> (u64::BITS - F64_MANTISSA_BITS);
        top_bits as f64 / (1u64 << F64_MANTISSA_BITS) as f64
    }
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(MIX_MULTIPLIER_A);
    z = (z ^ (z >> 27)).wrapping_mul(MIX_MULTIPLIER_B);
    z ^ (z >> 31)
}

pub fn gen_deterministic_seed(parts: &[u64]) -> impl FnMut() -> f64 + use<> {
    let mut rng = SplitMix64::from_parts(parts);
    move || rng.next_f64()
}

// probably overcomplicated, but fine for now
#[derive(Clone, Copy)]
pub(crate) enum DeterministicSeedPurpose {
    PickTime = 0,
    SendChance = 1,
}

pub(crate) fn create_deterministic_seed(
    peer_id: i64,
    date: LocalDate,
    kind: Event,
    purpose: DeterministicSeedPurpose,
) -> impl FnMut() -> f64 + use<> {
    gen_deterministic_seed(&[
        peer_id as u64,
        date.num_days_from_ce() as u64,
        kind as u64,
        purpose as u64,
    ])
}

#[cfg(test)]
#[path = "../../tests/unit/utils/random_test.rs"]
mod tests;
