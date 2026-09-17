use std::path::{Path, PathBuf};
use std::time::Duration;

pub(crate) const ROOT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
pub(crate) const MTPROTO_SESSION_PATH: &str = ".anchor/session.sqlite";
pub(crate) const DATA_DIR_PATH: &str = ".anchor/data";

pub(crate) fn data_dir() -> PathBuf {
    Path::new(ROOT_DIR).join(DATA_DIR_PATH)
}

// hardcoded for now
pub(crate) const TYPING_DURATION: Duration = Duration::from_millis(1200);
