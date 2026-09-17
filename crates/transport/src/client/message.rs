use chrono::{DateTime, Utc};

const MAX_MESSAGE_LATENESS: chrono::Duration = chrono::Duration::minutes(3);

#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub user_id: i64,
    pub message_id: i32,
    pub sent_at: DateTime<Utc>,
    pub kind: MessageKind,
}

impl IncomingMessage {
    pub fn is_too_late_to_answer(&self, now: DateTime<Utc>) -> bool {
        now - self.sent_at > MAX_MESSAGE_LATENESS
    }
}

#[derive(Debug, Clone)]
pub enum MessageKind {
    Text(String),
    Emoji { emoji: String },
    Photo { caption: Option<String> },
    Sticker { emoji: String, animated: bool },
    Gif { caption: Option<String> },
    Voice { duration: Option<f64> },
    Reaction { emoji: String },
    Other,
}

#[derive(Debug, Clone)]
pub enum OutgoingMessage {
    Text(String),
    Html(String),
}

#[cfg(test)]
#[path = "../../tests/unit/client/message_test.rs"]
mod tests;
