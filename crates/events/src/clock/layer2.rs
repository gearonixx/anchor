use chrono::{DateTime, Utc};
use state::Activity;
use state::utils::time_units::{
    HOURS_PER_DAY, minutes_to_days, wrap_hour_of_day, wrap_to_half_day,
    wrap_utc_offset_hours,
};

use crate::lever::{
    L2_RADIUS, L2_MIN_TOTAL_UTC_MESSAGES, L2_MIN_OBSERVED_DAYS, L2_QUIETEST_WINDOW_HOURS,
    L2_MAX_ACTIVITY_WHILE_QUIET, L2_NOISE_MINUTES, L2_SHIFT_SHARE,
};
use crate::utils::circular::longest_circular_run;
use state::utils::time_units::parse_str_to_minutes;

const QUIET_MIDPOINT_LOCAL_HOUR: f64 = 5.0;
const QUIET_TIE_SHARE: f64 = 0.01;

pub(crate) const MAX_RHYTHM_SHIFT_MINUTES: i32 = parse_str_to_minutes(L2_RADIUS);
const MIN_OBSERVED_DAYS: f64 = minutes_to_days(parse_str_to_minutes(L2_MIN_OBSERVED_DAYS));
const MIN_TOTAL_UTC_ACTIVITY: f64 = L2_MIN_TOTAL_UTC_MESSAGES as f64;

pub(crate) fn compute_layer2_from_utc_activity(
    activity: &Activity,
    now: DateTime<Utc>,
) -> Option<i32> {
    let observed_days = activity.observed_days(now);
    let total_utc_activity = activity.total();

    let has_enough_observed_days = observed_days >= MIN_OBSERVED_DAYS;
    let has_enough_activity = total_utc_activity >= MIN_TOTAL_UTC_ACTIVITY;

    if !has_enough_observed_days || !has_enough_activity { return None; }

    // [
    //     sum 00:00–08:00,
    //     sum 01:00–09:00,
    //     ...
    //     sum 23:00–07:00
    // ]
    let window_sums: Vec<f64> = (0..HOURS_PER_DAY)
        .map(|start| {
            (0..L2_QUIETEST_WINDOW_HOURS)
                .map(|hour| activity.utc[wrap_hour_of_day(start + hour)])
                .sum()
        })
        .collect();

    let quietest = window_sums.iter().copied().fold(f64::INFINITY, f64::min);
    let max_activity_while_quiet = total_utc_activity * L2_MAX_ACTIVITY_WHILE_QUIET;

    if quietest > max_activity_while_quiet { return None; }

    let tolerance = quietest + total_utc_activity * QUIET_TIE_SHARE;
    let quiet: Vec<bool> = window_sums.iter().map(|&sum| sum <= tolerance).collect();
    let (run_start, run_len) = longest_circular_run(&quiet)?;

    let middle_window_start = run_start as f64 + (run_len - 1) as f64 / 2.0;
    let sleep_midpoint_utc = middle_window_start + L2_QUIETEST_WINDOW_HOURS as f64 / 2.0;
    let offset = (QUIET_MIDPOINT_LOCAL_HOUR - sleep_midpoint_utc).round() as i32;

    Some(wrap_utc_offset_hours(offset))
}

pub(crate) fn merged_rhythm_shift(setting_offset_minutes: i32, lived_offset_minutes: i32) -> i32 {
    let lag = wrap_to_half_day(setting_offset_minutes - lived_offset_minutes);
    let beyond_noise = (lag.abs() - L2_NOISE_MINUTES).max(0);
    let shift = (f64::from(beyond_noise) * L2_SHIFT_SHARE).round() as i32;

    lag.signum() * shift.min(MAX_RHYTHM_SHIFT_MINUTES)
}
