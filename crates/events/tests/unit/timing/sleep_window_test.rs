use std::collections::BTreeMap;

use chrono::Duration;
use state::ReminderState;

use super::*;
use crate::types::LocalDate;

const PEER: i64 = 1_000_000_001;

fn utc_plus_three() -> PeerUtcLayers {
    PeerUtcLayers::with_offset_hours(3.0).unwrap()
}

fn night() -> LocalDate {
    "2026-09-14".parse().unwrap()
}

fn day_end_of(date: LocalDate) -> DateTime<Utc> {
    compute_event_time(PEER, date, Event::DayEnd, &utc_plus_three())
}

fn day_start_after(date: LocalDate) -> DateTime<Utc> {
    compute_event_time(PEER, date.succ_opt().unwrap(), Event::DayStart, &utc_plus_three())
}

fn is_asleep(now: DateTime<Utc>) -> bool {
    SleepWindow::is_agent_asleep(PEER, &utc_plus_three(), &EventsState::default(), now)
}

fn is_asleep_after_going_to_bed(went_to_bed: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    let cycle = ReminderState {
        started: Some((went_to_bed - Duration::minutes(5)).timestamp()),
        confirmed: Some(went_to_bed.timestamp()),
        ..ReminderState::default()
    };
    let events = EventsState {
        reminders: BTreeMap::from([(Reminder::GoToSleep.name().to_owned(), cycle)]),
        ..EventsState::default()
    };

    SleepWindow::is_agent_asleep(PEER, &utc_plus_three(), &events, now)
}

#[test]
fn the_agent_stays_awake_for_the_whole_go_to_sleep_window() {
    let falls_asleep = day_end_of(night()) + GO_TO_SLEEP_WINDOW_DURATION;

    assert!(!is_asleep(day_end_of(night())));
    assert!(!is_asleep(falls_asleep - Duration::minutes(1)));
    assert!(is_asleep(falls_asleep));
}

#[test]
fn the_agent_lies_down_as_soon_as_the_peer_said_good_night() {
    let went_to_bed = day_end_of(night()) + Duration::minutes(20);

    assert!(!is_asleep_after_going_to_bed(went_to_bed, went_to_bed - Duration::minutes(1)));
    assert!(is_asleep_after_going_to_bed(went_to_bed, went_to_bed));
}

#[test]
fn a_good_night_from_another_night_does_not_put_the_agent_to_bed() {
    let last_night = day_end_of(night().pred_opt().unwrap()) + Duration::minutes(20);
    let tonight = day_end_of(night()) + Duration::minutes(20);

    assert!(!is_asleep_after_going_to_bed(last_night, tonight));
}

#[test]
fn the_agent_sleeps_until_day_start() {
    let wakes_up = day_start_after(night());

    assert!(is_asleep(wakes_up - Duration::minutes(1)));
    assert!(!is_asleep(wakes_up));
}

#[test]
fn the_agent_is_asleep_in_the_small_hours() {
    let morning_after = night().succ_opt().unwrap();
    let small_hours = utc_plus_three().apply_layer_1(morning_after, 0.0) + Duration::hours(3);

    assert!(is_asleep(small_hours));
}

#[test]
fn the_agent_is_awake_through_the_day() {
    let wakes_up = day_start_after(night());
    let next_day_end = day_end_of(night().succ_opt().unwrap());

    let mut now = wakes_up;
    while now < next_day_end {
        assert!(!is_asleep(now), "asleep at {now}");
        now += Duration::minutes(10);
    }
}

#[test]
fn a_day_end_after_midnight_still_starts_the_same_night() {
    let first: LocalDate = "2026-09-01".parse().unwrap();
    let date = (0..120)
        .map(|offset| first + Duration::days(offset))
        .find(|&date| day_end_of(date) >= utc_plus_three().apply_layer_1(date.succ_opt().unwrap(), 0.0))
        .unwrap();
    let falls_asleep = day_end_of(date) + GO_TO_SLEEP_WINDOW_DURATION;

    assert!(!is_asleep(falls_asleep - Duration::minutes(1)));
    assert!(is_asleep(falls_asleep));
    assert!(is_asleep(day_start_after(date) - Duration::minutes(1)));
}
