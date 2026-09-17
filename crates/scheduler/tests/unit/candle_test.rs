use chrono::FixedOffset;

use super::*;

const PEER: i64 = 1_000_000_001;

fn utc_plus_three() -> PeerUtcLayers {
    PeerUtcLayers { layer1: FixedOffset::east_opt(3 * 3600).unwrap(), layer2: 0 }
}

fn date() -> LocalDate {
    "2026-09-14".parse().unwrap()
}

fn plan() -> Vec<Candle> {
    Candle::plan_day(PEER, date(), &utc_plus_three())
}

fn plan_of_the_next_day() -> Vec<Candle> {
    Candle::plan_day(PEER, date().succ_opt().unwrap(), &utc_plus_three())
}

fn due(events: &EventsState, now: DateTime<Utc>) -> Option<Candle> {
    Candle::due_now(PEER, events, &utc_plus_three(), now, NoReplyLimit::On)
}

fn limit_before_the_peer_ever_wrote() -> u32 {
    NoReplyLimit::since_the_peer_wrote(&EventsState::default())
}

fn fired_minutes_before(candle: &Candle, count: u32) -> Vec<i64> {
    (1..=i64::from(count)).map(|minutes| (candle.time - Duration::minutes(minutes)).timestamp()).collect()
}

#[test]
fn a_day_of_candles_fills_the_window_between_the_greetings() {
    let day_start = compute_event_time(PEER, date(), Event::DayStart, &utc_plus_three());
    let day_end = compute_event_time(PEER, date(), Event::DayEnd, &utc_plus_three());

    let candles = plan();
    let window_start = day_start + CANDLES_START_AFTER_DAY_START;
    let window_end = day_end - CANDLES_END_BEFORE_DAY_END;
    let longest_gap = Duration::minutes(GapDistribution::probability_of_every_gap().last().unwrap().0);

    assert!(candles.iter().all(|candle| candle.time > window_start && candle.time <= window_end));
    assert!(candles[0].time - window_start <= longest_gap);
    assert!(window_end - candles.last().unwrap().time <= longest_gap);
}

#[test]
fn candles_stay_inside_the_drawn_range_but_are_not_evenly_spaced() {
    let gaps: Vec<Duration> = plan().windows(2).map(|pair| pair[1].time - pair[0].time).collect();

    let drawn = GapDistribution::probability_of_every_gap();

    assert!(gaps.iter().all(|&gap| gap >= Duration::minutes(drawn.first().unwrap().0)));
    assert!(gaps.iter().all(|&gap| gap <= Duration::minutes(drawn.last().unwrap().0)));
    assert!(gaps.iter().any(|&gap| gap != gaps[0]));
}

#[test]
fn a_day_carries_both_the_quick_second_thoughts_and_the_long_silences() {
    let week: Vec<Duration> = (0..7)
        .flat_map(|day| {
            let date = (0..day).fold(date(), |walked, _| walked.succ_opt().unwrap());
            let candles = Candle::plan_day(PEER, date, &utc_plus_three());

            candles.windows(2).map(|pair| pair[1].time - pair[0].time).collect::<Vec<_>>()
        })
        .collect();

    let every_gap = GapDistribution::probability_of_every_gap();
    let shortest = Duration::minutes(every_gap.first().unwrap().0);
    let longest = Duration::minutes(every_gap.last().unwrap().0);
    let densest = Duration::minutes(every_gap.iter().max_by(|left, right| left.1.total_cmp(&right.1)).unwrap().0);
    let near_densest = |gap: Duration| (gap - densest).num_minutes().abs() <= 1;

    assert!(week.contains(&shortest));
    assert!(week.contains(&longest));
    assert!(week.iter().filter(|&&gap| near_densest(gap)).count() * 4 > week.len());
}

#[test]
fn the_same_day_is_planned_the_same_way_and_the_next_day_differently() {
    let next_day = Candle::plan_day(PEER, date().succ_opt().unwrap(), &utc_plus_three());
    let next_day_times: Vec<_> = next_day.iter().map(|candle| candle.time - Duration::days(1)).collect();

    assert_eq!(plan(), plan());
    assert_ne!(plan().iter().map(|candle| candle.time).collect::<Vec<_>>(), next_day_times);
}

