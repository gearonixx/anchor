use chrono::{DateTime, Utc};
use state::{PeerConfig, TimezoneConfig};

use crate::spec::{Answer, Step};
use crate::utils::utc_offset::{OffsetError, parse_utc_offset};

const DESCRIPTION: &str = "Please send your current timezone. The bot will use this to determine your local time.\n\nExamples: +3, 0, -5, +5:30";
const TIMEZONE_UPDATED: &str = "Your UTC offset is now {offset}.";
const INVALID_UTC_OFFSET: &str = "<b>Invalid UTC offset.</b>";

pub(crate) struct Timezone;

impl Step for Timezone {
    fn name(&self) -> &'static str {
        "timezone"
    }

    fn heading(&self) -> &'static str {
        "Time zone"
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn check_answer(&self, text: &str, peer_config: &mut PeerConfig, now: DateTime<Utc>) -> Answer {
        match parse_utc_offset(text) {
            Ok(offset) => {
                peer_config.timezone = Some(TimezoneConfig {
                    utc: offset,
                    updated_at: now.timestamp(),
                });
                Answer::Accepted(TIMEZONE_UPDATED.replace("{offset}", &format!("UTC{offset}")))
            }
            Err(OffsetError::NoSuchZone) => Answer::Rejected(Some(INVALID_UTC_OFFSET)),
            Err(OffsetError::NotAnOffset) => Answer::Rejected(None),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/steps/timezone_test.rs"]
mod tests;
