use chrono::TimeZone;

use super::*;

const PEER_ID: i64 = 1_000_000_001;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 14, 20, 12, 0).unwrap()
}

fn sent_at(sent_at: DateTime<Utc>) -> IncomingMessage {
    IncomingMessage {
        user_id: PEER_ID,
        message_id: 4242,
        sent_at,
        kind: MessageKind::Text("hello".to_string()),
    }
}

#[test]
fn a_message_that_arrives_right_away_is_answered() {
    assert!(!sent_at(now()).is_too_late_to_answer(now()));
    assert!(!sent_at(now() - chrono::Duration::seconds(30)).is_too_late_to_answer(now()));
}

#[test]
fn a_message_delivered_exactly_on_the_limit_is_still_answered() {
    assert!(!sent_at(now() - MAX_MESSAGE_LATENESS).is_too_late_to_answer(now()));
}

#[test]
fn a_message_held_back_longer_than_the_limit_is_not_answered() {
    let over_the_limit = MAX_MESSAGE_LATENESS + chrono::Duration::seconds(1);

    assert!(sent_at(now() - over_the_limit).is_too_late_to_answer(now()));
    assert!(sent_at(now() - chrono::Duration::hours(3)).is_too_late_to_answer(now()));
}

#[test]
fn a_reaction_ages_out_the_same_way_a_message_does() {
    let mut reaction = sent_at(now() - chrono::Duration::hours(2));
    reaction.kind = MessageKind::Reaction { emoji: "👍".to_string() };

    assert!(reaction.is_too_late_to_answer(now()));
}

#[test]
fn a_message_from_the_future_is_never_too_late() {
    assert!(!sent_at(now() + chrono::Duration::minutes(5)).is_too_late_to_answer(now()));
}
