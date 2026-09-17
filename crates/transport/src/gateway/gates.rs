use crate::gateway::MessageGate;
use crate::client::IncomingMessage;

pub struct MessageGateImpl;

impl MessageGate for MessageGateImpl {
    async fn on_text(&self, msg: &IncomingMessage, text: &str) -> Option<String> {
        log::info!(
            "anchor.text.received user_id={} msg_id={} text={text:?}",
            msg.user_id,
            msg.message_id
        );
        println!("reply [text]");
        Some("reply [text]".to_string())
    }

    async fn on_emoji(&self, msg: &IncomingMessage, emoji: &str) -> Option<String> {
        log::info!(
            "anchor.emoji.received user_id={} msg_id={} emoji={emoji:?}",
            msg.user_id,
            msg.message_id
        );
        println!("reply [emoji]");
        Some("reply [emoji]".to_string())
    }

    async fn on_photo(&self, msg: &IncomingMessage, caption: Option<&str>) -> Option<String> {
        log::info!(
            "anchor.photo.received user_id={} msg_id={} caption={caption:?}",
            msg.user_id,
            msg.message_id
        );
        println!("reply [photo]");
        Some("reply [photo]".to_string())
    }

    async fn on_sticker(
        &self,
        msg: &IncomingMessage,
        emoji: &str,
        animated: bool,
    ) -> Option<String> {
        log::info!(
            "anchor.sticker.received user_id={} msg_id={} emoji={emoji:?} animated={animated}",
            msg.user_id,
            msg.message_id
        );
        println!("reply [sticker]");
        Some("reply [sticker]".to_string())
    }

    async fn on_gif(&self, msg: &IncomingMessage, caption: Option<&str>) -> Option<String> {
        log::info!(
            "anchor.gif.received user_id={} msg_id={} caption={caption:?}",
            msg.user_id,
            msg.message_id
        );
        println!("reply [gif]");
        Some("reply [gif]".to_string())
    }

    async fn on_voice(&self, msg: &IncomingMessage, duration: Option<f64>) -> Option<String> {
        log::info!(
            "anchor.voice.received user_id={} msg_id={} duration={duration:?}",
            msg.user_id,
            msg.message_id
        );
        println!("reply [voice]");
        Some("reply [voice]".to_string())
    }

    async fn on_reaction(&self, msg: &IncomingMessage, emoji: &str) -> Option<String> {
        log::info!(
            "anchor.reaction.received user_id={} msg_id={} emoji={emoji:?}",
            msg.user_id,
            msg.message_id
        );
        println!("reply [reaction]");
        Some("reply [reaction]".to_string())
    }

    async fn on_other(&self, msg: &IncomingMessage) -> Option<String> {
        log::info!(
            "anchor.message.unhandled user_id={} msg_id={}",
            msg.user_id,
            msg.message_id
        );
        None
    }
}
