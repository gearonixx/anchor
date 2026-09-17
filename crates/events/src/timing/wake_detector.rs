use std::ops::RangeInclusive;

use chrono::{DateTime, Duration, Utc};
use state::EventsState;

use crate::clock::peer_utc_layers::PeerUtcLayers;
use crate::events::Event;
use crate::lever::{REMIND_EVERY, WAKE_TIME_WINDOW};
use crate::timing::event_time::compute_event_time;
use crate::utils::duration::parse_str_to_minutes;

const REMIND_EVERY_SECONDS: i64 = parse_str_to_minutes(REMIND_EVERY) as i64 * 60;
// see this constant at lever.rs
// we should actually make this thing randomized
const WAKE_TIME_WINDOW_DURATION: Duration = Duration::minutes(parse_str_to_minutes(WAKE_TIME_WINDOW) as i64);

pub struct WakeUpDetector;

impl WakeUpDetector {
    // temporary
    pub const GET_UP_START: &'static str = "get up";
    pub const GET_UP_REMIND: &'static str = "so, are you up?";
    pub const GET_UP_OK: &'static str = "well done!";

    pub fn is_first_activity_today(
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


    pub fn is_get_up_reply(text: &str) -> bool {
        text.trim().to_lowercase() == "up"
    }

    pub fn should_remind(events: &EventsState, now: DateTime<Utc>) -> bool {
        if events.got_up.is_some() { return false; }

        let since = events.last_reminder.or(events.woke_up);
        since.is_some_and(|since| now.timestamp() - since >= REMIND_EVERY_SECONDS)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/timing/wake_detector_test.rs"]
mod tests;
