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
fn a_message_after_the_wake_window_is_not_a_waking_up() {
    let morning = day_start_on("2026-09-14".parse().unwrap());
    let layers = utc_plus_three();

    let edge = morning + WAKE_TIME_WINDOW_DURATION;
    assert!(WakeUpDetector::is_first_activity_today(PEER, &layers, None, edge));
    assert!(!WakeUpDetector::is_first_activity_today(PEER, &layers, None, edge + Duration::minutes(1)));
}

#[test]
fn the_wake_window_opens_at_day_start_and_lasts_the_lever() {
    let morning = day_start_on("2026-09-14".parse().unwrap());
    let window = WakeUpDetector::get_wake_window(morning);

    assert_eq!((*window.start(), *window.end()), (morning, morning + WAKE_TIME_WINDOW_DURATION));
}

#[test]
fn a_message_in_the_small_hours_is_not_a_waking_up_but_the_morning_one_is() {
    let date = "2026-09-14".parse().unwrap();
    let layers = utc_plus_three();

    let night = layers.apply_layer_1(date, 0.0) + Duration::hours(2);
    assert!(!WakeUpDetector::is_first_activity_today(PEER, &layers, None, night));

    let morning = day_start_on(date) + Duration::minutes(8);
    assert!(WakeUpDetector::is_first_activity_today(PEER, &layers, Some(night), morning));
}

#[test]
fn get_up_reply_matches_only_the_exact_word() {
    assert!(WakeUpDetector::is_get_up_reply("up"));
    assert!(WakeUpDetector::is_get_up_reply("  Up  "));
    assert!(!WakeUpDetector::is_get_up_reply("not up"));
    assert!(!WakeUpDetector::is_get_up_reply("getting up"));
}

#[test]
fn no_reminder_before_the_first_activity() {
    assert!(!WakeUpDetector::should_remind(&EventsState::default(), at(1_000_000)));
}

#[test]
fn reminds_once_the_interval_passed_since_waking() {
    let woke = 1_000_000;
    let events = EventsState { woke_up: Some(woke), ..EventsState::default() };

    assert!(!WakeUpDetector::should_remind(&events, at(woke + 19 * 60)));
    assert!(WakeUpDetector::should_remind(&events, at(woke + 20 * 60)));
}

#[test]
fn spaces_repeated_reminders_by_the_interval() {
    let last = 1_000_000;
    let events = EventsState { woke_up: Some(1), last_reminder: Some(last), ..EventsState::default() };

    assert!(!WakeUpDetector::should_remind(&events, at(last + 19 * 60)));
    assert!(WakeUpDetector::should_remind(&events, at(last + 20 * 60)));
}

#[test]
fn stops_nudging_after_the_peer_confirmed_getting_up() {
    let events = EventsState { woke_up: Some(1), got_up: Some(2), ..EventsState::default() };
    assert!(!WakeUpDetector::should_remind(&events, at(1_000_000)));
}
