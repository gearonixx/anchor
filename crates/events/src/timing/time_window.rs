use state::utils::time_units::{MINUTES_PER_DAY, hours_to_minutes};

use crate::types::LocalDate;
use crate::events::Event;
use crate::utils::random::{DeterministicSeedPurpose, create_deterministic_seed};

// just holds the bounds of a time window
// 09:00–10:00
#[derive(Clone, Copy)]
pub(crate) struct TimeWindow {
    pub(crate) start: f64,
    pub(crate) end: f64,
}

impl TimeWindow {
    pub(crate) const fn new(start: &str, end: &str) -> Self {
        let start = Self::parse_time_str(start);
        let end = Self::parse_time_str(end);
        let end = if end <= start { end + MINUTES_PER_DAY } else { end };

        Self { start: start as f64, end: end as f64 }
    }

    const fn parse_time_str(clock: &str) -> i32 {
        let bytes = clock.as_bytes();
        let hours = (bytes[0] - b'0') as i32 * 10 + (bytes[1] - b'0') as i32;
        let minutes = (bytes[3] - b'0') as i32 * 10 + (bytes[4] - b'0') as i32;

        hours_to_minutes(hours) + minutes
    }

    // picks a random minute inside the window (not the hour)
    pub(crate) fn pick_random_minute_in_window(self, peer_id: i64, date: LocalDate, kind: Event) -> f64 {
        let mut random_seed =
            create_deterministic_seed(peer_id, date, kind, DeterministicSeedPurpose::PickTime);
        let minute_count = self.end - self.start + 1.0;

        self.start + (random_seed() * minute_count).floor()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/timing/time_window_test.rs"]
mod tests;
