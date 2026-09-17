use chrono::Duration;
use state::{Activity, EventsState};

use super::*;

fn utc(raw: &str) -> DateTime<Utc> {
    raw.parse().unwrap()
}

#[test]
fn observing_a_message_records_when_and_clears_the_ignored_counter() {
    let mut events = EventsState { events_no_reply: 4, ..EventsState::default() };
    let at = utc("2026-09-11T07:30:00Z");

    ActivityRecorder::record_peer_message(&mut events, at);

    assert_eq!(events.date, Some(at.timestamp()));
    assert_eq!(events.events_no_reply, 0);

    let activity = events.activity.as_ref().unwrap();
    assert_eq!(activity.created_at, at.timestamp());
    assert_eq!(activity.updated_at, at.timestamp());
}

#[test]
fn a_message_lands_in_the_utc_hour_it_arrived_in() {
    let mut events = EventsState::default();
    ActivityRecorder::record_peer_message(&mut events, utc("2026-09-11T07:30:00Z"));
    ActivityRecorder::record_peer_message(&mut events, utc("2026-09-11T07:59:00Z"));
    ActivityRecorder::record_peer_message(&mut events, utc("2026-09-11T08:00:00Z"));

    let activity = events.activity.unwrap();
    assert!((activity.utc[7] - 2.0).abs() < 0.01, "{:?}", activity.utc[7]);
    assert_eq!(activity.utc[8], 1.0);
    assert_eq!(activity.utc[6], 0.0);
    assert_eq!(activity.utc[9], 0.0);
}

#[test]
fn one_half_life_of_silence_halves_what_came_before() {
    let mut activity = Activity {
        utc: [0.0; 24],
        created_at: utc("2026-09-01T00:00:00Z").timestamp(),
        updated_at: utc("2026-09-01T00:00:00Z").timestamp(),
    };
    ActivityRecorder::increment_peer_activity(&mut activity, utc("2026-09-01T07:00:00Z"));
    assert_eq!(activity.utc[7], 1.0);

    let half_life_later =
        utc("2026-09-01T07:00:00Z") + Duration::days(ACTIVITY_HALF_LIFE_DAYS as i64);
    ActivityRecorder::increment_peer_activity(&mut activity, half_life_later);

    assert!((activity.utc[7] - 1.5).abs() < 1e-9, "{:?}", activity.utc[7]);
}

#[test]
fn a_message_that_arrives_out_of_order_neither_decays_nor_rewinds_the_clock() {
    let mut activity = Activity {
        utc: [0.0; 24],
        created_at: utc("2026-09-01T00:00:00Z").timestamp(),
        updated_at: utc("2026-09-01T00:00:00Z").timestamp(),
    };
    let newest = utc("2026-09-20T10:00:00Z");
    ActivityRecorder::increment_peer_activity(&mut activity, newest);

    ActivityRecorder::increment_peer_activity(&mut activity, utc("2026-09-05T03:00:00Z"));

    assert_eq!(activity.utc[10], 1.0);
    assert_eq!(activity.utc[3], 1.0);
    assert_eq!(activity.updated_at, newest.timestamp());
}
