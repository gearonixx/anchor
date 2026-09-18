use chrono::{DateTime, Utc};
use state::utils::time_units::{SECONDS_PER_MINUTE, parse_str_to_minutes};
use state::{CalendarEvent, CalendarState};

use crate::lever::{HEADS_UP_BEFORE, REFRESH_CALENDAR_EVERY, REMEMBER_SENT_FOR};

const HEADS_UP_BEFORE_SECONDS: i64 = parse_str_to_minutes(HEADS_UP_BEFORE) as i64 * 60;
const REFRESH_CALENDAR_EVERY_SECONDS: i64 = parse_str_to_minutes(REFRESH_CALENDAR_EVERY) as i64 * 60;
const REMEMBER_SENT_FOR_SECONDS: i64 = parse_str_to_minutes(REMEMBER_SENT_FOR) as i64 * 60;

pub struct CalendarReminders;

impl CalendarReminders {
    pub fn due_now(calendar: &CalendarState, now: DateTime<Utc>) -> Option<&CalendarEvent> {
        calendar
            .upcoming
            .iter()
            .find(|event| Self::is_heads_up_due(calendar, event, now))
    }

    pub fn should_refresh(calendar: &CalendarState, now: DateTime<Utc>) -> bool {
        let is_connected = calendar.is_connected();
        let is_stale = calendar
            .refreshed_at
            .is_none_or(|refreshed_at| now.timestamp() - refreshed_at >= REFRESH_CALENDAR_EVERY_SECONDS);

        is_connected && is_stale
    }

    pub fn heads_up_text(event: &CalendarEvent, now: DateTime<Utc>) -> String {
        let minutes_left = (event.starts_at - now.timestamp()).max(0) / i64::from(SECONDS_PER_MINUTE);
        let title = &event.title;

        match minutes_left {
            0 => format!("now: {title}"),
            minutes_left => format!("in {minutes_left} min: {title}"),
        }
    }

    pub fn store_upcoming(calendar: &mut CalendarState, upcoming: Vec<CalendarEvent>, now: DateTime<Utc>) {
        calendar.upcoming = upcoming;
        calendar.refreshed_at = Some(now.timestamp());
        calendar.forget_events_before(now.timestamp() - REMEMBER_SENT_FOR_SECONDS);
    }

    pub fn record_sent_heads_up(calendar: &mut CalendarState, event_id: &str, now: DateTime<Utc>) {
        calendar.reminded.insert(event_id.to_owned(), now.timestamp());
    }

    fn is_heads_up_due(calendar: &CalendarState, event: &CalendarEvent, now: DateTime<Utc>) -> bool {
        let starts_in = event.starts_at - now.timestamp();

        let has_not_started = starts_in >= 0;
        let is_close_enough = starts_in <= HEADS_UP_BEFORE_SECONDS;
        let is_not_sent_yet = !calendar.reminded.contains_key(&event.id);

        has_not_started && is_close_enough && is_not_sent_yet
    }
}

#[cfg(test)]
#[path = "../tests/unit/calendar_reminders_test.rs"]
mod tests;
