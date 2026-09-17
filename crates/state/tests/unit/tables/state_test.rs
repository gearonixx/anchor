use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;
use crate::paths::UserDataPaths;

fn temp_paths() -> UserDataPaths {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);

    let dir = std::env::temp_dir().join(format!("anchor-state-test-{}-{n}", std::process::id()));
    UserDataPaths::for_user(dir, 1_311_749_836)
}

#[test]
fn missing_file_loads_as_default() {
    let paths = temp_paths();
    let state = StateStore::new(&paths).load_json();
    assert!(state.chat_active.is_none());
}

// "can the store save JSON and load it back without losses?"
#[test]
fn update_reads_back_what_it_wrote() {
    let store = StateStore::new(&temp_paths());
    store
        .update_json(|s| {
            s.chat_active = Some(true);
            s.peer_message_count = Some(3);
        })
        .unwrap();

    let state = store.load_json();
    assert_eq!(state.chat_active, Some(true));
    assert_eq!(state.peer_message_count, Some(3));
}

// when reading an old/dirty state.json, unknown fields
// are dropped on the next save.
#[test]
fn unknown_fields_are_dropped_on_save() {
    let paths = temp_paths();
    fs::create_dir_all(&paths.user_dir).unwrap();
    // write arbitrary legacy fields
    fs::write(
        &paths.state,
        r#"{
            "chat_active": false,
            "awake": false,
            "conversation_status": "active",
            "said_ledger": [{"count": 1, "sample": "hi"}],
            "interest_signals": {"peer_initiated_today": 53}
        }"#,
    )
    .unwrap();

    StateStore::new(&paths).update_json(|s| s.chat_active = Some(true)).unwrap();

    // should be cleared
    let raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&paths.state).unwrap()).unwrap();
    assert_eq!(raw["chat_active"], serde_json::json!(true));
    assert!(raw["awake"].is_null());
    assert!(raw["conversation_status"].is_null());
    assert!(raw["said_ledger"].is_null());
    assert!(raw["interest_signals"].is_null());
}

#[test]
fn a_higher_peer_message_id_is_new_and_a_replay_is_not() {
    let mut state = AgentState::default();
    assert!(state.is_incoming_message_id_new(10));

    state.last_peer_message_id = Some(10);
    assert!(!state.is_incoming_message_id_new(10));
    assert!(!state.is_incoming_message_id_new(9));
    assert!(state.is_incoming_message_id_new(11));
}

fn reaction(message_id: i64, emoji: &str, date: i64) -> PeerReaction {
    PeerReaction {
        message_id,
        emoji: emoji.to_string(),
        date,
    }
}

#[test]
fn a_replayed_reaction_is_not_new_but_the_first_one_is() {
    let mut state = AgentState::default();
    let heart = reaction(4627, "❤", 1_789_341_113);

    assert!(state.is_peer_reaction_new(&heart));
    state.add_peer_reaction(heart.clone());

    assert!(!state.is_peer_reaction_new(&heart));
}

#[test]
fn the_same_emoji_placed_again_later_is_new() {
    let mut state = AgentState::default();
    state.add_peer_reaction(reaction(4627, "❤", 1_789_341_113));

    assert!(state.is_peer_reaction_new(&reaction(4627, "❤", 1_789_341_445)));
}

#[test]
fn reactions_in_the_same_second_are_told_apart_by_message_and_emoji() {
    let mut state = AgentState::default();
    state.add_peer_reaction(reaction(4627, "❤", 1_789_341_445));

    assert!(state.is_peer_reaction_new(&reaction(4601, "❤", 1_789_341_445)));
    assert!(state.is_peer_reaction_new(&reaction(4627, "🔥", 1_789_341_445)));
}

#[test]
fn an_older_reaction_stays_known_while_newer_ones_arrive() {
    let mut state = AgentState::default();
    let first = reaction(4627, "❤", 1_789_341_113);
    state.add_peer_reaction(first.clone());

    for message_id in 4600..4610 {
        state.add_peer_reaction(reaction(message_id, "🔥", 1_789_341_445));
    }

    assert!(!state.is_peer_reaction_new(&first));
}

#[test]
fn the_remembered_reactions_do_not_grow_without_bound() {
    let mut state = AgentState::default();
    for message_id in 0..500 {
        state.add_peer_reaction(reaction(message_id, "❤", 1_789_341_445));
    }

    assert_eq!(state.recent_peer_reactions.len(), PEER_REACTIONS_LIMIT);
    assert!(state.is_peer_reaction_new(&reaction(0, "❤", 1_789_341_445)));
    assert!(!state.is_peer_reaction_new(&reaction(499, "❤", 1_789_341_445)));
}

#[test]
fn a_state_file_without_reactions_loads_and_gains_the_field() {
    let paths = temp_paths();
    fs::create_dir_all(&paths.user_dir).unwrap();
    fs::write(&paths.state, r#"{"chat_active": true}"#).unwrap();

    let store = StateStore::new(&paths);
    assert!(store.load_json().recent_peer_reactions.is_empty());

    store
        .update_json(|s| s.add_peer_reaction(reaction(4627, "❤", 1_789_341_113)))
        .unwrap();

    let state = store.load_json();
    assert_eq!(state.chat_active, Some(true));
    assert!(!state.is_peer_reaction_new(&reaction(4627, "❤", 1_789_341_113)));
}
