use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::Deserialize;
use state::GoogleTokens;

use crate::google_app::GoogleApp;
use crate::google_http_client::GOOGLE_HTTP_CLIENT;

const CONSENT_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";

const CONSENT_ANSWER_BASE: &str = "http://callback.invalid";

const EXPIRY_SAFETY_MARGIN_SECONDS: i64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentAnswer {
    pub code: String,
    pub peer_id: i64,
}

pub struct GoogleAuth;

impl GoogleAuth {
    pub fn build_consent_url(app: &GoogleApp, peer_id: i64) -> Result<String> {
        let mut url = Url::parse(CONSENT_ENDPOINT)?;

        url.query_pairs_mut()
            .append_pair("client_id", &app.client_id)
            .append_pair("redirect_uri", &app.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", CALENDAR_SCOPE)
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent")
            .append_pair("state", &peer_id.to_string());

        Ok(url.into())
    }

    pub fn read_consent_answer(request_target: &str) -> Option<ConsentAnswer> {
        let url = Url::parse(&format!("{CONSENT_ANSWER_BASE}{request_target}")).ok()?;

        let mut code = None;
        let mut peer_id = None;

        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "code" => code = Some(value.into_owned()),
                "state" => peer_id = value.parse().ok(),
                _ => {}
            }
        }

        Some(ConsentAnswer { code: code?, peer_id: peer_id? })
    }

    pub async fn exchange_consent_code(
        app: &GoogleApp,
        code: &str,
        now: DateTime<Utc>,
    ) -> Result<GoogleTokens> {
        let answer = Self::ask_token_endpoint(&[
            ("client_id", &app.client_id),
            ("client_secret", &app.client_secret),
            ("redirect_uri", &app.redirect_uri),
            ("grant_type", "authorization_code"),
            ("code", code),
        ])
        .await?;

        Self::read_tokens(&answer, now, "")
    }

    pub async fn refresh_access_token(
        app: &GoogleApp,
        tokens: &GoogleTokens,
        now: DateTime<Utc>,
    ) -> Result<GoogleTokens> {
        let answer = Self::ask_token_endpoint(&[
            ("client_id", &app.client_id),
            ("client_secret", &app.client_secret),
            ("grant_type", "refresh_token"),
            ("refresh_token", &tokens.refresh_token),
        ])
        .await?;

        Self::read_tokens(&answer, now, &tokens.refresh_token)
    }

    async fn ask_token_endpoint(form: &[(&str, &str)]) -> Result<String> {
        let answer = GOOGLE_HTTP_CLIENT.post(TOKEN_ENDPOINT).form(form).send().await?;

        let status = answer.status();
        let body = answer.text().await?;

        if !status.is_success() { bail!("google token endpoint answered {status}: {body}"); }

        Ok(body)
    }

    fn read_tokens(body: &str, now: DateTime<Utc>, known_refresh_token: &str) -> Result<GoogleTokens> {
        let answer: TokenAnswer = serde_json::from_str(body).context("google token answer")?;

        let refresh_token = answer
            .refresh_token
            .unwrap_or_else(|| known_refresh_token.to_owned());

        if refresh_token.is_empty() { bail!("google answered without a refresh token"); }

        Ok(GoogleTokens {
            refresh_token,
            access_token: answer.access_token,
            access_expires_at: now.timestamp() + answer.expires_in - EXPIRY_SAFETY_MARGIN_SECONDS,
        })
    }
}

#[derive(Deserialize)]
struct TokenAnswer {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

#[cfg(test)]
#[path = "../tests/unit/google_auth_test.rs"]
mod tests;
