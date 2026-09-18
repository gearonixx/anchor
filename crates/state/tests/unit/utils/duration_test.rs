use super::*;

#[test]
fn duration_settings_accept_days_hours_and_minutes() {
    for (duration, minutes) in [
        ("1h", 60), ("1h45m", 105), ("45m", 45), ("2h", 120), ("0m", 0),
        ("2d", 2880), ("3d", 4320), ("2d12h", 3600),
    ] {
        assert_eq!(parse_str_to_minutes(duration), minutes);
    }
}

#[test]
fn duration_settings_reject_invalid_values() {
    for duration in ["", "h", "1h45", "-1h", "1w"] {
        assert!(std::panic::catch_unwind(|| parse_str_to_minutes(duration)).is_err());
    }
}
