use events::gen_deterministic_seed;

use super::*;

const DRAWS: usize = 100_000;
const BUCKETS: [(i64, i64); 9] =
    [(2, 3), (4, 5), (6, 8), (9, 11), (12, 15), (16, 20), (21, 27), (28, 31), (32, 36)];

fn sample() -> Vec<i64> {
    sample_at_frequency(1.0)
}

fn sample_at_frequency(frequency: f64) -> Vec<i64> {
    let shape = GapDistribution::shape_at_frequency(frequency);
    let mut random = gen_deterministic_seed(&[42, 2026, 9]);

    (0..DRAWS).map(|_| GapDistribution::draw_from(&shape, random())).collect()
}

fn mean_of(gaps: &[i64]) -> f64 {
    gaps.iter().sum::<i64>() as f64 / gaps.len() as f64
}

fn share_between(gaps: &[i64], from: i64, to: i64) -> f64 {
    gaps.iter().filter(|&&gap| gap >= from && gap <= to).count() as f64 / gaps.len() as f64
}

#[test]
fn every_drawn_gap_stays_inside_the_supported_range() {
    let gaps = sample();

    assert!(gaps.iter().all(|&gap| (CANDLE_GAP_ABSOLUTE_MIN..=CANDLE_GAP_ABSOLUTE_MAX).contains(&gap)));
    assert!(gaps.contains(&CANDLE_GAP_ABSOLUTE_MIN));
    assert!(gaps.contains(&CANDLE_GAP_ABSOLUTE_MAX));
}

#[test]
fn the_shipped_frequency_stays_inside_its_own_ends() {
    let shape = GapDistribution::probability_of_every_gap();
    let shortest = shape.first().unwrap().0;
    let longest = shape.last().unwrap().0;
    let mut random = gen_deterministic_seed(&[7, 2026, 9]);
    let gaps: Vec<i64> = (0..DRAWS).map(|_| GapDistribution::draw_minutes(random())).collect();

    assert!(shortest >= 1);
    assert!(gaps.iter().all(|&gap| (shortest..=longest).contains(&gap)));
}

#[test]
fn the_frequency_multiplier_shortens_every_gap_by_its_factor() {
    let plain = mean_of(&sample_at_frequency(1.0));

    for factor in [1.17, 1.5, 2.0] {
        let quicker = mean_of(&sample_at_frequency(factor));
        let expected = plain / factor;

        assert!(
            (quicker - expected).abs() < expected * 0.05,
            "at {factor}x the mean is {quicker:.2}, expected about {expected:.2}"
        );
    }
}

#[test]
fn the_shape_keeps_its_proportions_when_the_frequency_moves() {
    let plain = GapDistribution::shape_at_frequency(1.0);
    let doubled = GapDistribution::shape_at_frequency(2.0);
    let peak_of = |shape: &[(i64, f64)]| shape.iter().max_by(|left, right| left.1.total_cmp(&right.1)).unwrap().0;

    assert_eq!(peak_of(&plain), CANDLE_PEAK_MODE.round() as i64);
    assert_eq!(peak_of(&doubled), (CANDLE_PEAK_MODE / 2.0).round() as i64);
    assert!(doubled.first().unwrap().0 >= 1);
}

#[test]
fn the_rhythm_around_the_peak_is_the_densest_region() {
    let gaps = sample();
    let peak = CANDLE_PEAK_MODE.round() as i64;
    let around_peak = share_between(&gaps, peak - 1, peak + 1);
    let quietest_near_peak =
        (peak - 1..=peak + 1).map(|gap| share_between(&gaps, gap, gap)).fold(f64::MAX, f64::min);
    let busiest_long_gap =
        (20..=CANDLE_GAP_ABSOLUTE_MAX).map(|gap| share_between(&gaps, gap, gap)).fold(0.0, f64::max);

    assert!(around_peak > 0.18, "the peak region holds only {around_peak:.3}");
    assert!(around_peak > share_between(&gaps, CANDLE_GAP_ABSOLUTE_MIN, 5));
    assert!(
        quietest_near_peak > busiest_long_gap * 2.5,
        "the peak is not far enough above 20+: {quietest_near_peak:.4} against {busiest_long_gap:.4}"
    );
}

