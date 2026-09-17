use super::*;

// predictable fake random for tests.
// fixed() does not produce bounds; it yields predefined random values in 0..1,
// e.g. fixed(vec![0.2, 0.8]) gives 0.2, 0.8, 0.2, 0.8, ...
// it is a stand-in for the random generator, not a generator itself.
// .cycle() is what makes the values repeat:
fn fixed(values: Vec<f64>) -> impl FnMut() -> f64 {
    let mut it = values.into_iter().cycle();
    move || it.next().unwrap()
}

#[test]
fn a_low_roll_lands_on_instant() {
    let timing = compute_reply_timing(&AgentState::default(), &mut fixed(vec![0.0, 0.0]));
    assert_eq!(timing.kind, ReplyTimingKind::Instant);
    assert_eq!(timing.delay, Duration::from_millis(300));
}

#[test]
fn the_top_of_the_mass_lands_on_ignore() {
    // delay will be Duration::ZERO
    let timing = compute_reply_timing(&AgentState::default(), &mut fixed(vec![1.0, 0.0]));
    // but the delay does not matter here: Ignore means no reply at all.
    assert_eq!(timing.kind, ReplyTimingKind::Ignore);
}

#[test]
fn every_kind_samples_inside_its_own_range() {
    for kind in ReplyTimingKind::ALL {
        let (min, max) = kind.range_ms();
        assert_eq!(sample_delay(kind, 0.0), Duration::from_millis(min));
        assert_eq!(sample_delay(kind, 1.0), Duration::from_millis(max));
    }
}
