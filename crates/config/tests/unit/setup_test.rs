use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;

const PEER: i64 = 1_000_000_001;

fn temp_paths() -> UserDataPaths {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("anchor-config-test-{}-{n}", std::process::id()));
    UserDataPaths::for_user(dir, PEER)
}

fn configuration() -> (Configuration, ConfigStore) {
    let paths = temp_paths();
    (
        Configuration::for_user(&paths.data_dir, PEER),
        ConfigStore::new(&paths),
    )
}

fn now() -> DateTime<Utc> {
    "2026-09-12T15:00:00Z".parse().unwrap()
}

fn reply(configuration: &Configuration, input: Input<'_>) -> String {
    match configuration.create_step(input, now()).unwrap() {
        ConfigurationStep::Reply(text) => text,
        ConfigurationStep::Finished => panic!("expected a reply, the peer is configured"),
        ConfigurationStep::Skip => panic!("expected a reply, got silence"),
    }
}

fn invited_and_consented(configuration: &Configuration) {
    reply(configuration, Input::Text("hello"));
    reply(configuration, Input::Text("yes"));
}

#[test]
fn a_new_peer_is_invited_and_gets_a_config_file() {
    let (configuration, store) = configuration();
    assert!(!store.exists());
    assert_eq!(reply(&configuration, Input::Text("hello")), SETUP_REQUIRED);
    assert!(store.exists());
    assert_eq!(store.load_json(), PeerConfig::default());
    assert!(!configuration.is_finished());
}

#[test]
fn without_consent_the_invitation_is_repeated() {
    let (configuration, store) = configuration();
    reply(&configuration, Input::Text("hello"));
    assert_eq!(reply(&configuration, Input::Text("+3")), SETUP_REQUIRED);
    assert_eq!(reply(&configuration, Input::Other), SETUP_REQUIRED);
    assert!(!store.load_json().started);
    assert_eq!(store.load_json().timezone, None);
}

#[test]
fn consent_opens_the_first_step() {
    let (configuration, store) = configuration();
    reply(&configuration, Input::Text("hello"));
    let prompt = reply(&configuration, Input::Text("Yes!"));
    assert!(prompt.starts_with("<b>Time zone.</b>"), "{prompt}");

    let peer_config = store.load_json();
    assert!(peer_config.started);
    assert_eq!(peer_config.current_step.as_deref(), Some("timezone"));
    assert!(!peer_config.finished);
}

#[test]
fn a_valid_timezone_completes_the_configuration() {
    let (configuration, store) = configuration();
    invited_and_consented(&configuration);

    let done = reply(&configuration, Input::Text("+3"));
    assert!(done.contains("UTC+03:00") && done.ends_with(FINISHED), "{done}");

    let peer_config = store.load_json();
    assert_eq!(peer_config.current_step.as_deref(), Some(DONE_STEP));
    assert!(peer_config.finished);
    assert_eq!(
        peer_config.timezone.map(|timezone| timezone.utc.to_string()),
        Some("+03:00".to_string())
    );
    assert!(configuration.is_finished());
    assert!(matches!(
        configuration.create_step(Input::Text("hello"), now()).unwrap(),
        ConfigurationStep::Finished
    ));
}

#[test]
fn a_wrong_answer_keeps_the_peer_on_the_same_step() {
    let (configuration, store) = configuration();
    invited_and_consented(&configuration);

    let explained = reply(&configuration, Input::Text("+15"));
    assert!(explained.starts_with("<b>Invalid UTC offset.</b>"), "{explained}");
    let repeated = reply(&configuration, Input::Text("not sure"));
    assert!(repeated.starts_with("<b>Time zone.</b>"), "{repeated}");
    let after_sticker = reply(&configuration, Input::Other);
    assert!(after_sticker.starts_with("<b>Time zone.</b>"), "{after_sticker}");

    assert_eq!(store.load_json().current_step.as_deref(), Some("timezone"));
    assert!(!configuration.is_finished());
}

#[test]
fn a_reaction_during_configuration_gets_no_reply() {
    let (configuration, _) = configuration();
    reply(&configuration, Input::Text("hello"));
    assert!(matches!(
        configuration.create_step(Input::Reaction, now()).unwrap(),
        ConfigurationStep::Skip
    ));
}

#[test]
fn a_config_file_from_before_the_configuration_flow_is_invited_again() {
    let paths = temp_paths();
    fs::create_dir_all(&paths.user_dir).unwrap();
    fs::write(
        paths.user_dir.join("config.json"),
        r#"{"timezone": {"utc_offset_minutes": 180, "set_at": 1789142757}}"#,
    )
    .unwrap();
    let configuration = Configuration::for_user(&paths.data_dir, PEER);

    assert!(!configuration.is_finished());
    assert_eq!(reply(&configuration, Input::Text("hello")), SETUP_REQUIRED);
}
