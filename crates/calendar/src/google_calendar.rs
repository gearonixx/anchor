use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, Utc};
use reqwest::Url;
use serde::Deserialize;
use state::CalendarEvent;
use state::utils::time_units::parse_str_to_minutes;

use crate::google_http_client::GOOGLE_HTTP_CLIENT;
use crate::lever::LOOK_AHEAD;

const EVENTS_ENDPOINT: &str = "https://www.googleapis.com/calendar/v3/calendars/primary/events";
const MAX_EVENTS_PER_ANSWER: &str = "50";
const UNTITLED_EVENT: &str = "untitled";
const CANCELLED: &str = "cancelled";

const LOOK_AHEAD_DURATION: Duration = Duration::minutes(parse_str_to_minutes(LOOK_AHEAD) as i64);

pub struct GoogleCalendar;

impl GoogleCalendar {
    pub async fn fetch_upcoming(access_token: &str, now: DateTime<Utc>) -> Result<Vec<CalendarEvent>> {
        let answer = GOOGLE_HTTP_CLIENT
            .get(Self::build_events_url(now)?)
            .bearer_auth(access_token)
            .send()
            .await?;

        let status = answer.status();
        let body = answer.text().await?;

        if !status.is_success() { bail!("google calendar answered {status}: {body}"); }

        Self::read_events(&body)
    }

    fn build_events_url(now: DateTime<Utc>) -> Result<Url> {
        let looks_ahead_until = now + LOOK_AHEAD_DURATION;

        let mut url = Url::parse(EVENTS_ENDPOINT)?;

        url.query_pairs_mut()
            .append_pair("timeMin", &now.to_rfc3339())
            .append_pair("timeMax", &looks_ahead_until.to_rfc3339())
            .append_pair("singleEvents", "true")
            .append_pair("orderBy", "startTime")
            .append_pair("maxResults", MAX_EVENTS_PER_ANSWER);

        Ok(url)
    }

    fn read_events(body: &str) -> Result<Vec<CalendarEvent>> {
        let answer: EventsAnswer = serde_json::from_str(body).context("google calendar answer")?;

        Ok(answer.items.into_iter().filter_map(Self::read_event).collect())
    }

    fn read_event(item: EventItem) -> Option<CalendarEvent> {
        let is_cancelled = item.status.as_deref() == Some(CANCELLED);
        if is_cancelled { return None; }

        let starts_at = DateTime::parse_from_rfc3339(&item.start?.date_time?).ok()?;

        Some(CalendarEvent {
            id: item.id?,
            title: item.summary.unwrap_or_else(|| UNTITLED_EVENT.to_owned()),
            starts_at: starts_at.timestamp(),
        })
    }
}

#[derive(Deserialize)]
struct EventsAnswer {
    #[serde(default)]
    items: Vec<EventItem>,
}

#[derive(Deserialize)]
struct EventItem {
    id: Option<String>,
    summary: Option<String>,
    status: Option<String>,
    start: Option<EventStart>,
}

#[derive(Deserialize)]
struct EventStart {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
}

#[cfg(test)]
#[path = "../tests/unit/google_calendar_test.rs"]
mod tests;
