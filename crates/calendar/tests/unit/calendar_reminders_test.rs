use std::collections::BTreeMap;

use state::GoogleTokens;

use super::*;

const NOW: i64 = 1_800_000_000;

fn at(timestamp: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp, 0).unwrap()
}

fn minutes(count: i64) -> i64 {
    count * 60
}

fn event(id: &str, starts_at: i64) -> CalendarEvent {
    CalendarEvent { id: id.into(), title: "standup".into(), starts_at }
}

fn connected(upcoming: Vec<CalendarEvent>) -> CalendarState {
    CalendarState {
        tokens: Some(GoogleTokens::default()),
        upcoming,
        ..CalendarState::default()
    }
}

#[test]
fn an_event_gets_its_heads_up_once_it_is_close_enough() {
    let calendar = connected(vec![event("one", NOW + minutes(11))]);

    assert!(CalendarReminders::due_now(&calendar, at(NOW)).is_none());
    assert!(CalendarReminders::due_now(&calendar, at(NOW + minutes(1))).is_some());
}

#[test]
fn an_event_that_already_started_gets_no_heads_up() {
    let calendar = connected(vec![event("one", NOW - minutes(1))]);

    assert!(CalendarReminders::due_now(&calendar, at(NOW)).is_none());
}

#[test]
fn a_heads_up_is_never_sent_twice_for_one_event() {
    let mut calendar = connected(vec![event("one", NOW + minutes(5))]);

    assert!(CalendarReminders::due_now(&calendar, at(NOW)).is_some());

    CalendarReminders::record_sent_heads_up(&mut calendar, "one", at(NOW));

    assert!(CalendarReminders::due_now(&calendar, at(NOW)).is_none());
}

#[test]
fn the_nearest_event_is_the_one_that_gets_the_heads_up() {
    let calendar = connected(vec![event("one", NOW + minutes(3)), event("two", NOW + minutes(9))]);

    assert_eq!(CalendarReminders::due_now(&calendar, at(NOW)).unwrap().id, "one");
}

#[test]
fn only_a_connected_calendar_is_refreshed() {
    let disconnected = CalendarState::default();
    assert!(!CalendarReminders::should_refresh(&disconnected, at(NOW)));

    let never_refreshed = connected(Vec::new());
    assert!(CalendarReminders::should_refresh(&never_refreshed, at(NOW)));
}

#[test]
fn a_calendar_is_refreshed_again_once_the_interval_passed() {
    let refreshed = CalendarState { refreshed_at: Some(NOW), ..connected(Vec::new()) };

    assert!(!CalendarReminders::should_refresh(&refreshed, at(NOW + minutes(14))));
    assert!(CalendarReminders::should_refresh(&refreshed, at(NOW + minutes(15))));
}

#[test]
fn the_heads_up_says_how_long_is_left() {
    let event = event("one", NOW + minutes(7));

    assert_eq!(CalendarReminders::heads_up_text(&event, at(NOW)), "in 7 min: standup");
    assert_eq!(CalendarReminders::heads_up_text(&event, at(NOW + minutes(7))), "now: standup");
}

#[test]
fn storing_upcoming_events_forgets_the_old_heads_ups() {
    let mut calendar = connected(Vec::new());
    calendar.reminded = BTreeMap::from([
        ("old".to_owned(), NOW - minutes(60 * 24 * 3)),
        ("fresh".to_owned(), NOW - minutes(60)),
    ]);

    CalendarReminders::store_upcoming(&mut calendar, vec![event("one", NOW + minutes(30))], at(NOW));

    assert_eq!(calendar.refreshed_at, Some(NOW));
    assert_eq!(calendar.upcoming.len(), 1);
    assert!(!calendar.reminded.contains_key("old"));
    assert!(calendar.reminded.contains_key("fresh"));
}
