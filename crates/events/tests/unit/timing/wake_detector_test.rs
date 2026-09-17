use chrono::Duration;

use super::*;
use crate::types::LocalDate;

const PEER: i64 = 1_000_000_001;

fn utc_plus_three() -> PeerUtcLayers {
    PeerUtcLayers::with_offset_hours(3.0).unwrap()
}

fn at(timestamp: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp, 0).unwrap()
}

fn day_start_on(date: LocalDate) -> DateTime<Utc> {
    compute_event_time(PEER, date, Event::DayStart, &utc_plus_three())
}

#[test]
fn first_activity_is_the_first_message_after_day_start() {
    let morning = day_start_on("2026-09-14".parse().unwrap());
    let layers = utc_plus_three();

    let yesterday_evening = morning - Duration::hours(12);
    assert!(WakeUpDetector::is_first_activity_today(PEER, &layers, Some(yesterday_evening), morning));

    let already_wrote = morning + Duration::minutes(5);
    assert!(!WakeUpDetector::is_first_activity_today(PEER, &layers, Some(already_wrote), morning + Duration::minutes(10)));

    assert!(!WakeUpDetector::is_first_activity_today(PEER, &layers, None, morning - Duration::minutes(1)));
}

#[test]
fn get_up_reply_matches_only_the_exact_word() {
    assert!(WakeUpDetector::is_get_up_reply("up"));
    assert!(WakeUpDetector::is_get_up_reply("  Up  "));
    assert!(!WakeUpDetector::is_get_up_reply("not up"));
    assert!(!WakeUpDetector::is_get_up_reply("getting up"));
}

#[test]
fn no_nudge_before_the_first_activity() {
    assert!(!WakeUpDetector::should_nudge(&EventsState::default(), at(1_000_000)));
}

#[test]
fn nudges_once_the_interval_passed_since_waking() {
    let woke = 1_000_000;
    let events = EventsState { woke_up: Some(woke), ..EventsState::default() };

    assert!(!WakeUpDetector::should_nudge(&events, at(woke + 19 * 60)));
    assert!(WakeUpDetector::should_nudge(&events, at(woke + 20 * 60)));
}

#[test]
fn spaces_repeated_nudges_by_the_interval() {
    let last = 1_000_000;
    let events = EventsState { woke_up: Some(1), last_nudge: Some(last), ..EventsState::default() };

    assert!(!WakeUpDetector::should_nudge(&events, at(last + 19 * 60)));
    assert!(WakeUpDetector::should_nudge(&events, at(last + 20 * 60)));
}

#[test]
fn stops_nudging_after_the_peer_confirmed_getting_up() {
    let events = EventsState { woke_up: Some(1), got_up: Some(2), ..EventsState::default() };
    assert!(!WakeUpDetector::should_nudge(&events, at(1_000_000)));
}
