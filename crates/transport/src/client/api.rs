use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use grammers_client::message::InputMessage;
use grammers_client::session::storages::SqliteSession;
use grammers_client::session::types::{PeerId, PeerRef};
use grammers_client::{Client, tl};
use grammers_session::Session;

use super::OutgoingMessage;

#[derive(Clone)]
pub struct ClientApi {
    pub(crate) client: Client,
    pub(crate) session: Arc<SqliteSession>,
}

impl ClientApi {
    pub(crate) fn new(client: Client, session: Arc<SqliteSession>) -> Self {
        Self { client, session }
    }

    pub(crate) fn raw(&self) -> &Client {
        &self.client
    }

    pub async fn send_message(&self, peer_id: i64, message: OutgoingMessage) -> Result<i32> {
        let peer = self.peer_ref(peer_id).await?;
        let message = match message {
            OutgoingMessage::Text(text) => InputMessage::new().text(text),
            OutgoingMessage::Html(html) => InputMessage::new().html(html),
        };
        let sent = self.client.send_message(peer, message).await?;
        Ok(sent.id())
    }

    pub async fn send_with_typing(
        &self,
        peer_id: i64,
        message: OutgoingMessage,
        duration: Duration,
    ) -> Result<i32> {
        self.set_typing(peer_id, true).await?;
        tokio::time::sleep(duration).await;
        self.set_typing(peer_id, false).await?;
        self.send_message(peer_id, message).await
    }

    pub async fn set_typing(&self, peer_id: i64, active: bool) -> Result<()> {
        let peer = self.peer_ref(peer_id).await?;
        let action = self.client.action(peer);
        if active {
            action.oneshot(tl::types::SendMessageTypingAction {}).await?;
        } else {
            action.cancel().await?;
        }
        Ok(())
    }

    pub async fn set_recording_voice(&self, peer_id: i64, active: bool) -> Result<()> {
        let peer = self.peer_ref(peer_id).await?;
        let action = self.client.action(peer);
        if active {
            action.oneshot(tl::types::SendMessageRecordAudioAction {}).await?;
        } else {
            action.cancel().await?;
        }
        Ok(())
    }

    pub async fn mark_read(&self, peer_id: i64, max_id: i32) -> Result<()> {
        let peer = self.peer_ref(peer_id).await?;
        self.client
            .invoke(&tl::functions::messages::ReadHistory {
                peer: peer.into(),
                max_id,
            })
            .await?;
        Ok(())
    }

    pub async fn send_reaction(
        &self,
        peer_id: i64,
        message_id: i32,
        emoji: &str,
        big: bool,
    ) -> Result<()> {
        let peer = self.peer_ref(peer_id).await?;
        self.client
            .invoke(&tl::functions::messages::SendReaction {
                big,
                add_to_recent: false,
                peer: peer.into(),
                msg_id: message_id,
                reaction: Some(vec![
                    tl::types::ReactionEmoji {
                        emoticon: emoji.to_string(),
                    }
                    .into(),
                ]),
            })
            .await?;
        Ok(())
    }

    async fn peer_ref(&self, peer_id: i64) -> Result<PeerRef> {
        let peer = PeerId::user(peer_id)
            .ok_or_else(|| anyhow!("{peer_id} is not a valid telegram user id"))?;

        if let Some(peer_ref) = self.session.peer_ref(peer).await? {
            return Ok(peer_ref);
        }

        log::debug!("tg.peer.cache_miss id={peer_id}, re-priming dialogs");
        self.prime_peer_cache().await;

        self.session
            .peer_ref(peer)
            .await?
            .ok_or_else(|| anyhow!("could not find the input entity for {peer_id}"))
    }

    pub async fn prime_peer_cache(&self) {
        let mut dialogs = self.client.iter_dialogs();
        let mut seen = 0usize;

        loop {
            match dialogs.next().await {
                Ok(Some(_)) => {
                    seen += 1;
                    if seen >= 100 {
                        break;
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    log::warn!("tg.dialogs.prime_failed: {err}");
                    break;
                }
            }
        }

        log::debug!("tg.dialogs.primed count={seen}");
    }
}
