use super::*;

#[test]
fn the_derived_constants_match_their_plain_values() {
    assert_eq!(SECONDS_PER_HOUR, 3_600);
    assert_eq!(MINUTES_PER_DAY, 1_440);
    assert_eq!(MILLIS_PER_MINUTE, 60_000);
    assert_eq!(SECONDS_PER_DAY, 86_400);
}

#[test]
fn whole_units_convert_upwards_and_downwards() {
    assert_eq!(hours_to_minutes(3), 180);
    assert_eq!(days_to_minutes(2), 2_880);
    assert_eq!(minutes_to_seconds(180), 10_800);
    assert_eq!(seconds_to_minutes(10_800), 180);
}

#[test]
fn a_negative_offset_survives_the_round_trip() {
    assert_eq!(minutes_to_seconds(-300), -18_000);
    assert_eq!(seconds_to_minutes(-18_000), -300);
    assert_eq!(hours_to_minutes(-11), -660);
}

#[test]
fn days_are_counted_as_fractions() {
    assert_eq!(seconds_to_days(86_400), 1.0);
    assert_eq!(seconds_to_days(43_200), 0.5);
    assert_eq!(minutes_to_days(2_880), 2.0);
    assert_eq!(minutes_to_days(720), 0.5);
}

#[test]
fn fractional_hours_round_to_whole_minutes() {
    assert_eq!(hours_to_minutes_rounded(5.5), 330);
    assert_eq!(hours_to_minutes_rounded(5.75), 345);
    assert_eq!(hours_to_minutes_rounded(-5.0), -300);
}

#[test]
fn minutes_after_midnight_become_milliseconds() {
    assert_eq!(minutes_to_millis(1.0), 60_000.0);
    assert_eq!(minutes_to_millis(0.5), 30_000.0);
}

#[test]
fn the_conversions_are_usable_in_const_context() {
    const HALF_DAY_MINUTES: i32 = hours_to_minutes(12);
    const TWO_DAYS: f64 = minutes_to_days(days_to_minutes(2));

    assert_eq!(HALF_DAY_MINUTES, 720);
    assert_eq!(TWO_DAYS, 2.0);
}

#[test]
fn an_hour_past_midnight_wraps_back_into_the_day() {
    assert_eq!(wrap_hour_of_day(0), 0);
    assert_eq!(wrap_hour_of_day(23), 23);
    assert_eq!(wrap_hour_of_day(24), 0);
    assert_eq!(wrap_hour_of_day(29), 5);
    assert_eq!(wrap_hour_of_day(48), 0);
}

#[test]
fn minutes_wrap_into_the_half_day_around_zero() {
    assert_eq!(wrap_to_half_day(0), 0);
    assert_eq!(wrap_to_half_day(hours_to_minutes(23)), -hours_to_minutes(1));
    assert_eq!(wrap_to_half_day(hours_to_minutes(13)), -hours_to_minutes(11));
    assert_eq!(wrap_to_half_day(-hours_to_minutes(13)), hours_to_minutes(11));
    assert_eq!(wrap_to_half_day(MINUTES_PER_HALF_DAY), -MINUTES_PER_HALF_DAY);
}

#[test]
fn utc_offsets_wrap_into_the_inhabited_range() {
    assert_eq!(wrap_utc_offset_hours(3), 3);
    assert_eq!(wrap_utc_offset_hours(-21), 3);
    assert_eq!(wrap_utc_offset_hours(12), 12);
    assert_eq!(wrap_utc_offset_hours(13), -11);
    assert_eq!(wrap_utc_offset_hours(-12), 12);
    assert_eq!(wrap_utc_offset_hours(WESTMOST_OFFSET_HOURS), WESTMOST_OFFSET_HOURS);
}
