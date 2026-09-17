use chrono::{DateTime, Utc};
use state::EventsState;

use crate::clock::peer_utc_layers::PeerUtcLayers;
use crate::limits::no_reply_limit::NoReplyLimit;
use crate::timing::event_time::{PlannedEvent, apply_final_event_time};
use crate::events::Event;
use crate::policy::{Decision, SkipReason, create_decision, find_pending_event, last_resolved_date};
use crate::types::LocalDate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventStatus {
    Planned,
    Fired,
    Skipped(SkipReason),
    Missed,
}

impl EventStatus {
    pub fn name(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Fired => "fired",
            Self::Skipped(reason) => reason.name(),
            Self::Missed => "missed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventForecast {
    pub kind: Event,
    pub date: LocalDate,
    pub time: DateTime<Utc>,
    pub text: &'static str,
    pub status: EventStatus,
}

impl EventForecast {
    pub fn for_today(
        peer_id: i64,
        events: &EventsState,
        layers: &PeerUtcLayers,
        now: DateTime<Utc>,
        forced_probabilities: bool,
        no_reply_limit: NoReplyLimit,
    ) -> Vec<Self> {
        Event::ALL
            .into_iter()
            .map(|kind| Self::for_kind(peer_id, kind, events, layers, now, forced_probabilities, no_reply_limit))
            .collect()
    }

    pub fn for_date(
        peer_id: i64,
        date: LocalDate,
        events: &EventsState,
        layers: &PeerUtcLayers,
        now: DateTime<Utc>,
        forced_probabilities: bool,
        no_reply_limit: NoReplyLimit,
    ) -> Vec<Self> {
        if date == layers.from_utc_to_local(now) {
            return Self::for_today(peer_id, events, layers, now, forced_probabilities, no_reply_limit);
        }

        Event::ALL
            .into_iter()
            .map(|kind| {
                let planned = apply_final_event_time(peer_id, date, kind, layers, forced_probabilities);
                Self::from_records(date, &planned, events)
            })
            .collect()
    }

    fn for_kind(
        peer_id: i64,
        kind: Event,
        events: &EventsState,
        layers: &PeerUtcLayers,
        now: DateTime<Utc>,
        forced_probabilities: bool,
        no_reply_limit: NoReplyLimit,
    ) -> Self {
        match find_pending_event(peer_id, kind, events, layers, now, forced_probabilities) {
            Some((date, planned)) => Self::from_decision(date, &planned, events, now, no_reply_limit),
            None => {
                let today = layers.from_utc_to_local(now);
                let planned = apply_final_event_time(peer_id, today, kind, layers, forced_probabilities);
                Self::from_records(today, &planned, events)
            }
        }
    }

    fn from_decision(
        date: LocalDate,
        planned: &PlannedEvent,
        events: &EventsState,
        now: DateTime<Utc>,
        no_reply_limit: NoReplyLimit,
    ) -> Self {
        let status = match create_decision(planned, date, events, now, no_reply_limit) {
            Some(Decision::SetAsSkipped { reason, .. }) => EventStatus::Skipped(reason),
            Some(Decision::SendEvent { .. }) | None => EventStatus::Planned,
        };

        Self::with_status(date, planned, status)
    }

    fn from_records(date: LocalDate, planned: &PlannedEvent, events: &EventsState) -> Self {
        let is_resolved = last_resolved_date(events, planned.kind).is_some_and(|resolved| resolved >= date);

        let status = match Self::recorded_skip_reason(events, planned.kind, date) {
            Some(reason) => EventStatus::Skipped(reason),
            None if !planned.should_send => EventStatus::Skipped(SkipReason::SkippedByChance),
            None if is_resolved => EventStatus::Fired,
            None => EventStatus::Missed,
        };

        Self::with_status(date, planned, status)
    }

    fn recorded_skip_reason(events: &EventsState, kind: Event, date: LocalDate) -> Option<SkipReason> {
        events
            .skipped_events
            .get(kind.name())
            .filter(|skipped| skipped.date == date.to_string())
            .and_then(|skipped| SkipReason::from_name(&skipped.reason))
    }

    fn with_status(date: LocalDate, planned: &PlannedEvent, status: EventStatus) -> Self {
        Self {
            kind: planned.kind,
            date,
            time: planned.time,
            text: planned.text,
            status,
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/event_forecast_test.rs"]
mod tests;
