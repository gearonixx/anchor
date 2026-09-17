use chrono::FixedOffset;
use state::utils::time_units::{hours_to_minutes, minutes_to_seconds};

const WESTMOST_OFFSET_MINUTES: i32 = hours_to_minutes(-12);
const EASTMOST_OFFSET_MINUTES: i32 = hours_to_minutes(14);
const ZONE_MINUTE_MARKS: [i32; 3] = [0, 30, 45];
const MAX_HOUR_DIGITS: usize = 2;
const MINUTE_DIGITS: usize = 2;
const UTC_PREFIXES: [&str; 2] = ["utc", "gmt"];
const MINUS_SIGNS: [char; 3] = ['-', '−', '–'];
const DECIMAL_SEPARATORS: [char; 2] = ['.', ','];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OffsetError {
    NotAnOffset,
    NoSuchZone,
}

pub(crate) fn parse_utc_offset(raw: &str) -> Result<FixedOffset, OffsetError> {
    let compact: String = raw
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    if compact.is_empty() {
        return Err(OffsetError::NotAnOffset);
    }

    let unprefixed = UTC_PREFIXES
        .iter()
        .find_map(|prefix| compact.strip_prefix(prefix))
        .unwrap_or(&compact);
    if unprefixed.is_empty() {
        return offset_from_minutes(0);
    }

    let (sign, magnitude) = match unprefixed.strip_prefix('+') {
        Some(rest) => (1, rest),
        None => match unprefixed.strip_prefix(&MINUS_SIGNS[..]) {
            Some(rest) => (-1, rest),
            None => (1, unprefixed),
        },
    };

    let (hours, minutes) = split_hours_and_minutes(magnitude).ok_or(OffsetError::NotAnOffset)?;
    let total = sign * (hours_to_minutes(hours) + minutes);
    if !ZONE_MINUTE_MARKS.contains(&minutes)
        || !(WESTMOST_OFFSET_MINUTES..=EASTMOST_OFFSET_MINUTES).contains(&total)
    {
        return Err(OffsetError::NoSuchZone);
    }
    offset_from_minutes(total)
}

fn offset_from_minutes(minutes: i32) -> Result<FixedOffset, OffsetError> {
    FixedOffset::east_opt(minutes_to_seconds(minutes)).ok_or(OffsetError::NoSuchZone)
}

fn split_hours_and_minutes(magnitude: &str) -> Option<(i32, i32)> {
    if let Some((hours, fraction)) = magnitude.split_once(DECIMAL_SEPARATORS) {
        let minutes = match fraction {
            "0" | "00" => 0,
            "5" | "50" => 30,
            "75" => 45,
            _ => return None,
        };
        return Some((digits(hours, 1..=MAX_HOUR_DIGITS)?, minutes));
    }

    if let Some((hours, minutes)) = magnitude.split_once(':') {
        return Some((
            digits(hours, 1..=MAX_HOUR_DIGITS)?,
            digits(minutes, MINUTE_DIGITS..=MINUTE_DIGITS)?,
        ));
    }

    if !magnitude.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    if magnitude.len() <= MAX_HOUR_DIGITS {
        return Some((digits(magnitude, 1..=MAX_HOUR_DIGITS)?, 0));
    }

    let (hours, minutes) = magnitude.split_at(magnitude.len() - MINUTE_DIGITS);
    Some((
        digits(hours, 1..=MAX_HOUR_DIGITS)?,
        digits(minutes, MINUTE_DIGITS..=MINUTE_DIGITS)?,
    ))
}

fn digits(text: &str, allowed_len: std::ops::RangeInclusive<usize>) -> Option<i32> {
    if !allowed_len.contains(&text.len()) || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
#[path = "../../tests/unit/utils/utc_offset_test.rs"]
mod tests;
