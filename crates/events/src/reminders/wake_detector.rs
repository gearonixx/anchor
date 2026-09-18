use std::ops::RangeInclusive;

use chrono::{DateTime, Duration, Utc};

use crate::clock::peer_utc_layers::PeerUtcLayers;
use crate::events::Event;
use crate::lever::WAKE_TIME_WINDOW;
use crate::timing::event_time::compute_event_time;
use crate::utils::duration::parse_str_to_minutes;

// see this constant at lever.rs
// we should actually make this thing randomized
const WAKE_TIME_WINDOW_DURATION: Duration = Duration::minutes(parse_str_to_minutes(WAKE_TIME_WINDOW) as i64);

pub(crate) struct WakeUpDetector;

impl WakeUpDetector {
    pub(crate) fn is_first_activity_today(
        peer_id: i64,
        layers: &PeerUtcLayers,
        previous_message: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> bool {
        let local_date = layers.from_utc_to_local(now);
        let day_start = compute_event_time(peer_id, local_date, Event::DayStart, layers);

        let is_within_wake_window = Self::get_wake_window(day_start).contains(&now);

        let was_before_day_start = |message_date| message_date < day_start;

        // if previous < day_start, that means it was the day before
        // 'now' is outside the wake window, so is_wake = false
        // independently of that, the agent is is_asleep at night, so it does not respond
        let had_no_activity_since_day_start = previous_message.is_none_or(was_before_day_start);

        is_within_wake_window && had_no_activity_since_day_start
    }

    fn get_wake_window(day_start: DateTime<Utc>) -> RangeInclusive<DateTime<Utc>> {
        let wake_window_start = day_start;
        let wake_window_end = day_start + WAKE_TIME_WINDOW_DURATION;

        wake_window_start..=wake_window_end
    }
}

#[cfg(test)]
#[path = "../../tests/unit/reminders/wake_detector_test.rs"]
mod tests;
