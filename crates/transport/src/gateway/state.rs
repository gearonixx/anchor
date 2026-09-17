use anyhow::Result;
use chrono::{DateTime, Utc};
use events::{Event, LocalDate, PeerUtcLayers, SkipReason, SleepWindow, WakeUpDetector};
use scheduler::Candle;
use state::{
    ConfigStore, EventsState, EventsStore, PeerConfig, Sender, AgentState, StateStore, UserDataPaths, PeerReaction,
};

use crate::client::{IncomingMessage, MessageKind};
use crate::consts::data_dir;

pub(crate) enum RecordStatus {
    WokeUp,
    Normal,
}

pub(crate) struct UserState {
    user_id: i64,
    state_store: StateStore,
    events_store: EventsStore,
    config_store: ConfigStore,
}

impl UserState {
    pub(crate) fn for_user(user_id: i64) -> Self {
        let paths = UserDataPaths::for_user(data_dir(), user_id);
        Self {
            user_id,
            state_store: StateStore::new(&paths),
            events_store: EventsStore::new(&paths),
            config_store: ConfigStore::new(&paths),
        }
    }

    pub(crate) fn load_events(&self) -> EventsState {
        self.events_store.load_json()
    }

    pub(crate) fn load_config(&self) -> PeerConfig {
        self.config_store.load_json()
    }

    pub(crate) fn get_two_utc_layers(&self, now: DateTime<Utc>) -> Option<PeerUtcLayers> {
        PeerUtcLayers::resolve_two_utc_layers(&self.load_config(), &self.load_events(), now)
    }

    pub(crate) fn is_asleep(&self, now: DateTime<Utc>) -> bool {
        self.get_two_utc_layers(now)
            .is_some_and(|layers| SleepWindow::is_agent_asleep(self.user_id, &layers, now))
    }

    pub(crate) fn record_incoming(&self, message: &IncomingMessage) -> Result<RecordStatus> {
        let is_reaction = matches!(message.kind, MessageKind::Reaction { .. });
        let message_id = message.message_id;
        let sent_at = message.sent_at;

        // None
        let previous_message = self.load_events().last_message.and_then(|ts| DateTime::from_timestamp(ts, 0));
        // true - so
        let is_wake = self.get_two_utc_layers(sent_at).is_some_and(|layers| {
            WakeUpDetector::is_first_activity_today(self.user_id, &layers, previous_message, sent_at)
        });

        self.state_store.update_json(|s: &mut AgentState| {
            if !is_reaction {
                s.tg_message_id = Some(message_id as i64);
                s.sender = Some(Sender::Peer);
                s.date = Some(sent_at.timestamp());
            }
            // TODO: note that this is never reset to false anywhere
            s.chat_active = Some(true);
            // same
            s.peer_ignored_count = Some(0);
            // TODO: note that the message count is incremented here
            // but never reset
            s.peer_message_count = Some(s.peer_message_count.unwrap_or(1) + 1);
        })?;

        self.events_store.update_json(|e| {
            events::ActivityRecorder::record_peer_message(e, sent_at);
            if is_wake {
                e.woke_up = Some(sent_at.timestamp());
                e.got_up = None;
                e.last_reminder = None;
            }
        })?;

        // temporary
        let status = if is_wake {
            RecordStatus::WokeUp
        } else {
            RecordStatus::Normal
        };

        Ok(status)
    }

    pub(crate) fn is_awaiting_get_up(&self) -> bool {
        let events = self.load_events();
        events.woke_up.is_some() && events.got_up.is_none()
    }

    pub(crate) fn confirm_got_up(&self, now: DateTime<Utc>) -> Result<()> {
        self.events_store.update_json(|e| e.got_up = Some(now.timestamp()))?;
        Ok(())
    }

    pub(crate) fn record_wake_reminder(&self, now: DateTime<Utc>) -> Result<()> {
        self.events_store.update_json(|e| e.last_reminder = Some(now.timestamp()))?;
        Ok(())
    }

    pub(crate) fn was_message_already_present(&self, message: &IncomingMessage) -> Result<bool> {
        match &message.kind {
            MessageKind::Reaction { emoji } => {
                self.was_peer_reaction_present(PeerReaction::new(message.message_id, emoji, message.sent_at))
            }
            _ => self.was_peer_message_present(message.message_id),
        }
    }

    // marks the peer's message as already seen/handled
    fn was_peer_message_present(&self, message_id: i32) -> Result<bool> {
        let message_id = i64::from(message_id);
        let mut is_new = false;

        self.state_store.update_json(|state: &mut AgentState| {
            if state.is_incoming_message_id_new(message_id) {
                is_new = true;
                state.last_peer_message_id = Some(message_id);
            }
        })?;

        Ok(is_new)
    }

    fn was_peer_reaction_present(&self, reaction: PeerReaction) -> Result<bool> {
        let mut is_new = false;

        self.state_store.update_json(|state: &mut AgentState| {
            if state.is_peer_reaction_new(&reaction) {
                is_new = true;
                state.add_peer_reaction(reaction);
            }
        })?;

        Ok(is_new)
    }

    pub(crate) fn record_outgoing(&self, message_id: i32) -> Result<()> {
        self.state_store.update_json(|s: &mut AgentState| {
            s.tg_message_id = Some(message_id as i64);
            s.sender = Some(Sender::Agent);
            s.date = Some(Utc::now().timestamp());
        })?;
        Ok(())
    }

    pub(crate) fn record_sent_event(&self, kind: Event, date: LocalDate) -> Result<()> {
        self.events_store
            .update_json(|e| events::EventRecorder::record_sent_event(e, kind, date))?;
        Ok(())
    }

    pub(crate) fn record_skipped_event(&self, kind: Event, date: LocalDate, reason: SkipReason) -> Result<()> {
        self.events_store
            .update_json(|e| events::EventRecorder::record_skipped_event(e, kind, date, reason))?;
        Ok(())
    }

    pub(crate) fn record_fired_candle(&self, candle: &Candle) -> Result<()> {
        self.events_store.update_json(|e| candle.record_fired(e, Utc::now()))?;
        Ok(())
    }
}
