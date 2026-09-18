use std::ops::RangeInclusive;

use chrono::{DateTime, Duration, Utc};
use state::EventsState;

use crate::clock::peer_utc_layers::PeerUtcLayers;
use crate::timing::event_time::compute_event_time;
use crate::events::Event;
use crate::lever::GO_TO_SLEEP_WINDOW;
use crate::reminders::reminder::Reminder;
use state::utils::time_units::parse_str_to_minutes;

const GO_TO_SLEEP_WINDOW_DURATION: Duration =
    Duration::minutes(parse_str_to_minutes(GO_TO_SLEEP_WINDOW) as i64);

pub struct SleepWindow;

// keeps the agent unavailable during the night
impl SleepWindow {
    pub fn is_agent_asleep(
        peer_id: i64,
        layers: &PeerUtcLayers,
        events: &EventsState,
        now: DateTime<Utc>,
    ) -> bool {
        let today = layers.from_utc_to_local(now);
        let yesterday = today.pred_opt().unwrap_or(today);

        [yesterday, today].into_iter().any(|night| {
            let falls_asleep =
                Self::falls_asleep(compute_event_time(peer_id, night, Event::DayEnd, layers), events);

            let wakes_up = night
                .succ_opt()
                .map(|morning| compute_event_time(peer_id, morning, Event::DayStart, layers));

            wakes_up.is_some_and(|wakes_up| (falls_asleep..wakes_up).contains(&now))
        })
    }

    fn falls_asleep(day_end: DateTime<Utc>, events: &EventsState) -> DateTime<Utc> {
        let go_to_sleep_window = Self::get_go_to_sleep_window(day_end);
        let stays_awake_until = *go_to_sleep_window.end();

        Self::went_to_bed_within(events, go_to_sleep_window).unwrap_or(stays_awake_until)
    }

    fn get_go_to_sleep_window(day_end: DateTime<Utc>) -> RangeInclusive<DateTime<Utc>> {
        let go_to_sleep_window_start = day_end;
        let go_to_sleep_window_end = day_end + GO_TO_SLEEP_WINDOW_DURATION;

        go_to_sleep_window_start..=go_to_sleep_window_end
    }

    fn went_to_bed_within(
        events: &EventsState,
        window: RangeInclusive<DateTime<Utc>>,
    ) -> Option<DateTime<Utc>> {
        let cycle = Reminder::GoToSleep.cycle_of(events)?;
        let said_good_night = DateTime::from_timestamp(cycle.confirmed?, 0)?;

        window.contains(&said_good_night).then_some(said_good_night)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/timing/sleep_window_test.rs"]
mod tests;
