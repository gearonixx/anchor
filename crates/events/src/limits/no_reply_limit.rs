use state::EventsState;

use crate::lever::MAX_EVENTS_NO_REPLY;
use crate::utils::random::gen_deterministic_seed;

const NO_REPLY_LIMIT_SEED_PURPOSE: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoReplyLimit {
    On,
    Off,
}

impl NoReplyLimit {
    pub const LOWEST: u32 = *MAX_EVENTS_NO_REPLY.start();

    pub fn since_the_peer_wrote(events: &EventsState) -> u32 {
        let peer_wrote = events.last_message.unwrap_or_default() as u64;
        let mut random = gen_deterministic_seed(&[peer_wrote, NO_REPLY_LIMIT_SEED_PURPOSE]);

        let (lowest, highest) = (*MAX_EVENTS_NO_REPLY.start(), *MAX_EVENTS_NO_REPLY.end());
        let drawn = lowest + (random() * f64::from(highest - lowest + 1)) as u32;

        drawn.min(highest)
    }

    pub fn is_reached(self, events: &EventsState) -> bool {
        self == Self::On && events.events_no_reply >= Self::since_the_peer_wrote(events)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/limits/no_reply_limit_test.rs"]
mod tests;
