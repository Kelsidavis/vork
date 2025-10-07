use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

use super::conversation::Conversation;
use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub conversation: Conversation,
    pub working_directory: PathBuf,
}

impl Session {
    pub fn new(working_directory: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: format!("{}", now.timestamp()),
            created_at: now,
            updated_at: now,
            conversation: Conversation::new(),
            working_directory,
        }
    }

    pub fn sessions_dir() -> Result<PathBuf> {
        let config_dir = Config::config_dir()?;
        Ok(config_dir.join("sessions"))
    }

    pub fn session_path(&self) -> Result<PathBuf> {
        Ok(Self::sessions_dir()?.join(format!("{}.json", self.id)))
    }

    pub fn save(&mut self) -> Result<()> {
        self.updated_at = Utc::now();

        let dir = Self::sessions_dir()?;
        fs::create_dir_all(&dir)?;

        let path = self.session_path()?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;

        Ok(())
    }

    pub fn load(session_id: &str) -> Result<Self> {
        let path = Self::sessions_dir()?.join(format!("{}.json", session_id));
        let json = fs::read_to_string(path)?;
        let session = serde_json::from_str(&json)?;
        Ok(session)
    }

    pub fn list_sessions() -> Result<Vec<Session>> {
        let dir = Self::sessions_dir()?;

        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut sessions = vec![];

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(json) = fs::read_to_string(&path) {
                    if let Ok(session) = serde_json::from_str::<Session>(&json) {
                        sessions.push(session);
                    }
                }
            }
        }

        // Sort by updated_at, most recent first
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(sessions)
    }

    pub fn get_last_session() -> Result<Option<Session>> {
        let sessions = Self::list_sessions()?;
        Ok(sessions.into_iter().next())
    }
}
