use chrono::Duration;

use super::*;

const PEER: i64 = 1_000_000_001;
const OTHER_PEER: i64 = 1_311_749_836;
const THIRD_PEER: i64 = 424_242;
const THREE_YEARS: i64 = 3 * 365;

fn utc_plus_three() -> PeerUtcLayers {
    PeerUtcLayers::with_offset_hours(3.0).unwrap()
}

fn new_york() -> PeerUtcLayers {
    PeerUtcLayers::with_offset_hours(-5.0).unwrap()
}

fn days(first: &str, count: i64) -> impl Iterator<Item =LocalDate> {
    let first: LocalDate = first.parse().unwrap();
    (0..count).map(move |offset| first + Duration::days(offset))
}

fn day_start_at(peer_id: i64, date: LocalDate) -> DateTime<Utc> {
    apply_final_event_time(peer_id, date, Event::DayStart, &utc_plus_three(), false).time
}

fn local_minutes(planned: &PlannedEvent, date: LocalDate, clock: &PeerUtcLayers) -> f64 {
    (planned.time - clock.apply_layer_1(date, 0.0)).num_seconds() as f64 / 60.0
}

#[test]
fn a_day_is_planned_the_same_way_after_every_restart() {
    for date in days("2026-09-01", 60) {
        for kind in Event::ALL {
            assert_eq!(
                apply_final_event_time(PEER, date, kind, &utc_plus_three(), false),
                apply_final_event_time(PEER, date, kind, &utc_plus_three(), false)
            );
        }
    }
}

#[test]
fn peers_do_not_share_one_synchronized_schedule() {
    let identical = days("2026-09-01", 30)
        .filter(|&date| day_start_at(PEER, date) == day_start_at(OTHER_PEER, date))
        .count();
    assert!(identical * 2 < 30, "identical={identical}");
}

#[test]
fn the_same_routine_lands_on_each_peers_own_local_clock() {
    for date in days("2026-09-01", 14) {
        let in_utc_plus_three =
            apply_final_event_time(PEER, date, Event::DayStart, &utc_plus_three(), false);
        let in_new_york =
            apply_final_event_time(PEER, date, Event::DayStart, &new_york(), false);
        assert_eq!(
            local_minutes(&in_utc_plus_three, date, &utc_plus_three()),
            local_minutes(&in_new_york, date, &new_york())
        );
        assert_eq!((in_new_york.time - in_utc_plus_three.time).num_hours(), 8);
    }
}

#[test]
fn a_rhythm_shift_moves_every_event_by_the_same_minutes() {
    let shifted = PeerUtcLayers {
        layer2: 45,
        ..utc_plus_three()
    };
    for date in days("2026-09-01", 14) {
        for kind in Event::ALL {
            let moved = apply_final_event_time(PEER, date, kind, &shifted, false).time
                - apply_final_event_time(PEER, date, kind, &utc_plus_three(), false).time;
            assert_eq!(moved.num_minutes(), 45);
        }
    }
}

#[test]
fn mornings_and_nights_stay_inside_their_windows() {
    for peer in [PEER, OTHER_PEER, THIRD_PEER] {
        for date in days("2026-01-01", THREE_YEARS) {
            let local_minutes_of = |kind| {
                let planned = apply_final_event_time(peer, date, kind, &utc_plus_three(), false);
                local_minutes(&planned, date, &utc_plus_three())
            };
            let morning = local_minutes_of(Event::DayStart);
            let night = local_minutes_of(Event::DayEnd);

            assert!(
                (DAY_START_WINDOW.start..=DAY_START_WINDOW.end).contains(&morning),
                "{date} morning={morning}"
            );
            assert!(
                (DAY_END_WINDOW.start..=DAY_END_WINDOW.end).contains(&night),
                "{date} night={night}"
            );
        }
    }
}

#[test]
fn the_agent_skips_a_few_days_but_not_many() {
    for kind in Event::ALL {
        let sent = days("2026-01-01", THREE_YEARS)
            .filter(|&date| {
                apply_final_event_time(PEER, date, kind, &utc_plus_three(), false).should_send
            })
            .count();
        let rate = sent as f64 / THREE_YEARS as f64;
        assert!((rate - kind.get_chance()).abs() < 0.03, "{} rate={rate}", kind.name());
    }
}

#[test]
fn forced_probabilities_send_every_day_without_moving_the_time() {
    for kind in Event::ALL {
        for date in days("2026-01-01", THREE_YEARS) {
            let forced = apply_final_event_time(PEER, date, kind, &utc_plus_three(), true);
            let unforced = apply_final_event_time(PEER, date, kind, &utc_plus_three(), false);

            assert!(forced.should_send, "{date} {}", kind.name());
            assert_eq!(forced.time, unforced.time);
        }
    }
}
