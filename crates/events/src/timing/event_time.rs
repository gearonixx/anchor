use chrono::{DateTime, Utc};

use crate::types::LocalDate;
use crate::events::Event;
use crate::clock::peer_utc_layers::PeerUtcLayers;
use crate::lever::{DAY_START_WINDOW, DAY_END_WINDOW};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlannedEvent {
    pub kind: Event,
    pub time: DateTime<Utc>,
    pub should_send: bool,
    pub text: &'static str,
}

// APPLY from COMPUTED LAYERS
pub fn apply_final_event_time(
    peer_id: i64,
    date: LocalDate,
    event: Event,
    clock: &PeerUtcLayers,
    forced_probabilities: bool,
) -> PlannedEvent {
    // should we send?
    let should_send = forced_probabilities || event.should_send(peer_id, date);

    PlannedEvent {
        kind: event,
        time: compute_event_time(peer_id, date, event, clock),
        should_send,
        text: event.text(),
    }
}

pub fn compute_event_time(peer_id: i64, local_date: LocalDate, event: Event, clock: &PeerUtcLayers) -> DateTime<Utc> {
    // 09:00–10:00
    let window = match event {
        Event::DayStart => DAY_START_WINDOW,
        Event::DayEnd => DAY_END_WINDOW,
    };

    // 09:37
    let chosen_minute = window.pick_random_minute_in_window(peer_id, local_date, event);

    // L2
    // L2 = +20 minutes
    // local_minutes = 09:57
    // could be moved to a separate fn
    let with_layer2_applied = chosen_minute + f64::from(clock.layer2);

    // L1
    // 09:57 local UTC+3
    // → 06:57 UTC
    clock.apply_layer_1(local_date, with_layer2_applied)
}

#[cfg(test)]
#[path = "../../tests/unit/timing/event_time_test.rs"]
mod tests;
