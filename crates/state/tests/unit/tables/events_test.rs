use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;
use crate::paths::UserDataPaths;

fn temp_paths() -> UserDataPaths {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("anchor-events-test-{}-{n}", std::process::id()));
    UserDataPaths::for_user(dir, 1_311_749_836)
}

#[test]
fn missing_file_loads_as_default() {
    assert_eq!(EventsStore::new(&temp_paths()).load_json(), EventsState::default());
}

#[test]
fn update_reads_back_what_it_wrote_and_leaves_state_json_alone() {
    let paths = temp_paths();
    let store = EventsStore::new(&paths);
    store
        .update_json(|e| {
            e.last_message = Some(1_789_083_952);
            e.events_no_reply = 2;
            e.resolved_events.insert("day_start".into(), "2026-09-11".into());
            e.skipped_events.insert(
                "day_end".into(),
                SkippedEvent { date: "2026-09-11".into(), reason: "ignored_too_many_times".into() },
            );
        })
        .unwrap();

    let events = store.load_json();
    assert_eq!(events.last_message, Some(1_789_083_952));
    assert_eq!(events.events_no_reply, 2);
    assert_eq!(events.resolved_events["day_start"], "2026-09-11");
    assert_eq!(events.skipped_events["day_end"].reason, "ignored_too_many_times");
    assert!(!paths.state.exists());
}

#[test]
fn a_partial_file_fills_the_gaps_with_defaults() {
    let paths = temp_paths();
    let store = EventsStore::new(&paths);
    fs::create_dir_all(&paths.user_dir).unwrap();
    fs::write(paths.user_dir.join(EventsState::FILENAME), r#"{"events_no_reply": 1}"#).unwrap();
    let events = store.load_json();
    assert_eq!(events.events_no_reply, 1);
    assert!(events.resolved_events.is_empty());
    assert!(events.skipped_events.is_empty());
    assert!(events.activity.is_none());
}

#[test]
fn an_old_unanswered_counter_is_read_and_saved_under_the_new_name() {
    let paths = temp_paths();
    let file = paths.user_dir.join(EventsState::FILENAME);
    fs::create_dir_all(&paths.user_dir).unwrap();
    fs::write(&file, r#"{"unanswered_in_a_row": 2}"#).unwrap();

    let store = EventsStore::new(&paths);
    assert_eq!(store.load_json().events_no_reply, 2);

    store.update_json(|e| e.events_no_reply = 3).unwrap();

    let saved = fs::read_to_string(&file).unwrap();
    assert!(saved.contains("\"events_no_reply\": 3"), "{saved}");
    assert!(!saved.contains("unanswered_in_a_row"), "{saved}");
}

#[test]
fn an_old_events_without_reply_counter_is_read_and_saved_under_the_new_name() {
    let paths = temp_paths();
    let file = paths.user_dir.join(EventsState::FILENAME);
    fs::create_dir_all(&paths.user_dir).unwrap();
    fs::write(&file, r#"{"events_without_reply": 2}"#).unwrap();

    let store = EventsStore::new(&paths);
    assert_eq!(store.load_json().events_no_reply, 2);

    store.update_json(|e| e.events_no_reply = 3).unwrap();

    let saved = fs::read_to_string(&file).unwrap();
    assert!(saved.contains("\"events_no_reply\": 3"), "{saved}");
    assert!(!saved.contains("events_without_reply"), "{saved}");
}

#[test]
fn an_old_handled_map_is_read_as_resolved_events_and_saved_under_the_new_name() {
    let paths = temp_paths();
    let file = paths.user_dir.join(EventsState::FILENAME);
    fs::create_dir_all(&paths.user_dir).unwrap();
    fs::write(&file, r#"{"handled": {"day_end": "2026-09-12"}}"#).unwrap();

    let store = EventsStore::new(&paths);
    assert_eq!(store.load_json().resolved_events["day_end"], "2026-09-12");

    store.update_json(|e| e.events_no_reply = 1).unwrap();

    let saved = fs::read_to_string(&file).unwrap();
    assert!(saved.contains("\"resolved_events\""), "{saved}");
    assert!(!saved.contains("\"handled\""), "{saved}");
}

#[test]
fn an_old_first_seen_at_is_read_as_created_at_and_saved_under_the_new_name() {
    let paths = temp_paths();
    let file = paths.user_dir.join(EventsState::FILENAME);
    fs::create_dir_all(&paths.user_dir).unwrap();
    fs::write(
        &file,
        r#"{"activity": {"utc": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,0,0,0,0], "first_seen_at": 1789242109, "updated_at": 1789242110}}"#,
    )
    .unwrap();

    let store = EventsStore::new(&paths);
    assert_eq!(store.load_json().activity.unwrap().created_at, 1_789_242_109);

    store.update_json(|e| e.events_no_reply = 1).unwrap();
    let saved = fs::read_to_string(&file).unwrap();
    assert!(saved.contains("\"created_at\": 1789242109"), "{saved}");
    assert!(!saved.contains("first_seen_at"), "{saved}");
}

#[test]
fn the_written_total_is_always_the_sum_of_the_utc_bins() {
    let mut activity = Activity::default();
    activity.utc[11] = 0.975_723_633_126_923;
    activity.utc[14] = 6.867_789_535_201_007;
    activity.utc[23] = 4.999_593_755_156_860_6;

    let written: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&activity).unwrap()).unwrap();

    assert_eq!(written["total"], serde_json::json!(activity.total()));
    assert!((activity.total() - 12.843_106_923_484_79).abs() < 1e-9);
}

#[test]
fn the_written_observed_days_count_from_created_at_to_the_moment_of_writing() {
    let day_and_a_half = 36 * 60 * 60;
    let activity = Activity { created_at: Utc::now().timestamp() - day_and_a_half, ..Activity::default() };

    let before = activity.observed_days(Utc::now());
    let written: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&activity).unwrap()).unwrap();
    let after = activity.observed_days(Utc::now());

    let observed = written["observed_days"].as_f64().unwrap();
    assert!((before..=after).contains(&observed), "{observed} outside {before}..={after}");
    assert!((observed - 1.5).abs() < 0.001, "{observed}");
}

#[test]
fn a_stale_total_in_an_old_file_is_recomputed_not_trusted() {
    let stored = r#"{
        "utc": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2.5],
        "total": 999.0,
        "created_at": 1789242109,
        "updated_at": 1789341459
    }"#;

    let activity: Activity = serde_json::from_str(stored).unwrap();
    assert_eq!(activity.total(), 2.5);

    let written: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&activity).unwrap()).unwrap();
    assert_eq!(written["total"], serde_json::json!(2.5));
}
