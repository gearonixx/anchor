use chrono::{DateTime, Duration, FixedOffset, NaiveTime, Utc};
use state::utils::time_units::{hours_to_minutes, minutes_to_millis};
#[cfg(test)]
use state::utils::time_units::{hours_to_minutes_rounded, minutes_to_seconds};
use state::{EventsState, PeerConfig};

use crate::clock::layer2::{compute_layer2_from_utc_activity, merged_rhythm_shift};
use crate::types::LocalDate;

// core_clock.rs
// knows the peer's timezone
// ↓
// knows what their local time is right now
// ↓
// can turn "09:15 for the peer" into a concrete UTC time

// "09:15 for a peer at UTC+3" = 06:15 UTC.

// the core clock logic

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerUtcLayers {
    // Yes, l1 comes from config.json
    pub layer1: FixedOffset, // the peer's timezone from config.json (L1), e.g. +03:00
    pub layer2: i32,
}

// COMPUTE
impl PeerUtcLayers {
    pub fn resolve_two_utc_layers(peer_config: &PeerConfig, events: &EventsState, now: DateTime<Utc>) -> Option<Self> {
        // "utc": "+03:00" from config.json
        let timezone = peer_config.timezone?;

        //  offset_minutes = 180
        let l1_offset_minutes = timezone.to_numeric_utc_minutes();
        // 0 or 20 or -20 - i32
        let layer2_minutes = events
            .activity
            .as_ref()
            .and_then(|activity| compute_layer2_from_utc_activity(activity, now))
            // "not computed" means rhythm_shift is never called and layer2 falls back to 0
            .map_or(0, |lived_hours| {
                // L1 from config = UTC+3
                // activity looks like UTC+5
                //
                // difference = +2 hours
                // → rhythm_shift() turns this difference into L2
                merged_rhythm_shift(l1_offset_minutes, hours_to_minutes(lived_hours))
            });

        Some(Self {
            layer1: timezone.utc,
            layer2: layer2_minutes,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_offset_hours(hours: f64) -> Option<Self> {
        let minutes = hours_to_minutes_rounded(hours);
        Some(Self {
            layer1: FixedOffset::east_opt(minutes_to_seconds(minutes))?,
            layer2: 0,
        })
    }

    // from UTC to local
    pub fn from_utc_to_local(&self, at: DateTime<Utc>) -> LocalDate {
        at.with_timezone(&self.layer1).date_naive()
    }

    // important! applies L1
    pub fn apply_layer_1(&self, date: LocalDate, minutes_after_midnight: f64) -> DateTime<Utc> {
        let utc_midnight = date.and_time(NaiveTime::MIN)
            - Duration::seconds(i64::from(self.layer1.local_minus_utc()));

        utc_midnight.and_utc()
            + Duration::milliseconds(minutes_to_millis(minutes_after_midnight).round() as i64)
    }
}


#[cfg(test)]
#[path = "../../tests/unit/clock/peer_utc_layers_test.rs"]
mod tests;
