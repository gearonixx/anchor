use std::collections::BTreeMap;

use super::*;

const STARTED: i64 = 1_000_000;

fn at(timestamp: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp, 0).unwrap()
}

fn minutes(count: i64) -> i64 {
    count * 60
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
fn the_wake_up_cycle_has_no_window_and_reminds_until_it_is_confirmed() {
    let events = cycle(Reminder::WakeUp, ReminderState::started_at(STARTED));

    assert!(Reminder::WakeUp.should_remind(&events, at(STARTED + minutes(600))));
}

#[test]
fn each_cycle_has_its_own_confirmation_word() {
    assert!(Reminder::WakeUp.is_confirmation("  Up "));
    assert!(!Reminder::WakeUp.is_confirmation("down"));

    assert!(Reminder::GoToSleep.is_confirmation(" Down "));
    assert!(!Reminder::GoToSleep.is_confirmation("going down"));
    assert!(!Reminder::GoToSleep.is_confirmation("up"));
}

#[test]
fn a_started_cycle_awaits_its_confirmation() {
    let events = cycle(Reminder::GoToSleep, ReminderState::started_at(STARTED));

    assert!(Reminder::GoToSleep.is_awaiting_confirmation(&events));
    assert!(!Reminder::WakeUp.is_awaiting_confirmation(&events));
}
