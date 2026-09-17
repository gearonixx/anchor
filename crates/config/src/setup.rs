use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use state::{PeerConfig, ConfigStore, UserDataPaths};

use crate::spec::Answer;
use crate::steps::timezone;

const STEPS: &[&dyn crate::spec::Step] = &[&timezone::Timezone];

const SETUP_REQUIRED: &str = "Please complete the initial setup.\n\nReply with \"yes\" to begin.";
const FINISHED: &str = "✅ <b>Setup complete.</b> Happy chatting!";
const DONE_STEP: &str = "done";

const TRIMMED_PUNCTUATION: [char; 5] = ['.', '!', ',', ')', '('];

pub enum Input<'a> {
    Text(&'a str),
    Reaction,
    Other,
}

pub enum ConfigurationStep {
    Finished,
    Reply(String),
    Skip,
}

pub struct Configuration {
    user_id: i64,
    store: ConfigStore,
}

impl Configuration {
    pub fn for_user(data_dir: impl AsRef<Path>, user_id: i64) -> Self {
        let store = ConfigStore::new(&UserDataPaths::for_user(data_dir, user_id));

        Self { user_id, store }
    }

    pub fn is_finished(&self) -> bool {
        self.store.exists() && self.store.load_json().finished
    }

    pub fn create_step(&self, input: Input<'_>, now: DateTime<Utc>) -> Result<ConfigurationStep> {
        if self.store.load_json().finished {
            return Ok(ConfigurationStep::Finished);
        }

        if !self.store.exists() {
            self.store.update_json(|_| {})?;
            return Ok(ConfigurationStep::Reply(SETUP_REQUIRED.to_string()));
        }

        let text = match input {
            Input::Reaction => return Ok(ConfigurationStep::Skip),
            Input::Text(text) => Some(text),
            Input::Other => None,
        };

        let mut reply = String::new();
        self.store
            .update_json(|peer_config| reply = self.create_step_reply(peer_config, text, now))?;
        Ok(ConfigurationStep::Reply(reply))
    }

    fn create_step_reply(&self, peer_config: &mut PeerConfig, text: Option<&str>, now: DateTime<Utc>) -> String {
        if !peer_config.started {
            if !text.is_some_and(Self::is_consent) {
                return SETUP_REQUIRED.to_string();
            }
            peer_config.started = true;
            log::info!("anchor.config.started user_id={}", self.user_id);
            return self.move_to_next_step(peer_config, 0);
        }

        let index = match self.get_index(peer_config) {
            Some(idx) => idx,
            None => return self.finish(peer_config),
        };

        let text = match text {
            Some(text) => text,
            None => return self.create_step_prompt(index),
        };

        let step = STEPS[index];

        match step.check_answer(text, peer_config, now) {
            // the peer's answer was rejected, but there is no specific error
            Answer::Rejected(None) => self.create_step_prompt(index),
            Answer::Rejected(Some(error_prompt)) => error_prompt.to_string(),
            Answer::Accepted(confirmation) => {
                format!("{confirmation}\n\n{}", self.move_to_next_step(peer_config, index + 1))
            }
        }
    }

    fn move_to_next_step(&self, peer_config: &mut PeerConfig, index: usize) -> String {
        match STEPS.get(index) {
            Some(step) => {
                peer_config.current_step = Some(step.name().to_string());
                self.create_step_prompt(index)
            }
            None => self.finish(peer_config),
        }
    }

    fn finish(&self, peer_config: &mut PeerConfig) -> String {
        peer_config.current_step = Some(DONE_STEP.to_string());
        peer_config.finished = true;

        FINISHED.to_string()
    }

    fn get_index(&self, peer_config: &PeerConfig) -> Option<usize> {
        match peer_config.current_step.as_deref() {
            None => Some(0),
            Some(name) => STEPS.iter().position(|step| step.name() == name),
        }
    }

    fn create_step_prompt(&self, index: usize) -> String {
        let step = STEPS[index];
        format!("<b>{}.</b>\n\n{}", step.heading(), step.description())
    }

    fn is_consent(text: &str) -> bool {
        let word = text.trim().trim_matches(TRIMMED_PUNCTUATION).to_lowercase();
        matches!(word.as_str(), "yes" | "y")
    }
}


// TODO: document what these tests do

#[cfg(test)]
#[path = "../tests/unit/setup_test.rs"]
mod tests;
