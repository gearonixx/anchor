use std::time::Duration;

use state::AgentState;


// TODO: to be rewritten later

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyTimingKind {
    Instant,
    ShortDelay,
    MediumDelay,
    LongDelay,
    VeryLongDelay,
    Ignore,
}

impl ReplyTimingKind {
    pub const ALL: [Self; 6] = [
        Self::Instant,
        Self::ShortDelay,
        Self::MediumDelay,
        Self::LongDelay,
        Self::VeryLongDelay,
        Self::Ignore,
    ];

    fn baseline(self) -> f64 {
        match self {
            Self::Instant => 0.907,
            Self::ShortDelay => 0.060,
            Self::MediumDelay => 0.020,
            Self::LongDelay => 0.0073,
            Self::VeryLongDelay => 0.004,
            Self::Ignore => 0.002,
        }
    }

    fn range_ms(self) -> (u64, u64) {
        match self {
            Self::Instant => (300, 4_500),
            Self::ShortDelay => (60_000, 300_000),
            Self::MediumDelay => (300_000, 1_200_000),
            Self::LongDelay => (1_200_000, 5_400_000),
            Self::VeryLongDelay => (5_400_000, 21_600_000),
            Self::Ignore => (0, 0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReplyTiming {
    pub kind: ReplyTimingKind,
    pub delay: Duration,
}

impl ReplyTiming {
    pub fn zero() -> Self {
        Self {
            kind: ReplyTimingKind::Instant,
            delay: Duration::ZERO,
        }
    }

    pub fn will_reply(&self) -> bool {
        self.kind != ReplyTimingKind::Ignore
    }
}



// logic for the randomized reply timing

// TODO: this function is unused
pub fn compute_reply_timing(_state: &AgentState, rng: &mut impl FnMut() -> f64) -> ReplyTiming {
    let weights: Vec<f64> = ReplyTimingKind::ALL.iter().map(|k| k.baseline()).collect();
    let kind = draw(&weights, rng());

    ReplyTiming {
        kind,
        delay: sample_delay(kind, rng()),
    }
}

fn draw(weights: &[f64], roll: f64) -> ReplyTimingKind {
    let total: f64 = weights.iter().sum();
    let mut cursor = roll * total;

    for (kind, weight) in ReplyTimingKind::ALL.iter().zip(weights) {
        cursor -= weight;
        if cursor <= 0.0 {
            return *kind;
        }
    }
    ReplyTimingKind::Instant
}

fn sample_delay(kind: ReplyTimingKind, roll: f64) -> Duration {
    let (min, max) = kind.range_ms();
    Duration::from_millis(min + ((max - min) as f64 * roll) as u64)
}

#[cfg(test)]
#[path = "../tests/unit/reply_timing_test.rs"]
mod tests;
