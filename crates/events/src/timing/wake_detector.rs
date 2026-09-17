use chrono::{DateTime, Utc};
use state::EventsState;

use crate::clock::peer_utc_layers::PeerUtcLayers;
use crate::events::Event;
use crate::lever::NUDGE_EVERY;
use crate::timing::event_time::compute_event_time;
use crate::utils::duration::parse_str_to_minutes;

const NUDGE_EVERY_SECONDS: i64 = parse_str_to_minutes(NUDGE_EVERY) as i64 * 60;

pub struct WakeUpDetector;

impl WakeUpDetector {
    pub fn is_first_activity_today(
        peer_id: i64,
        layers: &PeerUtcLayers,
        previous_message: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> bool {
        let local_date = layers.from_utc_to_local(now);
        let day_start = compute_event_time(peer_id, local_date, Event::DayStart, layers);

        let is_after_day_start = now >= day_start;
        let had_no_activity_today = previous_message.is_none_or(|previous| previous < day_start);

        is_after_day_start && had_no_activity_today
    }

    pub fn is_get_up_reply(text: &str) -> bool {
        text.trim().to_lowercase() == "up"
    }

    pub fn should_nudge(events: &EventsState, now: DateTime<Utc>) -> bool {
        if events.got_up.is_some() { return false; }

        let since = events.last_nudge.or(events.woke_up);
        since.is_some_and(|since| now.timestamp() - since >= NUDGE_EVERY_SECONDS)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/timing/wake_detector_test.rs"]
mod tests;
