use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::json_store::{JsonFile, JsonStore};

const PEER_REACTIONS_LIMIT: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sender {
    Peer,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerReaction {
    pub message_id: i64,
    pub emoji: String,
    pub date: i64,
}

impl PeerReaction {
    pub fn new(message_id: i32, emoji: &str, sent_at: DateTime<Utc>) -> Self {
        Self {
            message_id: i64::from(message_id),
            emoji: emoji.to_owned(),
            date: sent_at.timestamp(),
        }
    }
}

// TODO: other fields in the old port:
// awake, phone_proximity, openness_to_chat, currently_doing, last_wake_reset_date,
// peer_timezone_offset, reply_pending, reply_fire_at, reply_timing, proactive_counter_date,
// proactive_messages_today
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tg_message_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<Sender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_message_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_ignored_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_peer_message_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_peer_reactions: Vec<PeerReaction>,
}

impl AgentState {
    pub fn is_incoming_message_id_new(&self, incoming_msg_id: i64) -> bool {
        self.last_peer_message_id.is_none_or(|last_id| incoming_msg_id > last_id)
    }

    // whether a reaction with the same message_id, emoji and date already exists
    pub fn is_peer_reaction_new(&self, reaction: &PeerReaction) -> bool {
        !self.recent_peer_reactions.contains(reaction)
    }

    pub fn add_peer_reaction(&mut self, reaction: PeerReaction) {
        self.recent_peer_reactions.push(reaction);

        let len = self.recent_peer_reactions.len();

        if len > PEER_REACTIONS_LIMIT {
            self.recent_peer_reactions
                .drain(..len - PEER_REACTIONS_LIMIT);
        }
    }
}

impl JsonFile for AgentState {
    const FILENAME: &'static str = "state.json";
}

pub type StateStore = JsonStore<AgentState>;

#[cfg(test)]
#[path = "../../tests/unit/tables/state_test.rs"]
mod tests;
