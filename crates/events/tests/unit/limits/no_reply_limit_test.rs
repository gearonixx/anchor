use std::collections::BTreeSet;

use super::*;

const SOME_MESSAGE_TIME: i64 = 1_700_000_000;

fn events_after_the_peer_wrote(timestamp: i64) -> EventsState {
    EventsState { last_message: Some(timestamp), ..EventsState::default() }
}

fn limits_over_a_thousand_peer_messages() -> BTreeSet<u32> {
    (0..1_000)
        .map(|minute| events_after_the_peer_wrote(SOME_MESSAGE_TIME + minute * 60))
        .map(|events| NoReplyLimit::since_the_peer_wrote(&events))
        .collect()
}

#[test]
fn every_peer_message_draws_one_of_the_two_limits() {
    assert_eq!(limits_over_a_thousand_peer_messages(), MAX_EVENTS_NO_REPLY.collect());
}

#[test]
fn the_limit_does_not_change_while_the_peer_stays_silent() {
    let mut events = events_after_the_peer_wrote(SOME_MESSAGE_TIME);
    let limit = NoReplyLimit::since_the_peer_wrote(&events);

    events.events_no_reply = limit - 1;

    assert_eq!(NoReplyLimit::since_the_peer_wrote(&events), limit);
}

#[test]
fn the_agent_goes_quiet_exactly_when_the_drawn_limit_is_reached() {
    let mut events = events_after_the_peer_wrote(SOME_MESSAGE_TIME);
    let limit = NoReplyLimit::since_the_peer_wrote(&events);

    events.events_no_reply = limit - 1;
    assert!(!NoReplyLimit::On.is_reached(&events));

    events.events_no_reply = limit;
    assert!(NoReplyLimit::On.is_reached(&events));
}

#[test]
fn the_limit_turned_off_is_never_reached() {
    let mut events = events_after_the_peer_wrote(SOME_MESSAGE_TIME);

    events.events_no_reply = *MAX_EVENTS_NO_REPLY.end() * 10;

    assert!(!NoReplyLimit::Off.is_reached(&events));
}
