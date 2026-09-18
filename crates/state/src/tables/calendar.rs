use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::utils::json_store::{JsonFile, JsonStore};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CalendarState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<GoogleTokens>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refreshed_at: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub upcoming: Vec<CalendarEvent>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub reminded: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GoogleTokens {
    pub refresh_token: String,
    pub access_token: String,
    pub access_expires_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub starts_at: i64,
}

impl CalendarState {
    pub fn is_connected(&self) -> bool {
        self.tokens.is_some()
    }

    pub fn forget_events_before(&mut self, timestamp: i64) {
        self.reminded.retain(|_, reminded_at| *reminded_at >= timestamp);
    }
}

impl GoogleTokens {
    pub fn has_fresh_access(&self, now: i64) -> bool {
        let is_present = !self.access_token.is_empty();
        let is_unexpired = self.access_expires_at > now;

        is_present && is_unexpired
    }
}

impl JsonFile for CalendarState {
    const FILENAME: &'static str = "calendar.json";
}

pub type CalendarStore = JsonStore<CalendarState>;

#[cfg(test)]
#[path = "../../tests/unit/tables/calendar_test.rs"]
mod tests;
