use crate::types::LocalDate;
use crate::utils::random::{DeterministicSeedPurpose, create_deterministic_seed};

// day start with a 90% chance (DAY_START_CHANCE = 0.9), day end with an 85% chance (DAY_END_CHANCE = 0.85).
const DAY_START_CHANCE: f64 = 0.9;
const DAY_END_CHANCE: f64 = 0.85;

// temporarily
const DAY_START_TEXT: &str = "hi";
const DAY_END_TEXT: &str = "bye";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    DayStart,
    DayEnd,
}

impl Event {
    pub const ALL: [Self; 2] = [Self::DayStart, Self::DayEnd];

    pub fn name(self) -> &'static str {
        match self {
            Self::DayStart => "day_start",
            Self::DayEnd => "day_end",
        }
    }

    pub(crate) fn should_send(self, peer_id: i64, date: LocalDate) -> bool {
        let mut random_seed =
            create_deterministic_seed(peer_id, date, self, DeterministicSeedPurpose::SendChance);

        Self::filter_by_chance(&mut random_seed, self.get_chance())
    }

    // inner helper
    pub(crate) fn filter_by_chance(
        random_seed: &mut impl FnMut() -> f64,
        probability: f64,
    ) -> bool {
        random_seed() < probability
    }

    pub(crate) fn get_chance(self) -> f64 {
        match self {
            Self::DayStart => DAY_START_CHANCE,
            Self::DayEnd => DAY_END_CHANCE,
        }
    }

    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::DayStart => DAY_START_TEXT,
            Self::DayEnd => DAY_END_TEXT,
        }
    }
}
