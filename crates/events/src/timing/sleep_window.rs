use chrono::{DateTime, Duration, Utc};

use crate::clock::peer_utc_layers::PeerUtcLayers;
use crate::timing::event_time::compute_event_time;
use crate::events::Event;
use crate::lever::FALLS_ASLEEP_AFTER_DAY_END;
use crate::utils::duration::parse_str_to_minutes;

const FALLS_ASLEEP_AFTER_DAY_END_DURATION: Duration =
    Duration::minutes(parse_str_to_minutes(FALLS_ASLEEP_AFTER_DAY_END) as i64);

pub struct SleepWindow;

// keeps the agent unavailable during the night
impl SleepWindow {
    pub fn is_agent_asleep(peer_id: i64, layers: &PeerUtcLayers, now: DateTime<Utc>) -> bool {
        let today = layers.from_utc_to_local(now);
        let yesterday = today.pred_opt().unwrap_or(today);

        [yesterday, today].into_iter().any(|night| {
            let falls_asleep =
                // the agent stays available for this long after day end
                compute_event_time(peer_id, night, Event::DayEnd, layers) + FALLS_ASLEEP_AFTER_DAY_END_DURATION;

            let wakes_up = night
                .succ_opt()
                .map(|morning| compute_event_time(peer_id, morning, Event::DayStart, layers));

            wakes_up.is_some_and(|wakes_up| (falls_asleep..wakes_up).contains(&now))
        })
    }
}

#[cfg(test)]
#[path = "../../tests/unit/timing/sleep_window_test.rs"]
mod tests;
