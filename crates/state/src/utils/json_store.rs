use std::fs;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use anyhow::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::paths::UserDataPaths;

// _guard makes sure only one async task changes state.json at a time
// _guard = protection against concurrent writes to state.json.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub trait JsonFile: Serialize + DeserializeOwned + Default {
    const FILENAME: &'static str;
}

#[derive(Debug, Clone)]
pub struct JsonStore<T> {
    user_dir: PathBuf,
    file: PathBuf,
    data: PhantomData<T>,
}

// TODO: quite primitive logic,
// but good enough for the state MVP

impl<T: JsonFile> JsonStore<T> {
    pub fn new(paths: &UserDataPaths) -> Self {
        Self {
            user_dir: paths.user_dir.clone(),
            file: paths.user_dir.join(T::FILENAME),
            data: PhantomData,
        }
    }

    pub fn exists(&self) -> bool {
        self.file.exists()
    }

    pub fn load_json(&self) -> T {
        let Ok(raw) = fs::read_to_string(&self.file) else {
            return T::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn update_json(&self, delta: impl FnOnce(&mut T)) -> Result<T> {
        let _guard = WRITE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);

        let mut value = self.load_json();
        delta(&mut value);
        self.write_json_safe(&value)?;
        Ok(value)
    }

    fn write_json_safe(&self, value: &T) -> Result<()> {
        fs::create_dir_all(&self.user_dir)?;

        let body = serde_json::to_string_pretty(value)?;

        // renames rather than copies:
        // state.json.tmp becomes state.json, and no .tmp file is left behind.
        let tmp = self.file.with_extension("json.tmp");
        fs::write(&tmp, body)?;
        fs::rename(&tmp, &self.file)?;

        Ok(())
    }
}