#[test]
fn a_candle_is_due_from_its_minute_until_its_slot_runs_out() {
    let candle = plan()[3];
    let events = EventsState::default();

    assert_ne!(due(&events, candle.time - Duration::seconds(1)), Some(candle));
    assert_eq!(due(&events, candle.time), Some(candle));
    assert_eq!(due(&events, candle.slot_ends - Duration::seconds(1)), Some(candle));
    assert_ne!(due(&events, candle.slot_ends), Some(candle));
}

#[test]
fn a_fired_candle_is_never_due_again() {
    let candle = plan()[3];
    let mut events = EventsState::default();

    candle.record_fired(&mut events, candle.time);

    assert_eq!(due(&events, candle.time), None);
    assert_eq!(candle.status(&events, candle.time, NoReplyLimit::On), CandleStatus::Fired);
}

#[test]
fn a_fired_candle_keeps_the_time_it_went_out_even_if_the_plan_changes() {
    let candle = plan()[3];
    let went_out = candle.time + Duration::minutes(1);
    let mut events = EventsState::default();

    candle.record_fired(&mut events, went_out);

    assert_eq!(Candle::fired_times_inside_plan(&plan(), &events), vec![went_out]);
    assert!(Candle::fired_times_inside_plan(&plan_of_the_next_day(), &events).is_empty());
}

#[test]
fn fired_times_older_than_two_days_are_forgotten() {
    let candle = plan()[3];
    let mut events = EventsState::default();

    candle.record_fired(&mut events, candle.time - Duration::days(3));
    candle.record_fired(&mut events, candle.time);

    assert_eq!(events.fired_candle_times, vec![candle.time.timestamp()]);
}

#[test]
fn the_agent_does_not_start_a_conversation_right_after_the_peer_wrote() {
    let candle = plan()[3];
    let events = EventsState { date: Some((candle.time - QUIET_AFTER_THE_PEER_WROTE / 2).timestamp()), ..EventsState::default() };

    assert_eq!(due(&events, candle.time), None);
}

#[test]
fn the_agent_goes_quiet_after_too_many_messages_without_reply() {
    let candle = plan()[3];
    let events = EventsState { events_no_reply: limit_before_the_peer_ever_wrote(), ..EventsState::default() };

    assert_eq!(due(&events, candle.time), None);
}

#[test]
fn the_agent_still_writes_one_message_short_of_the_limit() {
    let candle = plan()[3];
    let events = EventsState { events_no_reply: limit_before_the_peer_ever_wrote() - 1, ..EventsState::default() };

    assert_eq!(due(&events, candle.time), Some(candle));
}

#[test]
fn the_agent_keeps_writing_past_the_limit_when_it_is_off() {
    let candle = plan()[3];
    let events = EventsState { events_no_reply: limit_before_the_peer_ever_wrote() * 10, ..EventsState::default() };

    assert_eq!(Candle::due_now(PEER, &events, &utc_plus_three(), candle.time, NoReplyLimit::Off), Some(candle));
}

#[test]
fn a_candle_nothing_blocked_is_only_missed() {
    let candle = plan()[3];
    let too_late = candle.slot_ends;

    assert_eq!(candle.status(&EventsState::default(), candle.time, NoReplyLimit::On), CandleStatus::Planned);
    assert_eq!(candle.status(&EventsState::default(), too_late, NoReplyLimit::On), CandleStatus::Missed);
}

#[test]
fn a_candle_the_peer_talked_over_says_when_the_peer_wrote() {
    let candle = plan()[3];
    let peer_wrote = candle.time - QUIET_AFTER_THE_PEER_WROTE / 2;
    let too_late = candle.slot_ends;
    let events = EventsState { date: Some(peer_wrote.timestamp()), ..EventsState::default() };

    assert_eq!(
        candle.status(&events, too_late, NoReplyLimit::On),
        CandleStatus::Skipped(CandleSkipReason::ConversationWasGoing { peer_wrote })
    );
}

