use anyhow::Result;
use calendar::{GoogleApp, GoogleAuth};
use chrono::Utc;
use config::{Configuration, ConfigurationStep};
use reply_timing::ReplyTiming;

use crate::client::{ClientApi, IncomingMessage, MessageKind, OutgoingMessage};
use crate::consts::{TYPING_DURATION, data_dir};
use crate::gateway::MessageGate;
use crate::gateway::setup::ConfigurationInput;
use crate::gateway::state::UserState;
use crate::helpers::is_real_user;

const CONNECT_CALENDAR_COMMAND: &str = "calendar";
const CALENDAR_IS_OFF: &str = "calendar is not set up";

pub async fn handle_message(
    gate: &impl MessageGate,
    api: &ClientApi,
    peers: &[i64],
    google: Option<&GoogleApp>,
    message: &IncomingMessage,
) -> Result<()> {
    if !is_real_user(message.user_id, peers) {
        log::info!(
            "anchor.message.ignored user_id={} reason=foreign_peer",
            message.user_id
        );
        return Ok(());
    }

    let utc_now = Utc::now();
    let state = UserState::for_user(message.user_id);

    let is_new_message = state.was_message_already_present(message)?;

    if !is_new_message {
        let (user_id, msg_id) = (message.user_id, message.message_id);
        log::info!("anchor.message.duplicate_ignored user_id={user_id} msg_id={msg_id}");
        return Ok(());
    }

    api.mark_read(message.user_id, message.message_id).await?;

    let config = Configuration::for_user(data_dir(), message.user_id);

    if !config.is_finished() {
        match config.create_step(ConfigurationInput::from_message_kind(&message.kind), utc_now)? {
            ConfigurationStep::Finished => {}
            ConfigurationStep::Skip => return Ok(()),
            ConfigurationStep::Reply(text) => {
                api.send_with_typing(message.user_id, OutgoingMessage::Html(text.clone()), TYPING_DURATION)
                    .await?;

                log::info!("anchor.config.sent to={} text={text:?}", message.user_id);
                return Ok(());
            }
        }
    }

    state.record_incoming(message)?;

    if message.is_too_late_to_answer(utc_now) {
        let (user_id, msg_id) = (message.user_id, message.message_id);
        let late_by = (utc_now - message.sent_at).num_seconds();
        log::info!("anchor.message.too_late_ignored user_id={user_id} msg_id={msg_id} late_by={late_by}s");
        return Ok(());
    }

    if state.is_asleep(utc_now) {
        let (user_id, msg_id) = (message.user_id, message.message_id);
        log::info!("anchor.message.asleep user_id={user_id} msg_id={msg_id}");
        return Ok(());
    }

    let is_connect_calendar = match &message.kind {
        MessageKind::Text(text) => text.trim().eq_ignore_ascii_case(CONNECT_CALENDAR_COMMAND),
        _ => false,
    };

    if is_connect_calendar {
        return answer_with_consent_link(api, &state, google, message.user_id).await;
    }

    let reply = match &message.kind {
        MessageKind::Text(text) => gate.on_text(message, text).await,
        MessageKind::Emoji { emoji } => gate.on_emoji(message, emoji).await,
        MessageKind::Photo { caption } => gate.on_photo(message, caption.as_deref()).await,
        MessageKind::Sticker { emoji, animated } => {
            gate.on_sticker(message, emoji, *animated).await
        }
        MessageKind::Gif { caption } => gate.on_gif(message, caption.as_deref()).await,
        MessageKind::Voice { duration } => gate.on_voice(message, *duration).await,
        MessageKind::Reaction { emoji } => gate.on_reaction(message, emoji).await,
        MessageKind::Other => gate.on_other(message).await,
    };

    let confirmed = match &message.kind {
        MessageKind::Text(text) => state.awaiting_confirmation(text),
        _ => None,
    };

    let reply = match confirmed {
        Some(reminder) => {
            state.confirm_reminder(reminder, utc_now)?;

            log::info!("anchor.reminder.confirmed peer={} reminder={}", message.user_id, reminder.name());

            Some(reminder.ok().to_owned())
        }
        None => reply,
    };

    let reply = match reply {
        Some(reply) => reply,
        None => return Ok(()),
    };

    // TODO: placeholder - instant replies for now, to be made smarter later
    let timing = ReplyTiming::zero();

    if !timing.will_reply() { return Ok(()); }

    tokio::time::sleep(timing.delay).await;

    let sent_id = api.send_with_typing(message.user_id, OutgoingMessage::Text(reply.clone()), TYPING_DURATION).await?;

    state.record_outgoing(sent_id)?;

    log::info!("anchor.message.sent to={} text={reply:?}", message.user_id);

    Ok(())
}

async fn answer_with_consent_link(
    api: &ClientApi,
    state: &UserState,
    google: Option<&GoogleApp>,
    peer_id: i64,
) -> Result<()> {
    let link = match google {
        Some(app) => GoogleAuth::build_consent_url(app, peer_id)?,
        None => CALENDAR_IS_OFF.to_owned(),
    };

    let sent_id = api
        .send_with_typing(peer_id, OutgoingMessage::Text(link), TYPING_DURATION)
        .await?;
    state.record_outgoing(sent_id)?;

    log::info!("anchor.calendar.consent_link_sent peer={peer_id}");

    Ok(())
}
