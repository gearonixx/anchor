use chrono::Duration;

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
    SleepWindow::is_agent_asleep(PEER, &utc_plus_three(), now)
}

#[test]
fn the_agent_is_still_awake_for_a_while_after_sending_day_end() {
    let falls_asleep = day_end_of(night()) + FALLS_ASLEEP_AFTER_DAY_END_DURATION;

    assert!(!is_asleep(day_end_of(night())));
    assert!(!is_asleep(falls_asleep - Duration::minutes(1)));
    assert!(is_asleep(falls_asleep));
}

#[test]
fn the_agent_sleeps_until_day_start() {
    let wakes_up = day_start_after(night());

    assert!(is_asleep(wakes_up - Duration::minutes(1)));
    assert!(!is_asleep(wakes_up));
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
    let falls_asleep = day_end_of(date) + FALLS_ASLEEP_AFTER_DAY_END_DURATION;

    assert!(!is_asleep(falls_asleep - Duration::minutes(1)));
    assert!(is_asleep(falls_asleep));
    assert!(is_asleep(day_start_after(date) - Duration::minutes(1)));
}
