use super::*;

#[test]
fn common_ways_of_writing_an_offset_are_understood() {
    for (raw, minutes) in [
        ("+3", 180),
        ("3", 180),
        (" +3 ", 180),
        ("utc+3", 180),
        ("UTC +3", 180),
        ("GMT-5", -300),
        ("-5", -300),
        ("−5", -300),
        ("+5:30", 330),
        ("+05:30", 330),
        ("+0530", 330),
        ("5.5", 330),
        ("+5,75", 345),
        ("-3:30", -210),
        ("0", 0),
        ("UTC", 0),
        ("+14", 840),
        ("-12", -720),
    ] {
        assert_eq!(parse_utc_offset(raw), offset_from_minutes(minutes), "{raw}");
    }
}

#[test]
fn numbers_that_are_not_real_zones_are_rejected() {
    for raw in ["+15", "-13", "+14:30", "+5:20", "99", "123"] {
        assert_eq!(parse_utc_offset(raw), Err(OffsetError::NoSuchZone), "{raw}");
    }
}

#[test]
fn text_that_is_not_an_offset_is_rejected_without_panicking() {
    for raw in ["", "   ", "hello", "+3 hrs", "abc", "3.3", "+", "é1", "1é2", "+:30", "12345"] {
        assert_eq!(parse_utc_offset(raw), Err(OffsetError::NotAnOffset), "{raw}");
    }
}
