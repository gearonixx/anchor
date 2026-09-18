use std::sync::Arc;

use anyhow::{Result, bail};
use calendar::{ConsentAnswer, GoogleApp, GoogleAuth};
use chrono::Utc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

use crate::client::{ClientApi, OutgoingMessage};
use crate::consts::TYPING_DURATION;
use crate::gateway::UserState;
use crate::helpers::is_real_user;

const REQUEST_LIMIT_BYTES: usize = 8 * 1024;
const CONNECTED_ANSWER: &str = "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\nconnection: close\r\n\r\ncalendar connected\n";
const REFUSED_ANSWER: &str = "HTTP/1.1 400 Bad Request\r\ncontent-type: text/plain\r\nconnection: close\r\n\r\ncalendar not connected\n";
const CONNECTED_MESSAGE: &str = "calendar connected";

pub fn spawn(api: ClientApi, app: GoogleApp, port: u16, peers: Arc<[i64]>) -> JoinHandle<()> {
    tokio::spawn(GoogleCallback { api, app, port, peers }.run())
}

struct GoogleCallback {
    api: ClientApi,
    app: GoogleApp,
    port: u16,
    peers: Arc<[i64]>,
}

impl GoogleCallback {
    async fn run(self) {
        let listener = match TcpListener::bind(("0.0.0.0", self.port)).await {
            Ok(listener) => listener,
            Err(err) => return log::error!("anchor.calendar.callback_failed port={}: {err:#}", self.port),
        };

        log::info!("anchor.calendar.callback_listening port={}", self.port);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => self.answer(stream).await,
                Err(err) => log::warn!("anchor.calendar.callback_refused: {err:#}"),
            }
        }
    }

    async fn answer(&self, mut stream: TcpStream) {
        let is_connected = self.connect_from(&mut stream).await.unwrap_or_else(|err| {
            log::error!("anchor.calendar.connect_failed: {err:#}");
            false
        });

        let answer = if is_connected { CONNECTED_ANSWER } else { REFUSED_ANSWER };

        let _ = stream.write_all(answer.as_bytes()).await;
        let _ = stream.shutdown().await;
    }

    async fn connect_from(&self, stream: &mut TcpStream) -> Result<bool> {
        let request = Self::read_request(stream).await?;

        let answer = match Self::read_consent_answer(&request) {
            Some(answer) => answer,
            None => return Ok(false),
        };

        if !is_real_user(answer.peer_id, &self.peers) {
            bail!("consent answer for a foreign peer {}", answer.peer_id);
        }

        let now = Utc::now();
        let tokens = GoogleAuth::exchange_consent_code(&self.app, &answer.code, now).await?;

        let state = UserState::for_user(answer.peer_id);
        state.connect_calendar(tokens, now)?;

        let sent_id = self
            .api
            .send_with_typing(answer.peer_id, OutgoingMessage::Text(CONNECTED_MESSAGE.to_owned()), TYPING_DURATION)
            .await?;
        state.record_outgoing(sent_id)?;

        log::info!("anchor.calendar.connected peer={}", answer.peer_id);

        Ok(true)
    }

    async fn read_request(stream: &mut TcpStream) -> Result<String> {
        let mut request = vec![0_u8; REQUEST_LIMIT_BYTES];
        let read = stream.read(&mut request).await?;

        Ok(String::from_utf8_lossy(&request[..read]).into_owned())
    }

    fn read_consent_answer(request: &str) -> Option<ConsentAnswer> {
        let request_target = request.split_whitespace().nth(1)?;

        GoogleAuth::read_consent_answer(request_target)
    }
}
