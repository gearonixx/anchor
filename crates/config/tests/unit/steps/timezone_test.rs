use chrono::FixedOffset;
use state::utils::time_units::minutes_to_seconds;

use super::*;

fn now() -> DateTime<Utc> {
    "2026-09-12T15:00:00Z".parse().unwrap()
}

#[test]
fn a_valid_offset_is_stored_with_the_time_it_was_set() {
    let mut peer_config = PeerConfig::default();
    let Answer::Accepted(confirmation) = Timezone.check_answer("+3", &mut peer_config, now()) else {
        panic!("expected +3 to be accepted");
    };
    assert!(confirmation.contains("UTC+03:00"), "{confirmation}");
    assert_eq!(
        peer_config.timezone,
        Some(TimezoneConfig {
            utc: FixedOffset::east_opt(minutes_to_seconds(180)).unwrap(),
            updated_at: now().timestamp(),
        })
    );
}

#[test]
fn an_impossible_zone_is_explained_and_nothing_is_stored() {
    let mut peer_config = PeerConfig::default();
    assert!(matches!(
        Timezone.check_answer("+15", &mut peer_config, now()),
        Answer::Rejected(Some(reason)) if reason.starts_with("<b>Invalid UTC offset.</b>")
    ));
    assert_eq!(peer_config.timezone, None);
}

#[test]
fn text_that_is_not_an_offset_asks_again() {
    let mut peer_config = PeerConfig::default();
    assert!(matches!(Timezone.check_answer("hello", &mut peer_config, now()), Answer::Rejected(None)));
    assert_eq!(peer_config.timezone, None);
}
