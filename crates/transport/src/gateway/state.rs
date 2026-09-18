use anyhow::Result;
use chrono::{DateTime, Utc};
use calendar::CalendarReminders;
use events::{Event, LocalDate, PeerUtcLayers, Reminder, SkipReason, SleepWindow};
use scheduler::Candle;
use state::{
    AgentState, CalendarEvent, CalendarState, CalendarStore, ConfigStore, EventsState, EventsStore, GoogleTokens,
    PeerConfig, PeerReaction, Sender, StateStore, UserDataPaths,
};

use crate::client::{IncomingMessage, MessageKind};
use crate::consts::data_dir;

pub(crate) struct UserState {
    user_id: i64,
    state_store: StateStore,
    events_store: EventsStore,
    config_store: ConfigStore,
    calendar_store: CalendarStore,
}

impl UserState {
    pub(crate) fn for_user(user_id: i64) -> Self {
        let paths = UserDataPaths::for_user(data_dir(), user_id);
        Self {
            user_id,
            state_store: StateStore::new(&paths),
            events_store: EventsStore::new(&paths),
            config_store: ConfigStore::new(&paths),
            calendar_store: CalendarStore::new(&paths),
        }
    }

    pub(crate) fn load_events(&self) -> EventsState {
        self.events_store.load_json()
    }

    pub(crate) fn load_config(&self) -> PeerConfig {
        self.config_store.load_json()
    }

    pub(crate) fn load_calendar(&self) -> CalendarState {
        self.calendar_store.load_json()
    }

    pub(crate) fn connect_calendar(&self, tokens: GoogleTokens, now: DateTime<Utc>) -> Result<()> {
        self.calendar_store.update_json(|c| {
            c.tokens = Some(tokens);
            c.connected_at = Some(now.timestamp());
            c.refreshed_at = None;
        })?;
        Ok(())
    }

    pub(crate) fn store_access_token(&self, tokens: GoogleTokens) -> Result<()> {
        self.calendar_store.update_json(|c| c.tokens = Some(tokens))?;
        Ok(())
    }

    pub(crate) fn store_upcoming_events(&self, upcoming: Vec<CalendarEvent>, now: DateTime<Utc>) -> Result<()> {
        self.calendar_store
            .update_json(|c| CalendarReminders::store_upcoming(c, upcoming, now))?;
        Ok(())
    }

    pub(crate) fn record_sent_heads_up(&self, event_id: &str, now: DateTime<Utc>) -> Result<()> {
        self.calendar_store
            .update_json(|c| CalendarReminders::record_sent_heads_up(c, event_id, now))?;
        Ok(())
    }

    pub(crate) fn get_two_utc_layers(&self, now: DateTime<Utc>) -> Option<PeerUtcLayers> {
        PeerUtcLayers::resolve_two_utc_layers(&self.load_config(), &self.load_events(), now)
    }

    pub(crate) fn is_asleep(&self, now: DateTime<Utc>) -> bool {
        self.get_two_utc_layers(now)
            .is_some_and(|layers| SleepWindow::is_agent_asleep(self.user_id, &layers, &self.load_events(), now))
    }

    pub(crate) fn record_incoming(&self, message: &IncomingMessage) -> Result<()> {
        let is_reaction = matches!(message.kind, MessageKind::Reaction { .. });
        let message_id = message.message_id;
        let sent_at = message.sent_at;

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

        self.events_store
            .update_json(|e| events::ActivityRecorder::record_peer_message(e, sent_at))?;

        Ok(())
    }

    pub(crate) fn awaiting_confirmation(&self, text: &str) -> Option<Reminder> {
        let events = self.load_events();

        Reminder::ALL.into_iter().find(|reminder| {
            let is_its_confirmation_word = reminder.is_confirmation(text);
            let is_awaiting_confirmation = reminder.is_awaiting_confirmation(&events);

            is_its_confirmation_word && is_awaiting_confirmation
        })
    }

    pub(crate) fn start_reminder(&self, reminder: Reminder, now: DateTime<Utc>) -> Result<()> {
        self.events_store.update_json(|e| reminder.start_cycle(e, now))?;
        Ok(())
    }

    pub(crate) fn confirm_reminder(&self, reminder: Reminder, now: DateTime<Utc>) -> Result<()> {
        self.events_store.update_json(|e| reminder.confirm_cycle(e, now))?;
        Ok(())
    }

    pub(crate) fn record_sent_reminder(&self, reminder: Reminder, now: DateTime<Utc>) -> Result<()> {
        self.events_store.update_json(|e| reminder.record_sent(e, now))?;
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
