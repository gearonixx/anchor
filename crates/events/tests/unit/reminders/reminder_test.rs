use std::collections::BTreeMap;

use chrono::Duration;

use super::*;

const PEER: i64 = 1_000_000_001;

const STARTED: i64 = 1_000_000;

fn at(timestamp: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp, 0).unwrap()
}

fn minutes(count: i64) -> i64 {
    count * 60
}

fn minutes_after(count: i64) -> Duration {
    Duration::minutes(count)
}

fn cycle(reminder: Reminder, state: ReminderState) -> EventsState {
    EventsState {
        reminders: BTreeMap::from([(reminder.name().to_owned(), state)]),
        ..EventsState::default()
    }
}

#[test]
fn nothing_is_reminded_before_a_cycle_started() {
    assert!(!Reminder::WakeUp.should_remind(&EventsState::default(), at(STARTED)));
    assert!(!Reminder::GoToSleep.should_remind(&EventsState::default(), at(STARTED)));
}

#[test]
fn the_wake_up_cycle_reminds_every_twenty_minutes() {
    let events = cycle(Reminder::WakeUp, ReminderState::started_at(STARTED));

    assert!(!Reminder::WakeUp.should_remind(&events, at(STARTED + minutes(19))));
    assert!(Reminder::WakeUp.should_remind(&events, at(STARTED + minutes(20))));
}

#[test]
fn the_go_to_sleep_cycle_reminds_every_eleven_minutes() {
    let events = cycle(Reminder::GoToSleep, ReminderState::started_at(STARTED));

    assert!(!Reminder::GoToSleep.should_remind(&events, at(STARTED + minutes(10))));
    assert!(Reminder::GoToSleep.should_remind(&events, at(STARTED + minutes(11))));
}

#[test]
fn the_interval_counts_from_the_last_reminder() {
    let last = STARTED + minutes(11);
    let sent_once = ReminderState {
        started: Some(STARTED),
        last_reminder: Some(last),
        ..ReminderState::default()
    };
    let events = cycle(Reminder::GoToSleep, sent_once);

    assert!(!Reminder::GoToSleep.should_remind(&events, at(last + minutes(10))));
    assert!(Reminder::GoToSleep.should_remind(&events, at(last + minutes(11))));
}

#[test]
fn a_confirmed_cycle_stops_reminding() {
    let confirmed = ReminderState {
        started: Some(STARTED),
        confirmed: Some(STARTED + minutes(5)),
        ..ReminderState::default()
    };
    let events = cycle(Reminder::GoToSleep, confirmed);

    assert!(!Reminder::GoToSleep.should_remind(&events, at(STARTED + minutes(30))));
}

#[test]
fn the_go_to_sleep_cycle_runs_out_with_its_window() {
    let events = cycle(Reminder::GoToSleep, ReminderState::started_at(STARTED));

    assert!(Reminder::GoToSleep.should_remind(&events, at(STARTED + minutes(78))));
    assert!(!Reminder::GoToSleep.should_remind(&events, at(STARTED + minutes(79))));
}

#[test]
fn the_wake_up_cycle_runs_out_with_its_window() {
    let events = cycle(Reminder::WakeUp, ReminderState::started_at(STARTED));

    assert!(Reminder::WakeUp.should_remind(&events, at(STARTED + minutes(84))));
    assert!(!Reminder::WakeUp.should_remind(&events, at(STARTED + minutes(85))));
}

#[test]
fn each_cycle_has_its_own_confirmation_word() {
    assert!(Reminder::WakeUp.is_confirmation("  Up "));
    assert!(!Reminder::WakeUp.is_confirmation("down"));

    assert!(Reminder::GoToSleep.is_confirmation(" Down "));
    assert!(!Reminder::GoToSleep.is_confirmation("going down"));
    assert!(!Reminder::GoToSleep.is_confirmation("up"));
}

fn utc_plus_three() -> PeerUtcLayers {
    PeerUtcLayers::with_offset_hours(3.0).unwrap()
}

fn anchor_of(reminder: Reminder) -> DateTime<Utc> {
    let date = "2026-09-14".parse().unwrap();

    compute_event_time(PEER, date, reminder.anchor_event(), &utc_plus_three())
}

#[test]
fn a_cycle_starts_on_its_anchor_event_without_the_peer_writing() {
    let opens = anchor_of(Reminder::WakeUp);
    let layers = utc_plus_three();
    let nothing_started = EventsState::default();

    assert!(!Reminder::WakeUp.should_start(PEER, &nothing_started, &layers, opens - Duration::minutes(1)));
    assert!(Reminder::WakeUp.should_start(PEER, &nothing_started, &layers, opens));
    assert!(Reminder::WakeUp.should_start(PEER, &nothing_started, &layers, opens + minutes_after(83)));
    assert!(!Reminder::WakeUp.should_start(PEER, &nothing_started, &layers, opens + minutes_after(85)));
}

#[test]
fn a_cycle_started_within_the_window_does_not_start_again() {
    let opens = anchor_of(Reminder::GoToSleep);
    let layers = utc_plus_three();
    let running = cycle(Reminder::GoToSleep, ReminderState::started_at(opens.timestamp()));

    assert!(!Reminder::GoToSleep.should_start(PEER, &running, &layers, opens + minutes_after(30)));

    let yesterday = cycle(Reminder::GoToSleep, ReminderState::started_at((opens - Duration::days(1)).timestamp()));
    assert!(Reminder::GoToSleep.should_start(PEER, &yesterday, &layers, opens + minutes_after(30)));
}

#[test]
fn a_started_cycle_awaits_its_confirmation() {
    let events = cycle(Reminder::GoToSleep, ReminderState::started_at(STARTED));

    assert!(Reminder::GoToSleep.is_awaiting_confirmation(&events));
    assert!(!Reminder::WakeUp.is_awaiting_confirmation(&events));
}
