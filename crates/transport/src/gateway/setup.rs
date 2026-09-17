use config::Input;

use crate::client::MessageKind;

pub(super) struct ConfigurationInput;

impl ConfigurationInput {
    pub(super) fn from_message_kind(kind: &MessageKind) -> Input<'_> {
        match kind {
            MessageKind::Text(text) => Input::Text(text),
            MessageKind::Emoji { emoji } => Input::Text(emoji),
            MessageKind::Reaction { .. } => Input::Reaction,
            _ => Input::Other,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/gateway/setup_test.rs"]
mod tests;
