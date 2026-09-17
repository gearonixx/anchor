use chrono::Duration;

use super::*;
use crate::EventRecorder;

const PEER: i64 = 1_000_000_001;

fn utc_plus_three() -> PeerUtcLayers {
    PeerUtcLayers::with_offset_hours(3.0).unwrap()
}

fn first_date_where_sending_is(kind: Event, is_sending: bool) -> (LocalDate, PlannedEvent) {
    let first: LocalDate = "2026-09-01".parse().unwrap();
    (0..120)
        .map(|offset| first + Duration::days(offset))
        .map(|date| {
            (date, apply_final_event_time(PEER, date, kind, &utc_plus_three(), false))
        })
        .find(|(_, planned)| planned.should_send == is_sending)
        .unwrap()
}

fn decisions_for_kind(
    kind: Event,
    events: &EventsState,
    now: DateTime<Utc>,
) -> Vec<Decision> {
    compute_decision(PEER, events, &utc_plus_three(), now, false, NoReplyLimit::On)
        .into_iter()
        .filter(|decision| match decision {
            Decision::SendEvent { kind: decided, .. } | Decision::SetAsSkipped { kind: decided, .. } => {
                *decided == kind
            }
        })
        .collect()
}

fn expected_send(kind: Event, date: LocalDate, planned: &PlannedEvent) -> Vec<Decision> {
    vec![Decision::SendEvent {
        kind,
        date,
        text: planned.text,
    }]
}

fn expected_skip(kind: Event, date: LocalDate, reason: SkipReason) -> Vec<Decision> {
    vec![Decision::SetAsSkipped { kind, date, reason }]
}

#[test]
fn nothing_happens_before_the_planned_minute() {
    for kind in Event::ALL {
        let (_, planned) = first_date_where_sending_is(kind, true);
        let just_before = planned.time - Duration::seconds(1);
        assert!(decisions_for_kind(kind, &EventsState::default(), just_before).is_empty());
    }
}

#[test]
fn the_message_goes_out_when_its_minute_comes() {
    for kind in Event::ALL {
        let (date, planned) = first_date_where_sending_is(kind, true);
        assert_eq!(
            decisions_for_kind(kind, &EventsState::default(), planned.time),
            expected_send(kind, date, &planned)
        );
    }
}

#[test]
fn greetings_are_sent_only_within_the_configured_lateness() {
    let date: LocalDate = "2026-09-13".parse().unwrap();
    let planned_time = "2026-09-13T05:30:00Z".parse().unwrap();
    let max_delay_seconds = MAX_EVENT_LATENESS_DURATION.num_seconds();

    for kind in Event::ALL {
        let planned = PlannedEvent {
            kind,
            time: planned_time,
            should_send: true,
            text: kind.text(),
        };

        for (delay_seconds, is_expected_to_send) in [
            (-1, false),
            (0, true),
            (max_delay_seconds / 2, true),
            (max_delay_seconds, true),
            (max_delay_seconds + 1, false),
        ] {
            let now = planned_time + Duration::seconds(delay_seconds);
            let expected = is_expected_to_send.then_some(Decision::SendEvent {
                kind,
                date,
                text: kind.text(),
            });
            assert_eq!(
                create_decision(&planned, date, &EventsState::default(), now, NoReplyLimit::On),
                expected,
                "kind={kind:?} delay_seconds={delay_seconds}",
            );
        }
    }
}

#[test]
fn a_resolved_date_is_never_repeated() {
    for kind in Event::ALL {
        let (date, planned) = first_date_where_sending_is(kind, true);
        let mut events = EventsState::default();
        EventRecorder::record_sent_event(&mut events, kind, date);
        let decisions = decisions_for_kind(kind, &events, planned.time);
        assert!(
            decisions.iter().all(|decision| match decision {
                Decision::SendEvent { date: decided, .. }
                | Decision::SetAsSkipped { date: decided, .. } => *decided > date,
            }),
            "kind={kind:?} resolved_date={date} decisions={decisions:?}"
        );
    }
}

#[test]
fn yesterdays_mark_does_not_block_today() {
    let (date, planned) = first_date_where_sending_is(Event::DayStart, true);
    let mut events = EventsState::default();
    EventRecorder::record_resolved_event(&mut events, Event::DayStart, date.pred_opt().unwrap());
    assert_eq!(
        decisions_for_kind(Event::DayStart, &events, planned.time),
        expected_send(Event::DayStart, date, &planned)
    );
}

#[test]
fn the_agent_stops_greeting_after_three_events_no_reply() {
    let (date, planned) = first_date_where_sending_is(Event::DayEnd, true);
    let mut events = EventsState::default();
    for _ in 0..NoReplyLimit::since_the_peer_wrote(&events) - 1 {
        EventRecorder::record_sent_event(&mut events, Event::DayStart, date.pred_opt().unwrap());
    }
    assert_eq!(
        decisions_for_kind(Event::DayEnd, &events, planned.time),
        expected_send(Event::DayEnd, date, &planned)
    );

    EventRecorder::record_sent_event(&mut events, Event::DayStart, date.pred_opt().unwrap());
    assert_eq!(
        decisions_for_kind(Event::DayEnd, &events, planned.time),
        expected_skip(Event::DayEnd, date, SkipReason::IgnoredTooManyTimes)
    );
}

