use super::*;

const PEER: i64 = 1_000_000_001;

fn app() -> GoogleApp {
    GoogleApp {
        client_id: "client-id".into(),
        client_secret: "client-secret".into(),
        redirect_uri: "http://localhost:9099/google".into(),
    }
}

fn at(timestamp: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp, 0).unwrap()
}

#[test]
fn the_consent_url_asks_for_a_refresh_token_and_carries_the_peer() {
    let url = GoogleAuth::build_consent_url(&app(), PEER).unwrap();

    assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"), "{url}");
    assert!(url.contains("access_type=offline"), "{url}");
    assert!(url.contains("prompt=consent"), "{url}");
    assert!(url.contains("state=1000000001"), "{url}");
    assert!(url.contains("calendar.readonly"), "{url}");
    assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A9099%2Fgoogle"), "{url}");
}

#[test]
fn an_exchanged_answer_keeps_the_refresh_token_and_dates_the_access_one() {
    let body = r#"{"access_token":"access","expires_in":3600,"refresh_token":"refresh"}"#;

    let tokens = GoogleAuth::read_tokens(body, at(1_000), "").unwrap();

    assert_eq!(tokens.refresh_token, "refresh");
    assert_eq!(tokens.access_token, "access");
    assert_eq!(tokens.access_expires_at, 1_000 + 3600 - 60);
}

#[test]
fn a_refreshed_answer_without_a_refresh_token_keeps_the_known_one() {
    let body = r#"{"access_token":"newer","expires_in":3600}"#;

    let tokens = GoogleAuth::read_tokens(body, at(1_000), "kept").unwrap();

    assert_eq!(tokens.refresh_token, "kept");
    assert_eq!(tokens.access_token, "newer");
}

#[test]
fn an_answer_with_no_refresh_token_at_all_is_an_error() {
    let body = r#"{"access_token":"access","expires_in":3600}"#;

    assert!(GoogleAuth::read_tokens(body, at(1_000), "").is_err());
}

#[test]
fn a_consent_answer_carries_the_code_and_the_peer_it_belongs_to() {
    let answer = GoogleAuth::read_consent_answer("/google?code=4%2F0AX&state=1000000001&scope=calendar").unwrap();

    assert_eq!(answer.code, "4/0AX");
    assert_eq!(answer.peer_id, PEER);
}

#[test]
fn a_consent_answer_without_a_code_or_a_peer_is_not_read() {
    assert!(GoogleAuth::read_consent_answer("/google?state=1000000001").is_none());
    assert!(GoogleAuth::read_consent_answer("/google?code=4%2F0AX").is_none());
    assert!(GoogleAuth::read_consent_answer("/google?code=4%2F0AX&state=not-a-peer").is_none());
}
