use super::*;

fn tokens(expires_at: i64) -> GoogleTokens {
    GoogleTokens {
        refresh_token: "refresh".into(),
        access_token: "access".into(),
        access_expires_at: expires_at,
    }
}

#[test]
fn a_calendar_without_tokens_is_not_connected() {
    assert!(!CalendarState::default().is_connected());

    let connected = CalendarState { tokens: Some(tokens(10)), ..CalendarState::default() };
    assert!(connected.is_connected());
}

#[test]
fn an_access_token_goes_stale_at_its_expiry() {
    let tokens = tokens(1_000);

    assert!(tokens.has_fresh_access(999));
    assert!(!tokens.has_fresh_access(1_000));
}

#[test]
fn an_empty_access_token_is_never_fresh() {
    let tokens = GoogleTokens { access_token: String::new(), ..tokens(1_000) };

    assert!(!tokens.has_fresh_access(1));
}

#[test]
fn reminded_events_are_forgotten_once_they_are_old() {
    let mut calendar = CalendarState::default();
    calendar.reminded.insert("old".into(), 100);
    calendar.reminded.insert("fresh".into(), 300);

    calendar.forget_events_before(200);

    assert!(!calendar.reminded.contains_key("old"));
    assert!(calendar.reminded.contains_key("fresh"));
}
