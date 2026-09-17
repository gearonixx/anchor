use super::*;
use crate::clock::layer2::{MAX_RHYTHM_SHIFT_MINUTES, compute_layer2_from_utc_activity, merged_rhythm_shift};
use crate::lever::{L2_NOISE_MINUTES, L2_SHIFT_SHARE};
use crate::clock::activity_recorder::ActivityRecorder;
use state::utils::time_units::{
    MINUTES_PER_HALF_DAY, SECONDS_PER_HOUR, hours_to_minutes, minutes_to_seconds,
    wrap_to_half_day, wrap_utc_offset_hours,
};

const UTC_PLUS_THREE: i32 = 3;
const NEW_YORK: i32 = -5;
const VLADIVOSTOK: i32 = 10;

fn utc(raw: &str) -> DateTime<Utc> {
    raw.parse().unwrap()
}

fn date(raw: &str) -> LocalDate {
    raw.parse().unwrap()
}

fn chat_for_days(
    events: &mut EventsState,
    first_day: DateTime<Utc>,
    days: i64,
    offset_hours: i32,
    awake_local_hours: std::ops::Range<i64>,
) {
    for d in 0..days {
        for local_hour in awake_local_hours.clone() {
            for minute in [5, 35] {
                let at = first_day + Duration::days(d) + Duration::hours(local_hour)
                    - Duration::hours(i64::from(offset_hours))
                    + Duration::minutes(minute);
                ActivityRecorder::record_peer_message(events, at);
            }
        }
    }
}

fn inferred_offset(events: &EventsState, now: DateTime<Utc>) -> Option<i32> {
    compute_layer2_from_utc_activity(events.activity.as_ref()?, now)
}

fn peer_config_at(offset_hours: i32) -> PeerConfig {
    PeerConfig {
        timezone: Some(state::TimezoneConfig {
            utc: FixedOffset::east_opt(minutes_to_seconds(hours_to_minutes(offset_hours)))
                .unwrap(),
            updated_at: 0,
        }),
        ..PeerConfig::default()
    }
}

fn shift_for(setting_hours: i32, lived_like_hours: i32, awake: std::ops::Range<i64>) -> i32 {
    let mut events = EventsState::default();
    let start = utc("2026-09-01T00:00:00Z");
    chat_for_days(&mut events, start, 5, lived_like_hours, awake);
    PeerUtcLayers::resolve_two_utc_layers(&peer_config_at(setting_hours), &events, start + Duration::days(5))
        .unwrap()
        .layer2
}

#[test]
fn a_regular_day_reads_as_its_own_offset() {
    let mut events = EventsState::default();
    let start = utc("2026-09-01T00:00:00Z");
    chat_for_days(&mut events, start, 5, UTC_PLUS_THREE, 9..25);
    let now = start + Duration::days(5);
    assert_eq!(inferred_offset(&events, now), Some(UTC_PLUS_THREE));
}

#[test]
fn far_west_and_far_east_peers_are_told_apart() {
    let start = utc("2026-09-01T00:00:00Z");
    let now = start + Duration::days(5);

    let mut new_yorker = EventsState::default();
    chat_for_days(&mut new_yorker, start, 5, NEW_YORK, 9..25);
    assert_eq!(inferred_offset(&new_yorker, now), Some(NEW_YORK));

    let mut vladivostok = EventsState::default();
    chat_for_days(&mut vladivostok, start, 5, VLADIVOSTOK, 9..25);
    assert_eq!(inferred_offset(&vladivostok, now), Some(VLADIVOSTOK));
}

#[test]
fn a_night_owl_is_read_by_their_rhythm_not_their_configured_offset() {
    let mut events = EventsState::default();
    let start = utc("2026-09-01T00:00:00Z");
    chat_for_days(&mut events, start, 5, UTC_PLUS_THREE, 13..29);
    let inferred = inferred_offset(&events, start + Duration::days(5)).unwrap();
    assert_eq!(inferred, UTC_PLUS_THREE - 4);
}

#[test]
fn a_couple_of_days_is_not_enough_to_guess() {
    let mut events = EventsState::default();
    let start = utc("2026-09-01T00:00:00Z");
    chat_for_days(&mut events, start, 2, UTC_PLUS_THREE, 9..25);
    assert_eq!(inferred_offset(&events, start + Duration::days(2)), None);
}