#[test]
fn the_densest_single_minute_sits_next_to_the_mode() {
    let gaps = sample();
    let counted = |gap: i64| gaps.iter().filter(|&&drawn| drawn == gap).count();
    let densest = (CANDLE_GAP_ABSOLUTE_MIN..=CANDLE_GAP_ABSOLUTE_MAX).max_by_key(|&gap| counted(gap)).unwrap();


    assert!((densest - CANDLE_PEAK_MODE as i64).abs() <= 1, "densest minute is {densest}");
}

#[test]
fn long_gaps_happen_but_stay_rare() {
    let gaps = sample();
    let tail = share_between(&gaps, 28, 36);

    assert!(tail > 0.002, "the long tail never happens: {tail:.4}");
    assert!(tail < 0.08, "the long tail is not rare any more: {tail:.4}");
}

#[test]
fn the_second_thought_lifts_the_shortest_gaps_above_the_tail_around_them() {
    let gaps = sample();
    let second_thought = share_between(&gaps, 2, 3);
    let trough = share_between(&gaps, 4, 4);
    let peak = CANDLE_PEAK_MODE.round() as i64;
    let main_peak = share_between(&gaps, peak - 1, peak + 1);

    assert!(second_thought > trough, "no bump: {second_thought:.4} vs {trough:.4}");
    assert!(second_thought > 0.01, "the bump never happens: {second_thought:.4}");
    assert!(second_thought * 3.0 < main_peak, "the bump rivals the main rhythm: {second_thought:.4}");
}

#[test]
fn probability_falls_away_on_both_sides_of_the_mode() {
    let every = GapDistribution::shape_at_frequency(1.0);
    let at = |gap: i64| every.iter().find(|&&(minute, _)| minute == gap).unwrap().1;

    assert!(at(6) < at(7) && at(7) < at(8) && at(8) < at(9));
    assert!(at(9) > at(10) && at(10) > at(13) && at(13) > at(18) && at(18) > at(24) && at(24) > at(30));
    assert!(at(4) < at(3));
}

#[test]
fn the_shape_is_nothing_like_a_uniform_draw() {
    let every = GapDistribution::shape_at_frequency(1.0);
    let flat = 1.0 / every.len() as f64;
    let spread: f64 = every.iter().map(|&(_, share)| (share - flat).abs()).sum();

    assert!(spread > 0.5, "the curve is almost flat: {spread:.3}");
}

#[test]
fn the_same_seed_draws_the_same_gaps_every_time() {
    assert_eq!(sample(), sample());

    let shape = GapDistribution::shape_at_frequency(1.0);
    let mut other = gen_deterministic_seed(&[43, 2026, 9]);
    let elsewhere: Vec<i64> = (0..DRAWS).map(|_| GapDistribution::draw_from(&shape, other())).collect();

    assert_ne!(sample(), elsewhere);
}

#[test]
fn the_drawn_sample_matches_the_curve_it_was_drawn_from() {
    let gaps = sample();
    let every = GapDistribution::shape_at_frequency(1.0);

    for (gap, share) in every {
        let drawn = share_between(&gaps, gap, gap);
        assert!((drawn - share).abs() < 0.006, "gap {gap}: drawn {drawn:.4} against {share:.4}");
    }
}

#[test]
fn report_the_shape_of_a_hundred_thousand_draws() {
    let mut random = gen_deterministic_seed(&[42, 2026, 9]);
    let gaps: Vec<i64> = (0..DRAWS).map(|_| GapDistribution::draw_minutes(random())).collect();
    let mean = mean_of(&gaps);
    let mut sorted = gaps.clone();
    sorted.sort_unstable();
    let at = |q: f64| sorted[(sorted.len() as f64 * q) as usize];

    println!("\n{DRAWS} draws");
    for (from, to) in BUCKETS {
        let share = share_between(&gaps, from, to);
        let bar = "#".repeat((share * 200.0) as usize);
        println!("  {from:>2}..{to:<2} {:>5.2}%  {bar}", share * 100.0);
    }
    println!(
        "  mean {mean:.2}  median {}  p10 {}  p25 {}  p75 {}  p90 {}  p99 {}",
        at(0.5), at(0.10), at(0.25), at(0.75), at(0.90), at(0.99)
    );
    println!("  candles over an 820-minute window: {:.0}", 820.0 / mean);
}

