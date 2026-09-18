use std::ops::RangeInclusive;

use chrono::{DateTime, Duration, Utc};
use state::{EventsState, ReminderState};

use crate::clock::peer_utc_layers::PeerUtcLayers;
use crate::events::Event;
use crate::timing::event_time::compute_event_time;

use crate::lever::{GO_TO_SLEEP_WINDOW, REMIND_TO_GET_UP_EVERY, REMIND_TO_SLEEP_EVERY, WAKE_TIME_WINDOW};
use state::utils::time_units::parse_str_to_minutes;

const REMIND_TO_GET_UP_EVERY_SECONDS: i64 = parse_str_to_minutes(REMIND_TO_GET_UP_EVERY) as i64 * 60;
const REMIND_TO_SLEEP_EVERY_SECONDS: i64 = parse_str_to_minutes(REMIND_TO_SLEEP_EVERY) as i64 * 60;
const WAKE_TIME_WINDOW_SECONDS: i64 = parse_str_to_minutes(WAKE_TIME_WINDOW) as i64 * 60;
const GO_TO_SLEEP_WINDOW_SECONDS: i64 = parse_str_to_minutes(GO_TO_SLEEP_WINDOW) as i64 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reminder {
    WakeUp,
    GoToSleep,
}

impl Reminder {
    pub const ALL: [Self; 2] = [Self::WakeUp, Self::GoToSleep];

    pub fn name(self) -> &'static str {
        match self {
            Self::WakeUp => "wake_up",
            Self::GoToSleep => "go_to_sleep",
        }
    }

    pub fn prompt(self) -> &'static str {
        match self {
            Self::WakeUp => "confirm up",
            Self::GoToSleep => "confirm down",
        }
    }

    pub fn reminding_text(self) -> &'static str {
        match self {
            Self::WakeUp => "up pending",
            Self::GoToSleep => "down pending",
        }
    }

    pub fn ok(self) -> &'static str {
        match self {
            Self::WakeUp => "up confirmed",
            Self::GoToSleep => "down confirmed",
        }
    }

    pub fn is_confirmation(self, text: &str) -> bool {
        let said = text.trim().to_lowercase();

        match self {
            // temporary but ok
            Self::WakeUp => said == "up",
            Self::GoToSleep => said == "down",
        }
    }

    pub fn anchor_event(self) -> Event {
        match self {
            Self::WakeUp => Event::DayStart,
            Self::GoToSleep => Event::DayEnd,
        }
    }

    pub fn should_start(
        self,
        peer_id: i64,
        events: &EventsState,
        layers: &PeerUtcLayers,
        now: DateTime<Utc>,
    ) -> bool {
        self.get_running_window(peer_id, layers, now)
            .is_some_and(|window| !self.has_started_within(events, window))
    }

    fn get_running_window(
        self,
        peer_id: i64,
        layers: &PeerUtcLayers,
        now: DateTime<Utc>,
    ) -> Option<RangeInclusive<DateTime<Utc>>> {
        let today = layers.from_utc_to_local(now);
        let yesterday = today.pred_opt().unwrap_or(today);

        [yesterday, today]
            .into_iter()
            .map(|date| {
                let opens = compute_event_time(peer_id, date, self.anchor_event(), layers);

                opens..=opens + self.window_lasts()
            })
            .find(|window| window.contains(&now))
    }

    fn has_started_within(self, events: &EventsState, window: RangeInclusive<DateTime<Utc>>) -> bool {
        self.cycle_of(events)
            .and_then(|cycle| cycle.started)
            .and_then(|started| DateTime::from_timestamp(started, 0))
            .is_some_and(|started| window.contains(&started))
    }

    fn window_lasts(self) -> Duration {
        Duration::seconds(self.window_seconds())
    }

    pub fn cycle_of(self, events: &EventsState) -> Option<&ReminderState> {
        events.reminders.get(self.name())
    }

    pub fn start_cycle(self, events: &mut EventsState, now: DateTime<Utc>) {
        events.reminders.insert(self.name().to_owned(), ReminderState::started_at(now.timestamp()));
    }

    pub fn confirm_cycle(self, events: &mut EventsState, now: DateTime<Utc>) {
        self.cycle_mut(events).confirmed = Some(now.timestamp());
    }

    pub fn record_sent(self, events: &mut EventsState, now: DateTime<Utc>) {
        self.cycle_mut(events).last_reminder = Some(now.timestamp());
    }

    fn cycle_mut(self, events: &mut EventsState) -> &mut ReminderState {
        events.reminders.entry(self.name().to_owned()).or_default()
    }

    pub fn is_awaiting_confirmation(self, events: &EventsState) -> bool {
        self.cycle_of(events).is_some_and(ReminderState::is_awaiting_confirmation)
    }

    pub fn should_remind(self, events: &EventsState, now: DateTime<Utc>) -> bool {
        self.cycle_of(events).is_some_and(|cycle| self.is_reminder_due(cycle, now))
    }

    fn is_reminder_due(self, cycle: &ReminderState, now: DateTime<Utc>) -> bool {
        if cycle.confirmed.is_some() { return false; }
        if self.has_cycle_run_out(cycle, now) { return false; }

        let since = cycle.last_reminder.or(cycle.started);
        since.is_some_and(|since| now.timestamp() - since >= self.remind_every_seconds())
    }

    fn has_cycle_run_out(self, cycle: &ReminderState, now: DateTime<Utc>) -> bool {
        cycle
            .started
            .is_some_and(|started| now.timestamp() - started > self.window_seconds())
    }

    fn remind_every_seconds(self) -> i64 {
        match self {
            Self::WakeUp => REMIND_TO_GET_UP_EVERY_SECONDS,
            Self::GoToSleep => REMIND_TO_SLEEP_EVERY_SECONDS,
        }
    }

    fn window_seconds(self) -> i64 {
        match self {
            Self::WakeUp => WAKE_TIME_WINDOW_SECONDS,
            Self::GoToSleep => GO_TO_SLEEP_WINDOW_SECONDS,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/reminders/reminder_test.rs"]
mod tests;