#[test]
fn a_candle_over_the_unanswered_limit_counts_the_messages_before_it() {
    let candle = plan()[3];
    let too_late = candle.slot_ends;

    let limit = limit_before_the_peer_ever_wrote();
    let events = EventsState { fired_candle_times: fired_minutes_before(&candle, limit), ..EventsState::default() };

    assert_eq!(
        candle.status(&events, too_late, NoReplyLimit::On),
        CandleStatus::Skipped(CandleSkipReason::IgnoredTooManyTimes { unanswered: limit, limit })
    );
}

#[test]
fn with_the_limit_off_a_candle_after_many_unanswered_is_only_missed() {
    let candle = plan()[3];
    let too_late = candle.slot_ends;

    let events = EventsState {
        fired_candle_times: fired_minutes_before(&candle, limit_before_the_peer_ever_wrote()),
        ..EventsState::default()
    };

    assert_eq!(candle.status(&events, too_late, NoReplyLimit::Off), CandleStatus::Missed);
}

#[test]
fn messages_the_peer_already_answered_do_not_count_as_ignored() {
    let candle = plan()[3];
    let too_late = candle.slot_ends;

    let fired_candle_times = (1..=i64::from(NoReplyLimit::LOWEST))
        .map(|hours| (candle.time - Duration::hours(hours)).timestamp())
        .collect();
    let events = EventsState {
        date: Some((candle.time - Duration::minutes(30)).timestamp()),
        fired_candle_times,
        ..EventsState::default()
    };

    assert_eq!(candle.status(&events, too_late, NoReplyLimit::On), CandleStatus::Missed);
}

#[test]
fn a_candle_the_peer_answered_only_later_is_still_read_from_the_agent_silence() {
    let candle = plan()[3];
    let peer_wrote = candle.time + Duration::hours(1);
    let too_late = candle.slot_ends;

    let limit = NoReplyLimit::LOWEST;
    let events = EventsState {
        date: Some(peer_wrote.timestamp()),
        fired_candle_times: fired_minutes_before(&candle, limit),
        ..EventsState::default()
    };

    assert_eq!(
        candle.status(&events, too_late, NoReplyLimit::On),
        CandleStatus::Skipped(CandleSkipReason::IgnoredTooManyTimes { unanswered: limit, limit })
    );
}

#[test]
fn a_candle_the_agent_went_on_writing_after_loses_its_reason_to_the_reply() {
    let candle = plan()[3];
    let peer_wrote = candle.time + Duration::hours(1);
    let too_late = candle.slot_ends;

    let mut fired_candle_times = fired_minutes_before(&candle, NoReplyLimit::LOWEST);
    fired_candle_times.push((candle.time + Duration::minutes(30)).timestamp());

    let events = EventsState {
        date: Some(peer_wrote.timestamp()),
        fired_candle_times,
        ..EventsState::default()
    };

    assert_eq!(
        candle.status(&events, too_late, NoReplyLimit::On),
        CandleStatus::Skipped(CandleSkipReason::NotKnownSinceThePeerWrote { peer_wrote })
    );
}

#[test]
fn a_candle_owns_the_minutes_from_its_slot_until_the_next_one_takes_over() {
    let candle = plan()[0];

    assert!(candle.is_own_firing_time(candle.time));
    assert!(candle.is_own_firing_time(candle.slot_ends - Duration::seconds(1)));
    assert!(!candle.is_own_firing_time(candle.time - Duration::seconds(1)));
    assert!(!candle.is_own_firing_time(candle.slot_ends));
    assert!(candle.slot_ends - candle.time <= CANDLE_MAX_LATENESS);
}

#[test]
fn no_two_candles_of_a_day_claim_the_same_firing_time() {
    let plan = plan();

    for candle in &plan {
        let owners = plan.iter().filter(|other| other.is_own_firing_time(candle.time)).count();
        assert_eq!(owners, 1);
    }
}
