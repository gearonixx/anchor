use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use config::Configuration;
use events::{Decision, LocalDate, Event, NoReplyLimit, PeerUtcLayers, Reminder, SkipReason};
use scheduler::Candle;
use state::{ConfigStore, UserDataPaths};
use tokio::task::JoinHandle;
use tokio::time::{Interval, MissedTickBehavior};

use crate::client::{ClientApi, OutgoingMessage};
use crate::consts::{TYPING_DURATION, data_dir};
use crate::gateway::UserState;
use crate::helpers::is_real_user;

const TICK: Duration = Duration::from_secs(60);

pub fn spawn(api: ClientApi, peers: Arc<[i64]>, forced_probabilities: bool, no_reply_limit: NoReplyLimit) -> JoinHandle<()> {
    tokio::spawn(EventLoop::new(api, peers, forced_probabilities, no_reply_limit).run())
}

struct EventLoop {
    api: ClientApi,
    peers: Arc<[i64]>,
    forced_probabilities: bool,
    no_reply_limit: NoReplyLimit,
}

// background timer for proactive messages.

// find the peers
// ↓
// load config.json + events.json
// ↓
// work out their local time

impl EventLoop {
    fn new(api: ClientApi, peers: Arc<[i64]>, forced_probabilities: bool, no_reply_limit: NoReplyLimit) -> Self {
        Self {
            api,
            peers,
            forced_probabilities,
            no_reply_limit,
        }
    }

    async fn run(mut self) {
        if self.forced_probabilities { log::warn!("anchor.events.forced_probabilities on"); }
        if self.no_reply_limit == NoReplyLimit::Off { log::warn!("anchor.events.no_reply_limit off"); }

        let mut ticker = Self::create_minute_timer();

        loop {
            ticker.tick().await;
            for peer_id in self.get_peer_ids() {
                // checks whether an event is due for this peer right now.
                self.check_peer_events(peer_id)
                    .await
                    .unwrap_or_else(|err| log::error!("anchor.events.failed peer={peer_id}: {err:#}"));
            }
        }
    }

    fn create_minute_timer() -> Interval {
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        ticker
    }

    fn get_peer_ids(&self) -> Vec<i64> {
        std::fs::read_dir(data_dir())
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse().ok())
                    .filter(|&peer_id| is_real_user(peer_id, &self.peers))
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn check_peer_events(&mut self, peer_id: i64) -> Result<()> {
        let paths = UserDataPaths::for_user(data_dir(), peer_id);

        // PeerConfigStore.load_json() → config.json
        // EventsStore.load_json()     → events.json
        let peer_config = ConfigStore::new(&paths).load_json();
        let state = UserState::for_user(peer_id);
        let events = state.load_events();

        let now = Utc::now();
        let is_configured = Configuration::for_user(data_dir(), peer_id).is_finished();

        // l1 and l2
        let utc_layers = PeerUtcLayers::resolve_two_utc_layers(&peer_config, &events, now).filter(|_| is_configured);

        // DEBUG

        // "not computed" means rhythm_shift is never called and layer2 falls back to 0
        // anchor.events.clock peer=777000 layer1=Some(+03:00) layer2=Some(0)
        log::info!(
            "anchor.events.clock peer={peer_id} layer1={:?} layer2={:?}",
            utc_layers.map(|clock| clock.layer1),
            utc_layers.map(|clock| clock.layer2)
        );

        if utc_layers.is_none() { return Ok(()); }

        let clock = utc_layers.unwrap();

        let decisions_for_now = events::compute_decision(
            peer_id,
            &events,
            &clock,
            now,
            self.forced_probabilities,
            self.no_reply_limit,
        );

        for decision in decisions_for_now {
            match decision {
                Decision::SendEvent { kind, date, text } => {
                    self.record_and_send_event(peer_id, &state, kind, date, text).await?;
                }

                Decision::SetAsSkipped { kind, date, reason } => {
                    // resolved_events in events.json
                    // its only purpose: remember for which day each event is already closed.
                    // stored date != current day (e.g. still 12.09)
                    // so the event for 13.09 is NOT handled yet
                    Self::set_event_as_skipped(
                        peer_id,
                        &state,
                        kind,
                        date,
                        reason,
                    )?;
                }
            }
        }

        if let Some(candle) = Candle::due_now(peer_id, &events, &clock, now, self.no_reply_limit) {
            self.record_and_send_candle(peer_id, &state, candle).await?;
        }

        for reminder in Reminder::ALL {
            if reminder.should_remind(&events, now) && !state.is_asleep(now) {
                self.send_reminder(peer_id, &state, reminder, now).await?;
            }
        }

        Ok(())
    }

    async fn send_reminder(
        &self,
        peer_id: i64,
        state: &UserState,
        reminder: Reminder,
        now: DateTime<Utc>,
    ) -> Result<()> {
        state.record_sent_reminder(reminder, now)?;

        let text = reminder.reminding_text().to_string();
        let sent_id = self.api.send_with_typing(peer_id, OutgoingMessage::Text(text), TYPING_DURATION).await?;
        state.record_outgoing(sent_id)?;

        log::info!("anchor.reminder.sent peer={peer_id} reminder={}", reminder.name());

        Ok(())
    }

    async fn start_reminder(&self, peer_id: i64, state: &UserState, reminder: Reminder) -> Result<()> {
        state.start_reminder(reminder, Utc::now())?;

        let text = reminder.prompt().to_string();
        let sent_id = self.api.send_with_typing(peer_id, OutgoingMessage::Text(text), TYPING_DURATION).await?;
        state.record_outgoing(sent_id)?;

        log::info!("anchor.reminder.started peer={peer_id} reminder={}", reminder.name());

        Ok(())
    }

    async fn record_and_send_candle(&self, peer_id: i64, state: &UserState, candle: Candle) -> Result<()> {
        state.record_fired_candle(&candle)?;

        let sent_id = self
            .api
            .send_with_typing(peer_id, OutgoingMessage::Text(candle.text.to_string()), TYPING_DURATION)
            .await?;
        state.record_outgoing(sent_id)?;

        let (name, date, text) = (candle.name(), candle.date, candle.text);
        log::info!("anchor.candles.sent peer={peer_id} candle={name} date={date} text={text:?}");

        Ok(())
    }

    async fn record_and_send_event(
        &self,
        peer_id: i64,
        state: &UserState,
        kind: Event,
        date: LocalDate,
        text: &'static str,
    ) -> Result<()> {
        state.record_sent_event(kind, date)?;

        let sent_id = self.api.send_with_typing(peer_id, OutgoingMessage::Text(text.to_string()), TYPING_DURATION).await?;
        state.record_outgoing(sent_id)?;

        let event = kind.name();
        log::info!("anchor.events.sent peer={peer_id} event={event} date={date} text={text:?}");

        if kind == Event::DayEnd {
            self.start_reminder(peer_id, state, Reminder::GoToSleep).await?;
        }

        Ok(())
    }

    fn set_event_as_skipped(
        peer_id: i64,
        state: &UserState,
        kind: Event,
        date: LocalDate,
        reason: SkipReason,
    ) -> Result<()> {
        // "the event for this day is already handled and closed"
        state.record_skipped_event(kind, date, reason)?;

        let (event, why) = (kind.name(), reason.name());
        log::info!("anchor.events.skipped peer={peer_id} event={event} date={date} reason={why}");

        Ok(())
    }
}
