use chrono::{DateTime, Datelike, Duration, Utc};
use events::{Event, LocalDate, NoReplyLimit, PeerUtcLayers, compute_event_time, gen_deterministic_seed};
use state::EventsState;

use crate::gap_distribution::GapDistribution;
use crate::lever::{
    CANDLE_MAX_LATENESS, CANDLES_END_BEFORE_DAY_END, CANDLES_START_AFTER_DAY_START,
    QUIET_AFTER_THE_PEER_WROTE,
};

const CANDLE_SEED_PURPOSE: u64 = 2;
const FIRED_CANDLE_HISTORY: Duration = Duration::days(2);
const CANDLE_NAME_PREFIX: &str = "candle_";

const CANDLE_TEXTS: [&str; 10] = [
    "hey",
    "what are you up to?",
    "how is it going?",
    "what are you doing?",
    "anything new?",
    "how is your day going?",
    "are you there?",
    "what are you thinking about?",
    "tell me something",
    "got a minute?",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleSkipReason {
    ConversationWasGoing { peer_wrote: DateTime<Utc> },
    IgnoredTooManyTimes { unanswered: u32, limit: u32 },
    NotKnownSinceThePeerWrote { peer_wrote: DateTime<Utc> },
}

impl CandleSkipReason {
    pub fn name(self) -> &'static str {
        match self {
            Self::ConversationWasGoing { .. } => "conversation_was_going",
            Self::IgnoredTooManyTimes { .. } => "ignored_too_many_times",
            Self::NotKnownSinceThePeerWrote { .. } => "not_known_since_the_peer_wrote",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleStatus {
    Planned,
    Fired,
    Skipped(CandleSkipReason),
    Missed,
}

impl CandleStatus {
    pub fn name(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Fired => "fired",
            Self::Skipped(_) => "skipped",
            Self::Missed => "missed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candle {
    pub index: usize,
    pub date: LocalDate,
    pub time: DateTime<Utc>,
    pub slot_ends: DateTime<Utc>,
    pub text: &'static str,
}

impl Candle {
    pub fn plan_day(peer_id: i64, date: LocalDate, layers: &PeerUtcLayers) -> Vec<Self> {
        let mut random =
            gen_deterministic_seed(&[peer_id as u64, date.num_days_from_ce() as u64, CANDLE_SEED_PURPOSE]);

        let window_start = compute_event_time(peer_id, date, Event::DayStart, layers) + CANDLES_START_AFTER_DAY_START;
        let window_end = compute_event_time(peer_id, date, Event::DayEnd, layers) - CANDLES_END_BEFORE_DAY_END;

        let mut plan: Vec<Self> = Vec::new();
        let mut time = window_start;

        loop {
            time += Duration::minutes(GapDistribution::draw_minutes(random()));
            if time > window_end { break; }

            let text = CANDLE_TEXTS[(random() * CANDLE_TEXTS.len() as f64) as usize];
            let slot_ends = time + CANDLE_MAX_LATENESS;

            plan.push(Self { index: plan.len(), date, time, slot_ends, text });
        }

        Self::without_overlapping_slots(plan)
    }

    fn without_overlapping_slots(mut plan: Vec<Self>) -> Vec<Self> {
        for index in 1..plan.len() {
            let next_candle_lights = plan[index].time;
            plan[index - 1].slot_ends = plan[index - 1].slot_ends.min(next_candle_lights);
        }

        plan
    }

    pub fn due_now(
        peer_id: i64,
        events: &EventsState,
        layers: &PeerUtcLayers,
        now: DateTime<Utc>,
        no_reply_limit: NoReplyLimit,
    ) -> Option<Self> {
        let is_conversation_going =
            events.last_message.is_some_and(|last| now.timestamp() - last < QUIET_AFTER_THE_PEER_WROTE.num_seconds());
        let is_ignored = no_reply_limit.is_reached(events);

        if is_conversation_going || is_ignored { return None; }

        let today = layers.from_utc_to_local(now);
        let yesterday = today.pred_opt().unwrap_or(today);

        [yesterday, today]
            .into_iter()
            .flat_map(|date| Self::plan_day(peer_id, date, layers))
            .find(|candle| now >= candle.time && candle.status(events, now, no_reply_limit) == CandleStatus::Planned)
    }

    pub fn status(&self, events: &EventsState, now: DateTime<Utc>, no_reply_limit: NoReplyLimit) -> CandleStatus {
        let is_fired = events
            .fired_candle_times
            .iter()
            .filter_map(|&fired| DateTime::from_timestamp(fired, 0))
            .any(|fired| self.is_own_firing_time(fired));

        if is_fired { return CandleStatus::Fired; }
        if now < self.slot_ends { return CandleStatus::Planned; }

        self.what_blocked_it(events, no_reply_limit).map_or(CandleStatus::Missed, CandleStatus::Skipped)
    }

    fn what_blocked_it(&self, events: &EventsState, no_reply_limit: NoReplyLimit) -> Option<CandleSkipReason> {
        let peer_wrote = events.last_message.and_then(|written| DateTime::from_timestamp(written, 0));
        let last_chance = self.slot_ends;

        match peer_wrote {
            Some(wrote) if wrote >= self.time - QUIET_AFTER_THE_PEER_WROTE && wrote <= last_chance => {
                Some(CandleSkipReason::ConversationWasGoing { peer_wrote: wrote })
            }
            _ if no_reply_limit == NoReplyLimit::Off => None,
            Some(wrote) if wrote > last_chance => self
                .over_the_limit_while_the_agent_kept_quiet(events, wrote)
                .or(Some(CandleSkipReason::NotKnownSinceThePeerWrote { peer_wrote: wrote })),
            _ => self.over_the_limit_since_the_peer_wrote(events),
        }
    }

    fn over_the_limit_while_the_agent_kept_quiet(
        &self,
        events: &EventsState,
        peer_wrote: DateTime<Utc>,
    ) -> Option<CandleSkipReason> {
        let slot = self.time.timestamp();
        let spoke_again = events
            .fired_candle_times
            .iter()
            .any(|&fired| fired > slot && fired < peer_wrote.timestamp());

        if spoke_again { return None; }

        let fired_before = events.fired_candle_times.iter().filter(|&&fired| fired < slot).count() as u32;

        let limit = NoReplyLimit::LOWEST;

        (fired_before >= limit).then_some(CandleSkipReason::IgnoredTooManyTimes { unanswered: limit, limit })
    }

    fn over_the_limit_since_the_peer_wrote(&self, events: &EventsState) -> Option<CandleSkipReason> {
        let peer_wrote = events.last_message.unwrap_or(i64::MIN);
        let slot = self.time.timestamp();

        let unanswered = events
            .fired_candle_times
            .iter()
            .filter(|&&fired| fired > peer_wrote && fired < slot)
            .count() as u32;

        let limit = NoReplyLimit::since_the_peer_wrote(events);

        (unanswered >= limit).then_some(CandleSkipReason::IgnoredTooManyTimes { unanswered, limit })
    }

    pub fn record_fired(&self, events: &mut EventsState, now: DateTime<Utc>) {
        events.resolved_events.retain(|event, _| !event.starts_with(CANDLE_NAME_PREFIX));
        events.events_no_reply += 1;

        events.fired_candle_times.retain(|&time| now.timestamp() - time < FIRED_CANDLE_HISTORY.num_seconds());
        events.fired_candle_times.push(now.timestamp());
    }

    pub fn is_own_firing_time(&self, time: DateTime<Utc>) -> bool {
        time >= self.time && time < self.slot_ends
    }

    pub fn fired_times_inside_plan(plan: &[Self], events: &EventsState) -> Vec<DateTime<Utc>> {
        plan.first()
            .zip(plan.last())
            .map(|(first, last)| {
                let window = first.time..last.slot_ends;

                events
                    .fired_candle_times
                    .iter()
                    .filter_map(|&time| DateTime::from_timestamp(time, 0))
                    .filter(|time| window.contains(time))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn name(&self) -> String {
        format!("{CANDLE_NAME_PREFIX}{}", self.index)
    }
}

#[cfg(test)]
#[path = "../tests/unit/candle_test.rs"]
mod tests;
