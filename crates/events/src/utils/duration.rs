use state::utils::time_units::{days_to_minutes, hours_to_minutes};

// parses a duration string into minutes
pub(crate) const fn parse_str_to_minutes(duration: &str) -> i32 {
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
mod tests;
