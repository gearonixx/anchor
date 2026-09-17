use std::ops::RangeInclusive;

use crate::timing::time_window::TimeWindow;

// initial windows constants
pub(crate) const DAY_START_WINDOW: TimeWindow = TimeWindow::new("09:00", "10:00"); // 1 hour long
pub(crate) const DAY_END_WINDOW: TimeWindow = TimeWindow::new("23:00", "01:00"); // 2 hours long

// the real L2 is computed from activity and ranges from −105 to +105 minutes
pub(crate) const L2_RADIUS: &str = "1h45m";

// MINIMUM: the computation needs at least 30.0 UTC activity and at least 2 days of observations.
pub(crate) const L2_MIN_TOTAL_UTC_MESSAGES: u32 = 30;
pub(crate) const L2_MIN_OBSERVED_DAYS: &str = "2d";

// important
pub(crate) const L2_QUIETEST_WINDOW_HOURS: usize = 6;
// 10
pub(crate) const L2_MAX_ACTIVITY_WHILE_QUIET: f64 = 0.1;

// see policy.rs
// once the peer has ignored more than 7 or 8, the agent stops sending events
pub(crate) const MAX_EVENTS_NO_REPLY: RangeInclusive<u32> = 7..=8;

// L2 dead zone: while the gap between the config.json timezone and the inferred
// rhythm stays within it, the gap is treated as noise and the event does not move at all.
// only the part beyond these 40 minutes shifts the event, and only by half.
// the larger this value, the less L2 matters
pub(crate) const L2_NOISE_MINUTES: i32 = 40;
// which share of the gap beyond the dead zone is actually applied.
pub(crate) const L2_SHIFT_SHARE: f64 = 0.5; // → take half of the remainder

// how long after the planned minute the agent may still send the event.
// if this window is missed, the greeting is not sent that day at all; nothing resends it later.
pub(crate) const MAX_EVENT_LATENESS: &str = "9m";

// how long the agent stays available after the day end event
pub(crate) const FALLS_ASLEEP_AFTER_DAY_END: &str = "14m";

pub(crate) const NUDGE_EVERY: &str = "20m";
