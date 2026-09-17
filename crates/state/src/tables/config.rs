use chrono::FixedOffset;
use serde::{Deserialize, Serialize, Serializer};

use crate::utils::json_store::{JsonFile, JsonStore};
use crate::utils::time_units::{minutes_to_seconds, seconds_to_minutes};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PeerConfig {
    pub started: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_step: Option<String>,
    pub finished: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<TimezoneConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredTimezone")]
pub struct TimezoneConfig {
    #[serde(serialize_with = "write_utc")]
    pub utc: FixedOffset,
    pub updated_at: i64,
}

impl TimezoneConfig {
    pub fn to_numeric_utc_minutes(&self) -> i32 {
        seconds_to_minutes(self.utc.local_minus_utc())
    }
}

#[derive(Deserialize)]
struct StoredTimezone {
    utc: Option<String>,
    utc_offset_minutes: Option<i32>,
    updated_at: Option<i64>,
    set_at: Option<i64>,
}

impl TryFrom<StoredTimezone> for TimezoneConfig {
    type Error = String;

    fn try_from(stored: StoredTimezone) -> Result<Self, Self::Error> {
        let utc = match (stored.utc, stored.utc_offset_minutes) {
            (Some(text), _) => text
                .parse::<FixedOffset>()
                .map_err(|err| format!("timezone utc {text:?}: {err}"))?,
            (None, Some(minutes)) => FixedOffset::east_opt(minutes_to_seconds(minutes))
                .ok_or_else(|| format!("timezone utc_offset_minutes {minutes} is out of range"))?,
            (None, None) => return Err("timezone has no utc offset".to_string()),
        };

        Ok(Self {
            utc,
            updated_at: stored.updated_at.or(stored.set_at).unwrap_or_default(),
        })
    }
}

fn write_utc<S: Serializer>(utc: &FixedOffset, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(utc)
}

impl JsonFile for PeerConfig {
    const FILENAME: &'static str = "config.json";
}

pub type ConfigStore = JsonStore<PeerConfig>;

#[cfg(test)]
#[path = "../../tests/unit/tables/config_test.rs"]
mod tests;
