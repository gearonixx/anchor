use chrono::{DateTime, Utc};
use state::{EventsState, ReminderState};

use crate::clock::peer_utc_layers::PeerUtcLayers;
use crate::reminders::wake_detector::WakeUpDetector;

use crate::lever::{GO_TO_SLEEP_WINDOW, REMIND_TO_GET_UP_EVERY, REMIND_TO_SLEEP_EVERY};
use crate::utils::duration::parse_str_to_minutes;

const REMIND_TO_GET_UP_EVERY_SECONDS: i64 = parse_str_to_minutes(REMIND_TO_GET_UP_EVERY) as i64 * 60;
const REMIND_TO_SLEEP_EVERY_SECONDS: i64 = parse_str_to_minutes(REMIND_TO_SLEEP_EVERY) as i64 * 60;
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

    pub fn is_started_by_message(
        self,
        peer_id: i64,
        layers: &PeerUtcLayers,
        previous_message: Option<DateTime<Utc>>,
        sent_at: DateTime<Utc>,
    ) -> bool {
        match self {
            Self::WakeUp => {
                WakeUpDetector::is_first_activity_today(peer_id, layers, previous_message, sent_at)
            }
            Self::GoToSleep => false,
        }
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
        match (self.cycle_lasts_seconds(), cycle.started) {
            (Some(lasts), Some(started)) => now.timestamp() - started > lasts,
            _ => false,
        }
    }

    fn remind_every_seconds(self) -> i64 {
        match self {
            Self::WakeUp => REMIND_TO_GET_UP_EVERY_SECONDS,
            Self::GoToSleep => REMIND_TO_SLEEP_EVERY_SECONDS,
        }
    }

    fn cycle_lasts_seconds(self) -> Option<i64> {
        match self {
            Self::WakeUp => None,
            Self::GoToSleep => Some(GO_TO_SLEEP_WINDOW_SECONDS),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/reminders/reminder_test.rs"]
mod tests;
