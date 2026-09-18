use super::*;

const ANSWER: &str = r#"{
  "items": [
    {"id": "one", "summary": "standup", "status": "confirmed", "start": {"dateTime": "2026-09-18T09:30:00+03:00"}},
    {"id": "all-day", "summary": "holiday", "start": {"date": "2026-09-18"}},
    {"id": "gone", "summary": "cancelled one", "status": "cancelled", "start": {"dateTime": "2026-09-18T10:00:00+03:00"}},
    {"id": "two", "status": "confirmed", "start": {"dateTime": "2026-09-18T11:00:00+03:00"}}
  ]
}"#;

#[test]
fn only_timed_and_living_events_are_read() {
    let events = GoogleCalendar::read_events(ANSWER).unwrap();

    let ids: Vec<&str> = events.iter().map(|event| event.id.as_str()).collect();
    assert_eq!(ids, ["one", "two"]);
}

#[test]
fn an_event_without_a_title_is_read_as_untitled() {
    let events = GoogleCalendar::read_events(ANSWER).unwrap();

    assert_eq!(events[0].title, "standup");
    assert_eq!(events[1].title, "untitled");
}

#[test]
fn a_start_is_read_as_a_utc_timestamp() {
    let events = GoogleCalendar::read_events(ANSWER).unwrap();

    let expected = DateTime::parse_from_rfc3339("2026-09-18T09:30:00+03:00").unwrap();
    assert_eq!(events[0].starts_at, expected.timestamp());
}

#[test]
fn an_empty_calendar_reads_as_no_events() {
    assert!(GoogleCalendar::read_events(r#"{}"#).unwrap().is_empty());
}

#[test]
fn the_events_url_asks_google_to_expand_and_sort_the_events() {
    let now = DateTime::parse_from_rfc3339("2026-09-18T09:00:00+00:00").unwrap().to_utc();

    let url = GoogleCalendar::build_events_url(now).unwrap().to_string();

    assert!(url.contains("singleEvents=true"), "{url}");
    assert!(url.contains("orderBy=startTime"), "{url}");
    assert!(url.contains("timeMin=2026-09-18T09%3A00%3A00"), "{url}");
    assert!(url.contains("timeMax=2026-09-19T09%3A00%3A00"), "{url}");
}
