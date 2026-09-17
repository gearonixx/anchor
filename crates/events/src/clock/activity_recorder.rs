use chrono::{DateTime, Timelike, Utc};
use state::utils::time_units::{HOURS_PER_DAY, seconds_to_days};
use state::{Activity, EventsState};

const ACTIVITY_HALF_LIFE_DAYS: f64 = 14.0;

pub struct ActivityRecorder;

impl ActivityRecorder {
    // 1. remembers the time of the last message
    // 2. resets events_no_reply = 0
    // 3. updates the per-UTC-hour activity stats
    pub fn record_peer_message(events: &mut EventsState, at: DateTime<Utc>) {
        events.last_message = Some(at.timestamp());
        events.events_no_reply = 0;

        let activity = events.activity.get_or_insert_with(|| Activity {
            utc: [0.0; HOURS_PER_DAY],
            created_at: at.timestamp(),
            updated_at: at.timestamp(),
        });

        Self::increment_peer_activity(activity, at);
    }

    // 1. compute how much time passed since the previous message
    // 2. decay the old stats a little
    // 3. add +1 to the current UTC hour
    // 4. remember when this update happened
    fn increment_peer_activity(activity: &mut Activity, at: DateTime<Utc>) {
        let elapsed_days = seconds_to_days((at.timestamp() - activity.updated_at).max(0));

        let decay = 0.5_f64.powf(elapsed_days / ACTIVITY_HALF_LIFE_DAYS);
        activity.utc.iter_mut().for_each(|bin| *bin *= decay);
        activity.utc[at.hour() as usize] += 1.0;
        activity.updated_at = activity.updated_at.max(at.timestamp());
    }
}

#[cfg(test)]
#[path = "../../tests/unit/clock/activity_recorder_test.rs"]
mod tests;
