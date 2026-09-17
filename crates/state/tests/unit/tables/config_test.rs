use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;
use crate::paths::UserDataPaths;
use crate::utils::time_units::minutes_to_seconds;

const UTC_PLUS_THREE_MINUTES: i32 = 180;

fn temp_paths() -> UserDataPaths {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("anchor-peer-config-test-{}-{n}", std::process::id()));
    UserDataPaths::for_user(dir, 1_311_749_836)
}

#[test]
fn a_new_peer_has_no_timezone_yet() {
    assert_eq!(ConfigStore::new(&temp_paths()).load_json().timezone, None);
}

#[test]
fn a_saved_timezone_reads_back_and_leaves_other_files_alone() {
    let paths = temp_paths();
    let store = ConfigStore::new(&paths);
    let utc_plus_three = TimezoneConfig {
        utc: FixedOffset::east_opt(minutes_to_seconds(UTC_PLUS_THREE_MINUTES)).unwrap(),
        updated_at: 1_789_142_757,
    };
    store.update_json(|s| s.timezone = Some(utc_plus_three)).unwrap();

    assert_eq!(store.load_json().timezone, Some(utc_plus_three));
    assert!(!paths.state.exists());
}

#[test]
fn peer_config_is_written_to_config_json() {
    let paths = temp_paths();
    ConfigStore::new(&paths).update_json(|s| s.started = true).unwrap();
    assert!(paths.user_dir.join("config.json").exists());
}

#[test]
fn a_timezone_is_written_as_utc_text_with_updated_at() {
    let paths = temp_paths();
    let store = ConfigStore::new(&paths);
    store
        .update_json(|s| {
            s.timezone = Some(TimezoneConfig {
                utc: FixedOffset::east_opt(minutes_to_seconds(330)).unwrap(),
                updated_at: 1_789_237_998,
            })
        })
        .unwrap();

    let raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(paths.user_dir.join("config.json")).unwrap()).unwrap();
    assert_eq!(
        raw["timezone"],
        serde_json::json!({"utc": "+05:30", "updated_at": 1_789_237_998})
    );
    assert_eq!(store.load_json().timezone.unwrap().utc.to_string(), "+05:30");
}

#[test]
fn a_timezone_saved_in_the_old_minutes_format_still_loads() {
    let old: TimezoneConfig =
        serde_json::from_str(r#"{"utc_offset_minutes": -300, "set_at": 1789142757}"#).unwrap();
    assert_eq!(old.utc.to_string(), "-05:00");
    assert_eq!(old.updated_at, 1_789_142_757);
}

#[test]
fn a_timezone_without_an_offset_is_rejected() {
    assert!(serde_json::from_str::<TimezoneConfig>(r#"{"updated_at": 1}"#).is_err());
    assert!(serde_json::from_str::<TimezoneConfig>(r#"{"utc": "three"}"#).is_err());
}

#[test]
fn an_offset_reads_back_as_minutes_east_of_utc() {
    let utc_plus_three = TimezoneConfig {
        utc: FixedOffset::east_opt(minutes_to_seconds(UTC_PLUS_THREE_MINUTES)).unwrap(),
        updated_at: 1_789_142_757,
    };
    assert_eq!(utc_plus_three.to_numeric_utc_minutes(), UTC_PLUS_THREE_MINUTES);

    let half_hour_zone = TimezoneConfig {
        utc: FixedOffset::east_opt(minutes_to_seconds(330)).unwrap(),
        updated_at: 1_789_142_757,
    };
    assert_eq!(half_hour_zone.to_numeric_utc_minutes(), 330);

    let west_of_utc = TimezoneConfig {
        utc: FixedOffset::east_opt(minutes_to_seconds(-300)).unwrap(),
        updated_at: 1_789_142_757,
    };
    assert_eq!(west_of_utc.to_numeric_utc_minutes(), -300);
}
