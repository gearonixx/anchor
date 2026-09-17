use super::*;

fn take(parts: &[u64], n: usize) -> Vec<f64> {
    let mut random_seed = gen_deterministic_seed(parts);
    (0..n).map(|_| random_seed()).collect()
}

#[test]
fn the_same_parts_replay_the_same_rolls() {
    assert_eq!(take(&[42, 7, 1], 16), take(&[42, 7, 1], 16));
}

#[test]
fn neighbouring_parts_diverge() {
    assert_ne!(take(&[42, 7, 1], 4), take(&[42, 8, 1], 4));
    assert_ne!(take(&[42, 7, 1], 4), take(&[43, 7, 1], 4));
    assert_ne!(take(&[42, 7, 1], 4), take(&[42, 7, 2], 4));
}

#[test]
fn rolls_stay_inside_the_unit_interval_and_cover_it() {
    let rolls = take(&[1], 10_000);
    assert!(rolls.iter().all(|r| (0.0..1.0).contains(r)));
    let mean = rolls.iter().sum::<f64>() / rolls.len() as f64;
    assert!((mean - 0.5).abs() < 0.02, "mean={mean}");
}
