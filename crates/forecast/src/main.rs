use std::path::PathBuf;

use chrono::{DateTime, Utc};
use config::Configuration;
use events::{LocalDate, EventForecast, NoReplyLimit, PeerUtcLayers};
use scheduler::{Candle, CandleSkipReason, CandleStatus, GapDistribution};
use serde_json::{Value, json};
use state::utils::time_units::seconds_to_minutes;
use state::{EventsState, EventsStore, ConfigStore, UserDataPaths};

const SHOWN_DAYS: usize = 3;


struct Forecast {
    data_dir: PathBuf,
    forced_probabilities: bool,
    no_reply_limit: NoReplyLimit,
}

impl Forecast {
    fn from_command_line() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let data_dir = args.first().map(PathBuf::from).expect("data dir argument");

        let no_reply_limit = match args.iter().any(|arg| arg == "--off-no-reply-limit") {
            true => NoReplyLimit::Off,
            false => NoReplyLimit::On,
        };

        Self {
            data_dir,
            forced_probabilities: args.iter().any(|arg| arg == "--forced"),
            no_reply_limit,
        }
    }

    fn snapshot(&self) -> Value {
        let now = Utc::now();

        json!({
            "generated_at": now.to_rfc3339(),
            "forced_probabilities": self.forced_probabilities,
            "off_no_reply_limit": self.no_reply_limit == NoReplyLimit::Off,
            "gap_distribution": Self::gap_distribution(),
            "peers": self.peer_ids().into_iter().map(|peer_id| self.peer(peer_id, now)).collect::<Vec<_>>(),
        })
    }

    fn gap_distribution() -> Vec<Value> {
        GapDistribution::probability_of_every_gap()
            .iter()
            .map(|&(gap, share)| json!({ "gap": gap, "p": share }))
            .collect()
    }

    fn peer_ids(&self) -> Vec<i64> {
        std::fs::read_dir(&self.data_dir)
            .map(|entries| {
                let mut ids: Vec<i64> = entries
                    .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse().ok())
                    .collect();
                ids.sort_unstable();
                ids
            })
            .unwrap_or_default()
    }

    fn peer(&self, peer_id: i64, now: DateTime<Utc>) -> Value {
        let paths = UserDataPaths::for_user(&self.data_dir, peer_id);
        let peer_config = ConfigStore::new(&paths).load_json();
        let events = EventsStore::new(&paths).load_json();

        let is_configured = Configuration::for_user(&self.data_dir, peer_id).is_finished();
        let clock = PeerUtcLayers::resolve_two_utc_layers(&peer_config, &events, now)
            .filter(|_| is_configured);

        json!({
            "peer_id": peer_id,
            "is_configured": is_configured,
            "events_no_reply": events.events_no_reply,
            "layer1": clock.map(|clock| clock.layer1.to_string()),
            "layer1_minutes": clock.map(|clock| seconds_to_minutes(clock.layer1.local_minus_utc())),
            "layer2_minutes": clock.map(|clock| clock.layer2),
            "local_date": clock.map(|clock| clock.from_utc_to_local(now).to_string()),
            "days": clock.map(|clock| self.days(peer_id, &clock, &events, now)),
        })
    }

    fn days(&self, peer_id: i64, clock: &PeerUtcLayers, events: &EventsState, now: DateTime<Utc>) -> Vec<Value> {
        let mut dates: Vec<LocalDate> =
            std::iter::successors(Some(clock.from_utc_to_local(now)), |date| date.pred_opt())
                .take(SHOWN_DAYS)
                .collect();
        dates.reverse();

        dates
            .into_iter()
            .map(|date| {
                json!({
                    "local_date": date.to_string(),
                    "events": self.event_forecasts(peer_id, date, clock, events, now),
                    "candles": self.candles(peer_id, date, clock, events, now),
                })
            })
            .collect()
    }

    fn candles(
        &self,
        peer_id: i64,
        date: LocalDate,
        clock: &PeerUtcLayers,
        events: &EventsState,
        now: DateTime<Utc>,
    ) -> Vec<Value> {
        let plan = Candle::plan_day(peer_id, date, clock);

        let not_fired = plan
            .iter()
            .map(|candle| (candle.time, candle.status(events, now, self.no_reply_limit), Some(candle)))
            .filter(|&(_, status, _)| status != CandleStatus::Fired);
        let fired = Candle::fired_times_inside_plan(&plan, events)
            .into_iter()
            .map(|time| {
                let spoken = plan.iter().find(|candle| candle.is_own_firing_time(time));
                (time, CandleStatus::Fired, spoken)
            });

        let mut candles: Vec<_> = not_fired.chain(fired).collect();
        candles.sort_by_key(|&(time, _, _)| time);

        candles
            .into_iter()
            .map(|(time, status, candle)| {
                json!({
                    "name": candle.map(Candle::name),
                    "utc": time.to_rfc3339(),
                    "local": time.with_timezone(&clock.layer1).to_rfc3339(),
                    "status": status.name(),
                    "reason": Self::skip_reason(status, clock),
                    "text": candle.map(|candle| candle.text),
                })
            })
            .collect()
    }

    fn skip_reason(status: CandleStatus, clock: &PeerUtcLayers) -> Value {
        match status {
            CandleStatus::Skipped(reason) => Self::described_skip_reason(reason, clock),
            _ => Value::Null,
        }
    }

    fn described_skip_reason(reason: CandleSkipReason, clock: &PeerUtcLayers) -> Value {
        let name = reason.name();

        match reason {
            CandleSkipReason::ConversationWasGoing { peer_wrote } => json!({
                "name": name,
                "peer_wrote_local": peer_wrote.with_timezone(&clock.layer1).to_rfc3339(),
            }),
            CandleSkipReason::IgnoredTooManyTimes { unanswered, limit } => json!({
                "name": name,
                "unanswered": unanswered,
                "limit": limit,
            }),
            CandleSkipReason::NotKnownSinceThePeerWrote { peer_wrote } => json!({
                "name": name,
                "peer_wrote_local": peer_wrote.with_timezone(&clock.layer1).to_rfc3339(),
            }),
        }
    }

    fn event_forecasts(
        &self,
        peer_id: i64,
        date: LocalDate,
        clock: &PeerUtcLayers,
        events: &EventsState,
        now: DateTime<Utc>,
    ) -> Vec<Value> {
        EventForecast::for_date(peer_id, date, events, clock, now, self.forced_probabilities, self.no_reply_limit)
            .into_iter()
            .map(|forecast| {
                json!({
                    "kind": forecast.kind.name(),
                    "date": forecast.date.to_string(),
                    "utc": forecast.time.to_rfc3339(),
                    "local": forecast.time.with_timezone(&clock.layer1).to_rfc3339(),
                    "status": forecast.status.name(),
                    "text": forecast.text,
                })
            })
            .collect()
    }
}

fn main() {
    let snapshot = Forecast::from_command_line().snapshot();

    println!("{}", serde_json::to_string_pretty(&snapshot).expect("forecast is serializable"));
}
