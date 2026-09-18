// plain numeric constants for time unit conversions

pub const MILLIS_PER_SECOND: i32 = 1_000;
pub const SECONDS_PER_MINUTE: i32 = 60;
pub const MINUTES_PER_HOUR: i32 = 60;
pub const HOURS_PER_DAY: usize = 24;

pub const MILLIS_PER_MINUTE: i32 = MILLIS_PER_SECOND * SECONDS_PER_MINUTE;
pub const SECONDS_PER_HOUR: i32 = SECONDS_PER_MINUTE * MINUTES_PER_HOUR;
pub const MINUTES_PER_DAY: i32 = MINUTES_PER_HOUR * HOURS_PER_DAY as i32;
pub const SECONDS_PER_DAY: i32 = SECONDS_PER_HOUR * HOURS_PER_DAY as i32;
pub const MINUTES_PER_HALF_DAY: i32 = MINUTES_PER_DAY / 2;
pub const WESTMOST_OFFSET_HOURS: i32 = -11;

// wraps any hour count into the 0..23 range.
// 23 → 23
// 24 → 0
// 25 → 1
pub const fn wrap_hour_of_day(hour: usize) -> usize {
    hour % HOURS_PER_DAY
}

pub const fn wrap_to_half_day(minutes: i32) -> i32 {
    (minutes + MINUTES_PER_HALF_DAY).rem_euclid(2 * MINUTES_PER_HALF_DAY) - MINUTES_PER_HALF_DAY
}

pub const fn wrap_utc_offset_hours(hours: i32) -> i32 {
    (hours - WESTMOST_OFFSET_HOURS).rem_euclid(HOURS_PER_DAY as i32) + WESTMOST_OFFSET_HOURS
}

pub const fn hours_to_minutes(hours: i32) -> i32 {
    hours * MINUTES_PER_HOUR
}

pub const fn days_to_minutes(days: i32) -> i32 {
    days * MINUTES_PER_DAY
}

pub const fn minutes_to_seconds(minutes: i32) -> i32 {
    minutes * SECONDS_PER_MINUTE
}

pub const fn seconds_to_minutes(seconds: i32) -> i32 {
    seconds / SECONDS_PER_MINUTE
}

pub const fn seconds_to_days(seconds: i64) -> f64 {
    seconds as f64 / SECONDS_PER_DAY as f64
}

pub const fn minutes_to_days(minutes: i32) -> f64 {
    seconds_to_days(minutes_to_seconds(minutes) as i64)
}

pub const fn minutes_to_millis(minutes: f64) -> f64 {
    minutes * MILLIS_PER_MINUTE as f64
}

pub fn hours_to_minutes_rounded(hours: f64) -> i32 {
    (hours * MINUTES_PER_HOUR as f64).round() as i32
}

#[cfg(test)]
#[path = "../../tests/unit/utils/time_units_test.rs"]
mod tests;

// parses a duration string into minutes
pub const fn parse_str_to_minutes(duration: &str) -> i32 {
    let bytes = duration.as_bytes();
    let mut index = 0;
    let mut total = 0;
    let mut value = 0;
    let mut has_digits = false;
    assert!(!bytes.is_empty());

    while index < bytes.len() {
        match bytes[index] {
            b'0'..=b'9' => {
                value = value * 10 + (bytes[index] - b'0') as i32;
                has_digits = true;
            }
            unit @ (b'd' | b'h' | b'm') => {
                assert!(has_digits);
                total += value * match unit {
                    b'd' => days_to_minutes(1),
                    b'h' => hours_to_minutes(1),
                    _ => 1,
                };
                value = 0;
                has_digits = false;
            }
            _ => panic!(),
        }
        index += 1;
    }

    assert!(!has_digits);
    total
}

#[cfg(test)]
#[path = "../../tests/unit/utils/duration_test.rs"]
mod duration_tests;
