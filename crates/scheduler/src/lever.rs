use chrono::Duration;

// these two constants set the bounds of the daytime candle window
// 09:10 ───────────────────── 22:50
// EXPERIMENTAL ONLY: 2x more active, was 6 minutes
pub(crate) const CANDLES_START_AFTER_DAY_START: Duration = Duration::minutes(3);
// EXPERIMENTAL ONLY: 2x more active, was 4 minutes
pub(crate) const CANDLES_END_BEFORE_DAY_END: Duration = Duration::minutes(2);

// core most important thing - the multiplier
// EXPERIMENTAL ONLY: 2x more active, was 3.8
pub(crate) const CANDLE_FREQUENCY: f64 = 7.6;

// these two constants set the absolute bounds of a regular gap between candles
pub(crate) const CANDLE_GAP_ABSOLUTE_MIN: i64 = 3;
pub(crate) const CANDLE_GAP_ABSOLUTE_MAX: i64 = 34;

// CANDLE_GAP_MODE = 10.0 is the peak: about 10 minutes is the most likely gap.
pub(crate) const CANDLE_PEAK_MODE: f64 = 8.6;

// smaller spread → more values cluster around 10 minutes.
// larger spread → noticeably shorter and longer gaps show up more often.
pub(crate) const CANDLE_GAP_SPREAD: f64 = 0.46;

// parameters of a small extra peak for the "sent a message, then added something a couple of minutes later" case.
pub(crate) const CANDLE_SECOND_THOUGHT_AT: f64 = 2.6;
// how far the time wanders around 2.6
pub(crate) const CANDLE_SECOND_THOUGHT_WIDTH: f64 = 0.75;
pub(crate) const CANDLE_SECOND_THOUGHT_SHARE: f64 = 0.14;

// if the peer wrote to the agent, the agent must not start a new candle for the next 2 minutes.
// EXPERIMENTAL ONLY: 2x more active, was 4 minutes
pub(crate) const QUIET_AFTER_THE_PEER_WROTE: Duration = Duration::minutes(2);

pub(crate) const CANDLE_MAX_LATENESS: Duration = Duration::minutes(5);
