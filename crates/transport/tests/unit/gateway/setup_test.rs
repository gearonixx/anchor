use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use config::{Configuration, ConfigurationStep};
use state::{ConfigStore, UserDataPaths};

use super::*;

#[test]
fn transport_messages_complete_setup_while_reactions_leave_it_unchanged() {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let data_dir = std::env::temp_dir().join(format!("anchor-gateway-test-{}-{unique}", std::process::id()));
    let peer_id = 1_000_000_001;
    let configuration = Configuration::for_user(&data_dir, peer_id);
    let paths = UserDataPaths::for_user(&data_dir, peer_id);
    let store = ConfigStore::new(&paths);
    let now = Utc::now();
    let apply = |kind: MessageKind| {
        configuration.create_step(ConfigurationInput::from_message_kind(&kind), now).unwrap()
    };

    assert!(matches!(apply(MessageKind::Text("hello".into())), ConfigurationStep::Reply(_)));
    assert!(!configuration.is_finished());
    assert!(matches!(apply(MessageKind::Text("yes".into())), ConfigurationStep::Reply(_)));
    assert!(store.load_json().started);

    let before = store.load_json();
    assert!(matches!(
        apply(MessageKind::Reaction { emoji: "👍".into() }),
        ConfigurationStep::Skip
    ));
    assert_eq!(store.load_json(), before);

    assert!(matches!(apply(MessageKind::Photo { caption: Some("+3".into()) }), ConfigurationStep::Reply(_)));
    assert_eq!(store.load_json(), before);
    assert!(matches!(apply(MessageKind::Emoji { emoji: "👍".into() }), ConfigurationStep::Reply(_)));
    assert_eq!(store.load_json(), before);

    assert!(matches!(apply(MessageKind::Text("+3".into())), ConfigurationStep::Reply(_)));
    assert!(configuration.is_finished());
    assert_eq!(store.load_json().timezone.unwrap().to_numeric_utc_minutes(), 180);
    assert!(matches!(apply(MessageKind::Text("hello again".into())), ConfigurationStep::Finished));

    std::fs::remove_dir_all(data_dir).unwrap();
}
