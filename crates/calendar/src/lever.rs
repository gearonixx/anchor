// how long before an event starts the agent sends the heads-up
pub(crate) const HEADS_UP_BEFORE: &str = "10m";

// how often the upcoming events are pulled from google
pub(crate) const REFRESH_CALENDAR_EVERY: &str = "15m";

// how far ahead the upcoming events are pulled
pub(crate) const LOOK_AHEAD: &str = "1d";

// how long a sent heads-up is remembered, so that it is not sent twice
pub(crate) const REMEMBER_SENT_FOR: &str = "2d";
