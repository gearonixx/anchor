use chrono::Duration;

use super::*;
use crate::EventRecorder;
use crate::lever::MAX_EVENTS_NO_REPLY;
use crate::policy::{MAX_EVENT_LATENESS_DURATION, compute_decision};

const PEER: i64 = 1_000_000_001;

fn utc_plus_three() -> PeerUtcLayers {
    PeerUtcLayers::with_offset_hours(3.0).unwrap()
}

fn first_date_where_sending_is(kind: Event, is_sending: bool) -> (LocalDate, PlannedEvent) {
    let first: LocalDate = "2026-09-01".parse().unwrap();
    (0..120)
        .map(|offset| first + Duration::days(offset))
        .map(|date| (date, apply_final_event_time(PEER, date, kind, &utc_plus_three(), false)))
        .find(|(_, planned)| planned.should_send == is_sending)
        .unwrap()
}

fn first_day_end_after_midnight() -> (LocalDate, PlannedEvent) {
    let first: LocalDate = "2026-09-01".parse().unwrap();
    (0..120)
        .map(|offset| first + Duration::days(offset))
        .map(|date| (date, apply_final_event_time(PEER, date, Event::DayEnd, &utc_plus_three(), false)))
        .find(|(date, planned)| {
            let next_midnight = utc_plus_three().apply_layer_1(*date, 0.0) + Duration::days(1);
            planned.should_send && planned.time >= next_midnight
        })
        .unwrap()
}

fn forecast_for(kind: Event, events: &EventsState, now: DateTime<Utc>) -> EventForecast {
    EventForecast::for_today(PEER, events, &utc_plus_three(), now, false, NoReplyLimit::On)
        .into_iter()
        .find(|forecast| forecast.kind == kind)
        .unwrap()
}

fn record_like_the_bot(events: &mut EventsState, decision: Decision) {
    match decision {
        Decision::SendEvent { kind, date, .. } => EventRecorder::record_sent_event(events, kind, date),
        Decision::SetAsSkipped { kind, date, reason } => {
            EventRecorder::record_skipped_event(events, kind, date, reason);
        }
    }
}

fn act_like_the_bot(events: &mut EventsState, now: DateTime<Utc>) {
    for decision in compute_decision(PEER, events, &utc_plus_three(), now, false, NoReplyLimit::On) {
        record_like_the_bot(events, decision);
    }
}

#[test]
fn a_greeting_is_planned_before_its_minute_and_sent_once_the_bot_sends_it() {
    let (date, planned) = first_date_where_sending_is(Event::DayStart, true);
    let mut events = EventsState::default();

    let before = forecast_for(Event::DayStart, &events, planned.time - Duration::minutes(1));
    act_like_the_bot(&mut events, planned.time);
    let after = forecast_for(Event::DayStart, &events, planned.time + Duration::minutes(1));

    assert_eq!((before.date, before.time, before.status), (date, planned.time, EventStatus::Planned));
    assert_eq!((after.date, after.status), (date, EventStatus::Fired));
}

#[test]
fn a_greeting_skipped_for_being_ignored_too_many_times_is_not_shown_as_sent() {
    let (date, planned) = first_date_where_sending_is(Event::DayStart, true);
    let mut events = EventsState { events_no_reply: *MAX_EVENTS_NO_REPLY.end(), ..EventsState::default() };

    act_like_the_bot(&mut events, planned.time);
    let after = forecast_for(Event::DayStart, &events, planned.time + Duration::minutes(1));

    assert_eq!((after.date, after.status), (date, EventStatus::Skipped(SkipReason::IgnoredTooManyTimes)));
}

#[test]
fn a_greeting_nobody_sent_within_its_window_is_shown_as_missed() {
    let (date, planned) = first_date_where_sending_is(Event::DayStart, true);
    let too_late = planned.time + MAX_EVENT_LATENESS_DURATION + Duration::minutes(1);

    let forecast = forecast_for(Event::DayStart, &EventsState::default(), too_late);

    assert_eq!((forecast.date, forecast.status), (date, EventStatus::Missed));
}

#[test]
fn a_day_off_is_shown_as_skipped_by_chance_before_and_after_the_bot_records_it() {
    let (date, _) = first_date_where_sending_is(Event::DayStart, false);
    let start_of_date = utc_plus_three().apply_layer_1(date, 0.0);
    let mut events = EventsState::default();

    let before = forecast_for(Event::DayStart, &events, start_of_date);
    act_like_the_bot(&mut events, start_of_date);
    let after = forecast_for(Event::DayStart, &events, start_of_date + Duration::minutes(1));

    let skipped_by_chance = EventStatus::Skipped(SkipReason::SkippedByChance);
    assert_eq!((before.date, before.status), (date, skipped_by_chance));
    assert_eq!((after.date, after.status), (date, skipped_by_chance));
}

#[test]
fn yesterdays_day_end_after_midnight_is_shown_until_the_bot_sends_it() {
    let (date, planned) = first_day_end_after_midnight();
    let mut events = EventsState::default();

    let before = forecast_for(Event::DayEnd, &events, planned.time - Duration::minutes(1));
    act_like_the_bot(&mut events, planned.time);
    let after = forecast_for(Event::DayEnd, &events, planned.time + Duration::minutes(1));

    assert_eq!((before.date, before.time, before.status), (date, planned.time, EventStatus::Planned));
    assert_eq!(after.date, date.succ_opt().unwrap());
}

#[test]
fn the_forecast_shows_every_decision_the_bot_is_about_to_make() {
    let first_minute = utc_plus_three().apply_layer_1("2026-09-01".parse().unwrap(), 0.0);
    let mut events = EventsState::default();

    for minute in 0..Duration::days(30).num_minutes() {
        let now = first_minute + Duration::minutes(minute);
        let decisions = compute_decision(PEER, &events, &utc_plus_three(), now, false, NoReplyLimit::On);
        let forecasts = EventForecast::for_today(PEER, &events, &utc_plus_three(), now, false, NoReplyLimit::On);

        for decision in &decisions {
            let (kind, date, expected) = match *decision {
                Decision::SendEvent { kind, date, .. } => (kind, date, EventStatus::Planned),
                Decision::SetAsSkipped { kind, date, reason } => (kind, date, EventStatus::Skipped(reason)),
            };
            let forecast = forecasts.iter().find(|forecast| forecast.kind == kind).unwrap();
            assert_eq!((forecast.date, forecast.status), (date, expected), "{decision:?} at {now}");
        }

        decisions.into_iter().for_each(|decision| record_like_the_bot(&mut events, decision));
    }
}

#[test]
fn status_names_are_the_ones_the_dashboard_reads() {
    let names = [
        EventStatus::Planned,
        EventStatus::Fired,
        EventStatus::Skipped(SkipReason::SkippedByChance),
        EventStatus::Skipped(SkipReason::IgnoredTooManyTimes),
        EventStatus::Missed,
    ]
    .map(EventStatus::name);

    assert_eq!(names, ["planned", "fired", "skipped_by_chance", "ignored_too_many_times", "missed"]);
}
