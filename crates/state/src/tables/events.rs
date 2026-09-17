use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use crate::utils::json_store::{JsonFile, JsonStore};
use crate::utils::time_units::seconds_to_days;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EventsState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub woke_up: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub got_up: Option<i64>,
    #[serde(alias = "last_nudge", skip_serializing_if = "Option::is_none")]
    pub last_reminder: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
    #[serde(alias = "unanswered_in_a_row", alias = "events_without_reply")]
    pub events_no_reply: u32,
    #[serde(alias = "handled")]
    pub resolved_events: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub skipped_events: BTreeMap<String, SkippedEvent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fired_candle_times: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedEvent {
    pub date: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Activity {
    pub utc: [f64; 24],
    #[serde(alias = "first_seen_at")]
    pub created_at: i64,
    pub updated_at: i64,
}

impl Activity {
    pub fn total(&self) -> f64 {
        self.utc.iter().sum()
    }

    pub fn observed_days(&self, now: DateTime<Utc>) -> f64 {
        seconds_to_days(now.timestamp() - self.created_at)
    }
}

impl Serialize for Activity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut activity = serializer.serialize_struct("Activity", 5)?;

        activity.serialize_field("utc", &self.utc)?;
        activity.serialize_field("total", &self.total())?;
        activity.serialize_field("observed_days", &self.observed_days(Utc::now()))?;
        activity.serialize_field("created_at", &self.created_at)?;
        activity.serialize_field("updated_at", &self.updated_at)?;
        activity.end()
    }
}

impl JsonFile for EventsState {
    const FILENAME: &'static str = "events.json";
}

pub type EventsStore = JsonStore<EventsState>;

#[cfg(test)]
#[path = "../../tests/unit/tables/events_test.rs"]
mod tests;