#[test]
fn the_agent_keeps_greeting_past_the_limit_when_it_is_off() {
    let (date, planned) = first_date_where_sending_is(Event::DayEnd, true);
    let events = EventsState {
        events_no_reply: NoReplyLimit::since_the_peer_wrote(&EventsState::default()),
        ..EventsState::default()
    };

    assert_eq!(
        create_decision(&planned, date, &events, planned.time, NoReplyLimit::Off),
        Some(Decision::SendEvent { kind: Event::DayEnd, date, text: planned.text })
    );
}

#[test]
fn a_day_off_is_marked_at_midnight_once_yesterday_is_resolved() {
    for kind in Event::ALL {
        let (date, _) = first_date_where_sending_is(kind, false);
        let start_of_date = utc_plus_three().apply_layer_1(date, 0.0);
        let mut events = EventsState::default();
        EventRecorder::record_resolved_event(&mut events, kind, date.pred_opt().unwrap());

        assert_eq!(
            decisions_for_kind(kind, &events, start_of_date),
            expected_skip(kind, date, SkipReason::SkippedByChance)
        );
    }
}

fn first_day_end_after_midnight() -> (LocalDate, PlannedEvent) {
    let first: LocalDate = "2026-09-01".parse().unwrap();
    (0..120)
        .map(|offset| first + Duration::days(offset))
        .map(|date| {
            let planned =
                apply_final_event_time(PEER, date, Event::DayEnd, &utc_plus_three(), false);
            (date, planned)
        })
        .find(|(date, planned)| {
            let next_midnight =
                utc_plus_three().apply_layer_1(*date, 0.0) + Duration::days(1);
            planned.should_send && planned.time >= next_midnight
        })
        .unwrap()
}

#[test]
fn a_day_end_past_midnight_still_belongs_to_its_own_date() {
    let (date, planned) = first_day_end_after_midnight();
    assert_eq!(utc_plus_three().from_utc_to_local(planned.time), date.succ_opt().unwrap());
    assert_eq!(
        decisions_for_kind(Event::DayEnd, &EventsState::default(), planned.time),
        expected_send(Event::DayEnd, date, &planned)
    );
}

#[test]
fn a_greeting_left_over_from_yesterday_is_never_sent_today() {
    let (date, planned) = first_date_where_sending_is(Event::DayStart, true);
    let tomorrow = planned.time + Duration::days(1);
    assert!(decisions_for_kind(Event::DayStart, &EventsState::default(), tomorrow)
        .into_iter()
        .all(|decision| match decision {
            Decision::SendEvent { date: decided, .. } | Decision::SetAsSkipped { date: decided, .. } =>
                decided != date,
        }));
}

fn greetings_sent_by_ticking_every_minute(first: LocalDate, days: i64) -> Vec<(Event, LocalDate)> {
    let first_minute = utc_plus_three().apply_layer_1(first, 0.0);
    let mut events = EventsState::default();
    let mut sent = Vec::new();

    for minute in 0..Duration::days(days).num_minutes() {
        let now = first_minute + Duration::minutes(minute);
        for decision in compute_decision(PEER, &events, &utc_plus_three(), now, false, NoReplyLimit::On) {
            match decision {
                Decision::SendEvent { kind, date, .. } => {
                    EventRecorder::record_sent_event(&mut events, kind, date);
                    events.events_no_reply = 0;
                    sent.push((kind, date));
                }
                Decision::SetAsSkipped { kind, date, reason } => {
                    EventRecorder::record_skipped_event(&mut events, kind, date, reason);
                }
            }
        }
    }

    sent
}

fn greetings_planned_to_go_out(first: LocalDate, days: i64) -> Vec<(Event, LocalDate)> {
    let ticked = utc_plus_three().apply_layer_1(first, 0.0)
        ..utc_plus_three().apply_layer_1(first + Duration::days(days), 0.0);

    (-1..days)
        .map(|offset| first + Duration::days(offset))
        .flat_map(|date| Event::ALL.map(|kind| (kind, date)))
        .filter(|&(kind, date)| {
            let planned = apply_final_event_time(PEER, date, kind, &utc_plus_three(), false);
            planned.should_send && ticked.contains(&planned.time)
        })
        .collect()
}

#[test]
fn every_planned_greeting_goes_out_exactly_once() {
    let first: LocalDate = "2026-09-01".parse().unwrap();
    let by_date = |&(kind, date): &(Event, LocalDate)| (date, kind.name());

    let mut sent = greetings_sent_by_ticking_every_minute(first, 60);
    let mut planned = greetings_planned_to_go_out(first, 60);
    sent.sort_by_key(by_date);
    planned.sort_by_key(by_date);

    assert_eq!(sent, planned);
}
