use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use events::NoReplyLimit;

use crate::consts::{MTPROTO_SESSION_PATH, ROOT_DIR};

#[derive(Debug, Clone)]
pub struct Config {
    pub api_id: i32,
    pub api_hash: String,
    pub phone: String,

    pub session_file: PathBuf,
    pub peers: Vec<i64>,
    pub forced_probabilities: bool,
    pub no_reply_limit: NoReplyLimit,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::from_path(Path::new(ROOT_DIR).join(".env"));

        let api_id: i32 = parse(
            &var("ANCHOR_API_ID")
                .context("ANCHOR_API_ID is required")?
        ).context("ANCHOR_API_ID must be a valid integer")?;

        let api_hash = var("ANCHOR_API_HASH").context("ANCHOR_API_HASH is required")?;
        let phone = var("ANCHOR_PHONE").context("ANCHOR_PHONE is required")?;

        let session_file = Path::new(ROOT_DIR).join(MTPROTO_SESSION_PATH);

        let peers = array(&var("PEERS"));

        let forced_probabilities = boolean_var("FORCED_PROBABILITIES")?;

        let no_reply_limit = match boolean_var("OFF_NO_REPLY_LIMIT")? {
            true => NoReplyLimit::Off,
            false => NoReplyLimit::On,
        };

        Ok(Self {
            api_id,
            api_hash,
            phone,
            session_file,
            peers,
            forced_probabilities,
            no_reply_limit,
        })
    }
}

fn var(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
        _ => None,
    }
}

fn boolean_var(key: &str) -> Result<bool> {
    var(key)
        .map(|raw| raw.parse())
        .transpose()
        .with_context(|| format!("{key} must be true or false"))
        .map(Option::unwrap_or_default)
}

fn parse<T: FromStr + Default + PartialOrd>(raw: &str) -> Option<T> {
    raw.trim().parse().ok().filter(|n| *n > T::default())
}

fn array<T: FromStr + Default + PartialOrd>(raw: &Option<String>) -> Vec<T> {
    raw.as_deref()
        .unwrap_or_default()
        .split(',')
        .filter_map(parse)
        .collect()
}

