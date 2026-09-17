use chrono::{DateTime, Utc};
use state::PeerConfig;

pub(crate) enum Answer {
    Accepted(String),
    Rejected(Option<&'static str>),
}

pub(crate) trait Step: Sync {
    fn name(&self) -> &'static str;
    fn heading(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn check_answer(&self, text: &str, peer_config: &mut PeerConfig, now: DateTime<Utc>) -> Answer;
}
