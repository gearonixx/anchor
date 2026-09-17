use std::path::{Path, PathBuf};

use crate::tables::state::AgentState;
use crate::utils::json_store::JsonFile;

#[derive(Debug, Clone)]
pub struct UserDataPaths {
    pub data_dir: PathBuf,
    pub user_dir: PathBuf,
    pub state: PathBuf,
    pub chat_history: PathBuf,
}

impl UserDataPaths {
    pub fn for_user(root: impl AsRef<Path>, user_id: i64) -> Self {
        let data_dir = root.as_ref().to_path_buf();
        let user_dir = data_dir.join(user_id.to_string());
        
        Self {
            state: user_dir.join(AgentState::FILENAME),
            // TODO:
            // not implemented
            chat_history: user_dir.join("chat_history.json"),
            data_dir,
            user_dir
        }
    }
}