#[test]
fn a_few_messages_are_not_enough_to_guess() {
    let mut events = EventsState::default();
    let start = utc("2026-09-01T00:00:00Z");
    chat_for_days(&mut events, start, 5, UTC_PLUS_THREE, 12..14);
    assert_eq!(inferred_offset(&events, start + Duration::days(5)), None);
}

#[test]
fn round_the_clock_chatter_has_no_sleep_to_read() {
    let mut events = EventsState::default();
    let start = utc("2026-09-01T00:00:00Z");
    chat_for_days(&mut events, start, 5, UTC_PLUS_THREE, 0..24);
    assert_eq!(inferred_offset(&events, start + Duration::days(5)), None);
}

#[test]
fn a_move_across_the_world_is_picked_up_as_old_days_fade() {
    let mut events = EventsState::default();
    let start = utc("2026-06-01T00:00:00Z");
    chat_for_days(&mut events, start, 30, UTC_PLUS_THREE, 9..25);
    let moved = start + Duration::days(30);
    chat_for_days(&mut events, moved, 45, NEW_YORK, 9..25);
    assert_eq!(inferred_offset(&events, moved + Duration::days(45)), Some(NEW_YORK));
}

#[test]
fn nothing_known_means_no_clock() {
    let now = utc("2026-09-01T00:00:00Z");
    assert_eq!(PeerUtcLayers::resolve_two_utc_layers(&PeerConfig::default(), &EventsState::default(), now), None);
}

#[test]
fn the_peers_own_setting_is_the_clock() {
    let now = utc("2026-09-01T00:00:00Z");
    let clock = PeerUtcLayers::resolve_two_utc_layers(&peer_config_at(UTC_PLUS_THREE), &EventsState::default(), now).unwrap();
    assert_eq!(clock.layer1.local_minus_utc(), UTC_PLUS_THREE * SECONDS_PER_HOUR);
    assert_eq!(clock.layer2, 0);
}

#[test]
fn activity_alone_never_sets_the_clock() {
    let mut events = EventsState::default();
    let start = utc("2026-09-01T00:00:00Z");
    chat_for_days(&mut events, start, 5, UTC_PLUS_THREE, 9..25);
    assert_eq!(PeerUtcLayers::resolve_two_utc_layers(&PeerConfig::default(), &events, start + Duration::days(5)), None);
}

#[test]
fn activity_never_overrides_the_peers_setting() {
    let mut events = EventsState::default();
    let start = utc("2026-09-01T00:00:00Z");
    chat_for_days(&mut events, start, 5, NEW_YORK, 9..25);
    let clock = PeerUtcLayers::resolve_two_utc_layers(&peer_config_at(UTC_PLUS_THREE), &events, start + Duration::days(5)).unwrap();
    assert_eq!(clock.layer1.local_minus_utc(), UTC_PLUS_THREE * SECONDS_PER_HOUR);
    assert!(clock.layer2.abs() <= MAX_RHYTHM_SHIFT_MINUTES);
}

#[test]
fn a_late_riser_gets_events_a_bit_later() {
    assert_eq!(
        shift_for(UTC_PLUS_THREE, UTC_PLUS_THREE, 13..29),
        merged_rhythm_shift(hours_to_minutes(4), 0)
    );
}

#[test]
fn an_early_bird_gets_events_a_bit_earlier() {
    assert_eq!(
        shift_for(UTC_PLUS_THREE, UTC_PLUS_THREE + 4, 9..25),
        merged_rhythm_shift(-hours_to_minutes(4), 0)
    );
}

#[test]
fn rhythm_shifts_stop_at_the_configured_radius_in_both_directions() {
    for lag in [hours_to_minutes(4), hours_to_minutes(6), hours_to_minutes(8)] {
        assert_eq!(merged_rhythm_shift(lag, 0), -merged_rhythm_shift(-lag, 0), "{lag}");
        assert!(merged_rhythm_shift(lag, 0).abs() <= MAX_RHYTHM_SHIFT_MINUTES, "{lag}");
    }

    assert_eq!(merged_rhythm_shift(MINUTES_PER_HALF_DAY - 1, 0), MAX_RHYTHM_SHIFT_MINUTES);
    assert_eq!(merged_rhythm_shift(1 - MINUTES_PER_HALF_DAY, 0), -MAX_RHYTHM_SHIFT_MINUTES);
}

#[test]
fn a_rhythm_that_matches_the_setting_shifts_nothing() {
    assert_eq!(shift_for(UTC_PLUS_THREE, UTC_PLUS_THREE, 9..25), 0);
}

