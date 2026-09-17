use std::sync::LazyLock;

use crate::lever::{
    CANDLE_FREQUENCY, CANDLE_GAP_ABSOLUTE_MAX, CANDLE_GAP_ABSOLUTE_MIN, CANDLE_PEAK_MODE,
    CANDLE_GAP_SPREAD, CANDLE_SECOND_THOUGHT_AT, CANDLE_SECOND_THOUGHT_SHARE,
    CANDLE_SECOND_THOUGHT_WIDTH,
};

static EVERY_GAP: LazyLock<Vec<(i64, f64)>> =
    LazyLock::new(|| GapDistribution::shape_at_frequency(CANDLE_FREQUENCY));

pub struct GapDistribution;

impl GapDistribution {
    pub(crate) fn draw_minutes(uniform: f64) -> i64 {
        Self::draw_from(&EVERY_GAP, uniform)
    }

    pub fn probability_of_every_gap() -> &'static [(i64, f64)] {
        &EVERY_GAP
    }

    fn draw_from(shape: &[(i64, f64)], uniform: f64) -> i64 {
        let mut left = uniform.clamp(0.0, 1.0);

        shape
            .iter()
            .find_map(|&(gap, share)| {
                left -= share;
                (left < 0.0).then_some(gap)
            })
            .unwrap_or_else(|| shape.last().map_or(CANDLE_GAP_ABSOLUTE_MAX, |&(gap, _)| gap))
    }

    fn shape_at_frequency(frequency: f64) -> Vec<(i64, f64)> {
        let shape: Vec<(i64, f64)> = (Self::shortest_gap(frequency)..=Self::longest_gap(frequency))
            .map(|gap| (gap, Self::density(gap as f64, frequency)))
            .collect();

        let total: f64 = shape.iter().map(|&(_, weight)| weight).sum();

        shape.into_iter().map(|(gap, weight)| (gap, weight / total)).collect()
    }

    fn shortest_gap(frequency: f64) -> i64 {
        ((CANDLE_GAP_ABSOLUTE_MIN as f64 / frequency).round() as i64).max(1)
    }

    fn longest_gap(frequency: f64) -> i64 {
        ((CANDLE_GAP_ABSOLUTE_MAX as f64 / frequency).round() as i64)
            .max(Self::shortest_gap(frequency) + 1)
    }

    fn density(gap: f64, frequency: f64) -> f64 {
        Self::main_rhythm(gap, frequency) * (1.0 - CANDLE_SECOND_THOUGHT_SHARE)
            + Self::second_thought(gap, frequency) * CANDLE_SECOND_THOUGHT_SHARE
    }

    fn main_rhythm(gap: f64, frequency: f64) -> f64 {
        let spread = CANDLE_GAP_SPREAD;
        let log_centre = (CANDLE_PEAK_MODE / frequency).ln() + spread * spread;

        (-(gap.ln() - log_centre).powi(2) / (2.0 * spread * spread)).exp() / (gap * spread)
    }

    fn second_thought(gap: f64, frequency: f64) -> f64 {
        let width = CANDLE_SECOND_THOUGHT_WIDTH / frequency;
        let centre = CANDLE_SECOND_THOUGHT_AT / frequency;

        (-(gap - centre).powi(2) / (2.0 * width * width)).exp() / width
    }
}

#[cfg(test)]
#[path = "../tests/unit/gap_distribution_test.rs"]
mod tests;
