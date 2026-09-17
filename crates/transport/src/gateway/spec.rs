use crate::client::IncomingMessage;

#[allow(async_fn_in_trait)]
pub trait MessageGate {
    async fn on_text(&self, msg: &IncomingMessage, text: &str) -> Option<String>;
    async fn on_emoji(&self, msg: &IncomingMessage, emoji: &str) -> Option<String>;
    async fn on_photo(&self, msg: &IncomingMessage, caption: Option<&str>) -> Option<String>;
    async fn on_sticker(
        &self,
        msg: &IncomingMessage,
        emoji: &str,
        animated: bool,
    ) -> Option<String>;
    async fn on_gif(&self, msg: &IncomingMessage, caption: Option<&str>) -> Option<String>;
    async fn on_voice(&self, msg: &IncomingMessage, duration: Option<f64>) -> Option<String>;
    async fn on_reaction(&self, msg: &IncomingMessage, emoji: &str) -> Option<String>;
    async fn on_other(&self, msg: &IncomingMessage) -> Option<String>;
}