#[test]
fn a_lag_inside_the_noise_band_is_ignored() {
    assert_eq!(merged_rhythm_shift(L2_NOISE_MINUTES, 0), 0);
    assert_eq!(merged_rhythm_shift(-L2_NOISE_MINUTES, 0), 0);
    assert_eq!(merged_rhythm_shift(L2_NOISE_MINUTES - 1, 0), 0);
}

#[test]
fn only_what_is_beyond_the_noise_band_moves_events_and_only_by_a_share_of_it() {
    let beyond_noise = hours_to_minutes(2);
    let expected = (f64::from(beyond_noise) * L2_SHIFT_SHARE).round() as i32;

    assert_eq!(merged_rhythm_shift(L2_NOISE_MINUTES + beyond_noise, 0), expected);
    assert_eq!(merged_rhythm_shift(-L2_NOISE_MINUTES - beyond_noise, 0), -expected);
}

#[test]
fn lags_wrap_around_the_date_line() {
    assert_eq!(
        merged_rhythm_shift(hours_to_minutes(12), hours_to_minutes(-11)),
        merged_rhythm_shift(-hours_to_minutes(1), 0)
    );
    assert_eq!(wrap_to_half_day(hours_to_minutes(23)), -hours_to_minutes(1));
}

#[test]
fn the_local_date_turns_over_at_the_peer_midnight() {
    let utc_plus_three = PeerUtcLayers::with_offset_hours(3.0).unwrap();
    assert_eq!(utc_plus_three.from_utc_to_local(utc("2026-09-10T20:59:00Z")), date("2026-09-10"));
    assert_eq!(utc_plus_three.from_utc_to_local(utc("2026-09-10T21:00:00Z")), date("2026-09-11"));
    assert_eq!(utc_plus_three.from_utc_to_local(utc("2026-09-11T20:30:00Z")), date("2026-09-11"));
}

#[test]
fn local_minutes_map_back_to_utc() {
    let utc_plus_three = PeerUtcLayers::with_offset_hours(3.0).unwrap();
    let eleventh = |minutes| utc_plus_three.apply_layer_1(date("2026-09-11"), minutes);

    assert_eq!(eleventh(9.0 * 60.0), utc("2026-09-11T06:00:00Z"));
    assert_eq!(eleventh(25.0 * 60.0), utc("2026-09-11T22:00:00Z"));
    assert_eq!(eleventh(0.0), utc("2026-09-10T21:00:00Z"));
}









#[test]
fn fractional_minutes_survive_the_trip_to_utc() {
    let utc_plus_three = PeerUtcLayers::with_offset_hours(3.0).unwrap();

    assert_eq!(
        utc_plus_three.apply_layer_1(date("2026-09-11"), 9.0 * 60.0 + 30.5),
        utc("2026-09-11T06:30:30Z")
    );
}

#[test]
fn the_local_date_turns_over_west_of_utc_too() {
    let new_york = PeerUtcLayers::with_offset_hours(f64::from(NEW_YORK)).unwrap();

    assert_eq!(new_york.from_utc_to_local(utc("2026-09-11T04:59:00Z")), date("2026-09-10"));
    assert_eq!(new_york.from_utc_to_local(utc("2026-09-11T05:00:00Z")), date("2026-09-11"));
}

#[test]
fn activity_too_thin_to_read_leaves_the_setting_untouched() {
    let mut events = EventsState::default();
    let start = utc("2026-09-01T00:00:00Z");
    chat_for_days(&mut events, start, 5, UTC_PLUS_THREE, 12..14);

    let clock =
        PeerUtcLayers::resolve_two_utc_layers(&peer_config_at(UTC_PLUS_THREE), &events, start + Duration::days(5))
            .unwrap();

    assert_eq!(clock.layer1.local_minus_utc(), UTC_PLUS_THREE * SECONDS_PER_HOUR);
    assert_eq!(clock.layer2, 0);
}


#[test]
fn wrapping_an_inferred_offset_never_changes_the_shift_it_produces() {
    for hours in -24..=24 {
        assert_eq!(
            merged_rhythm_shift(hours_to_minutes(UTC_PLUS_THREE), hours_to_minutes(wrap_utc_offset_hours(hours))),
            merged_rhythm_shift(hours_to_minutes(UTC_PLUS_THREE), hours_to_minutes(hours)),
            "{hours}"
        );
    }
}
