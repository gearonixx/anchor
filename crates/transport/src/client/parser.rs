use chrono::DateTime;
use grammers_client::media::Media;
use grammers_client::session::types::PeerKind;
use grammers_client::tl;
use grammers_client::update::Update;

use crate::helpers::{is_emoji, is_voice};

use super::{IncomingMessage, MessageKind};

pub struct MessageParser<'a> {
    update: &'a Update,
}

impl<'a> MessageParser<'a> {
    pub fn new(update: &'a Update) -> Self {
        Self { update }
    }

    pub fn parse(&self) -> Option<IncomingMessage> {
        self.message().or_else(|| self.reaction())
    }

    fn message(&self) -> Option<IncomingMessage> {
        let Update::NewMessage(message) = self.update else {
            return None;
        };
        if message.outgoing() {
            return None;
        }
        if message.peer_id().kind() != PeerKind::User {
            return None;
        }
        let user_id = message.sender_id()?.bare_id()?;

        let text = message.text();
        let caption = (!text.is_empty()).then(|| text.to_string());

        let media = message.media();
        let kind = match &media {
            None => {
                let text = caption.clone()?;
                if is_emoji(&text) {
                    MessageKind::Emoji { emoji: text }
                } else {
                    MessageKind::Text(text)
                }
            }
            Some(Media::Photo(_)) => MessageKind::Photo { caption },
            Some(Media::Sticker(sticker)) => MessageKind::Sticker {
                emoji: sticker.emoji().to_string(),
                animated: sticker.is_animated(),
            },
            Some(Media::Document(doc)) if doc.is_animated() => MessageKind::Gif { caption },
            Some(Media::Document(doc)) if is_voice(doc) => MessageKind::Voice {
                duration: doc.duration(),
            },
            Some(_) => MessageKind::Other,
        };

        Some(IncomingMessage {
            user_id,
            message_id: message.id(),
            sent_at: message.date(),
            kind,
        })
    }

    fn reaction(&self) -> Option<IncomingMessage> {
        let Update::Raw(raw) = self.update else {
            return None;
        };
        let tl::enums::Update::MessageReactions(reactions) = &raw.raw else {
            return None;
        };
        let tl::enums::Peer::User(sender) = &reactions.peer else {
            return None;
        };
        let user_id = sender.user_id;

        let tl::enums::MessageReactions::Reactions(results) = &reactions.reactions;
        let newest = results
            .recent_reactions
            .as_ref()?
            .iter()
            .map(|reaction| {
                let tl::enums::MessagePeerReaction::Reaction(reaction) = reaction;
                reaction
            })
            .filter(|reaction| !reaction.my && reaction.peer_id == reactions.peer)
            .max_by_key(|reaction| reaction.date)?;

        let emoji = match &newest.reaction {
            tl::enums::Reaction::Emoji(emoji) => emoji.emoticon.clone(),
            _ => return None,
        };

        Some(IncomingMessage {
            user_id,
            // the message the reaction was put on
            message_id: reactions.msg_id,
            sent_at: DateTime::from_timestamp(newest.date as i64, 0)?,
            kind: MessageKind::Reaction { emoji },
        })
    }
}
