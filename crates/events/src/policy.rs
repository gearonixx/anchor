
// whether the agent should send an event or not

use chrono::{DateTime, Utc};
use state::EventsState;

use crate::timing::event_time::{PlannedEvent, apply_final_event_time};
use crate::types::LocalDate;
use crate::events::Event;
use crate::lever::MAX_EVENT_LATENESS;
use crate::limits::no_reply_limit::NoReplyLimit;
use crate::clock::peer_utc_layers::PeerUtcLayers;
use crate::utils::duration::parse_str_to_minutes;

pub(crate) const MAX_EVENT_LATENESS_DURATION: chrono::Duration =
    chrono::Duration::minutes(parse_str_to_minutes(MAX_EVENT_LATENESS) as i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    SkippedByChance,
    IgnoredTooManyTimes,
}

impl SkipReason {
    const ALL: [Self; 2] = [Self::SkippedByChance, Self::IgnoredTooManyTimes];

    pub fn name(self) -> &'static str {
        match self {
            Self::SkippedByChance => "skipped_by_chance",
            Self::IgnoredTooManyTimes => "ignored_too_many_times",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|reason| reason.name() == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    SendEvent {
        kind: Event,
        date: LocalDate,
        text: &'static str,
    },
    SetAsSkipped {
        kind: Event,
        date: LocalDate,
        reason: SkipReason,
    },
}

pub fn compute_decision(
    peer_id: i64,
    events: &EventsState,
    layers: &PeerUtcLayers,
    now: DateTime<Utc>,
    forced_probabilities: bool,
    no_reply_limit: NoReplyLimit,
) -> Vec<Decision> {
    Event::ALL
        .into_iter()
        .filter_map(|kind| {
            compute_event_decision(peer_id, kind, events, layers, now, forced_probabilities, no_reply_limit)
        })
        .collect()
}

fn compute_event_decision(
    peer_id: i64,
    // DayStart or DayEnd
    kind: Event,
    events: &EventsState,
    layers: &PeerUtcLayers,
    now: DateTime<Utc>,
    forced_probabilities: bool,
    no_reply_limit: NoReplyLimit,
) -> Option<Decision> {
    let (layer1_event_date, layer1_layer2_planned_event) =
        find_pending_event(peer_id, kind, events, layers, now, forced_probabilities)?;

    create_decision(
        &layer1_layer2_planned_event,
        layer1_event_date,
        events,
        now,
        no_reply_limit,
    )
}

pub(crate) fn find_pending_event(
    peer_id: i64,
    kind: Event,
    events: &EventsState,
    layers: &PeerUtcLayers,
    now: DateTime<Utc>,
    forced_probabilities: bool,
) -> Option<(LocalDate, PlannedEvent)> {
    // L1: local calendar date from the config.json timezone
    // layer1_today          → today by the peer's clock
    // layer1_yesterday      → yesterday
    // layer1_start_of_today → when the peer's 00:00 today was in UTC

    // needed so a `DayEnd` that lands after midnight is not lost.
    // that is why the code looks at both `today` and `yesterday`, and `start_of_today` filters out
    // everything that definitely belongs to the past.
    let layer1_today = layers.from_utc_to_local(now);
    let layer1_yesterday = layer1_today.pred_opt().unwrap_or(layer1_today);

    // L1: start of today's local day, converted to UTC
    let layer1_start_of_today =
        layers.apply_layer_1(layer1_today, 0.0);

    // yesterday
    // today
    // DayEnd for yesterday
    // DayEnd for today
    [layer1_yesterday, layer1_today]
        .into_iter()
        .filter(|&layer1_event_date| {
            let resolved_date = last_resolved_date(events, kind);
            resolved_date.is_none_or(|resolved| resolved < layer1_event_date)
        })
        .map(|layer1_event_date| {
            (
                layer1_event_date,

                // BOTH layers are used here:
                // L1 = timezone
                // L2 = rhythm shift
                // CORE computation here
                apply_final_event_time(
                    peer_id,
                    layer1_event_date,
                    kind,
                    layers,
                    forced_probabilities,
                ),
            )
        })
        // just a filter: "which of these two events makes sense to take right now?"
        .find(|(_, layer1_layer2_planned_event)| {
            let is_since_midnight = layer1_layer2_planned_event.time >= layer1_start_of_today;
            let is_expired = now - layer1_layer2_planned_event.time > MAX_EVENT_LATENESS_DURATION;
            let needs_skip_record = !layer1_layer2_planned_event.should_send;
            is_since_midnight && (!is_expired || needs_skip_record)
        })
}

pub(crate) fn create_decision(
    planned: &PlannedEvent,
    date: LocalDate,
    events: &EventsState,
    now: DateTime<Utc>,
    no_reply_limit: NoReplyLimit,
) -> Option<Decision> {
    let kind = planned.kind;
    let skip = |reason| Some(Decision::SetAsSkipped { kind, date, reason });

    if !planned.should_send { return skip(SkipReason::SkippedByChance); }

    let is_expired = now - planned.time > MAX_EVENT_LATENESS_DURATION;
    if now < planned.time || is_expired { return None; }

    if no_reply_limit.is_reached(events) {
        return skip(SkipReason::IgnoredTooManyTimes);
    }

    Some(Decision::SendEvent {
        kind,
        date,
        text: planned.text,
    })
}

pub(crate) fn last_resolved_date(events: &EventsState, kind: Event) -> Option<LocalDate> {
    events.resolved_events.get(kind.name())?.parse().ok()
}

#[cfg(test)]
#[path = "../tests/unit/policy_test.rs"]
mod tests;
