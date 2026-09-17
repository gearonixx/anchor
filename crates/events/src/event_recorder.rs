use state::{EventsState, SkippedEvent};

use crate::events::Event;
use crate::policy::SkipReason;
use crate::types::LocalDate;

pub struct EventRecorder;

impl EventRecorder {
    pub(crate) fn record_resolved_event(events: &mut EventsState, kind: Event, date: LocalDate) {
        events
            .resolved_events
            .insert(kind.name().to_string(), date.to_string());
    }

    pub fn record_sent_event(events: &mut EventsState, kind: Event, date: LocalDate) {
        Self::record_resolved_event(events, kind, date);
        events.events_no_reply += 1;
    }

    pub fn record_skipped_event(events: &mut EventsState, kind: Event, date: LocalDate, reason: SkipReason) {
        Self::record_resolved_event(events, kind, date);

        let skipped = SkippedEvent { date: date.to_string(), reason: reason.name().to_string() };
        events.skipped_events.insert(kind.name().to_string(), skipped);
    }
}
