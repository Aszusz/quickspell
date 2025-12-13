use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

pub const STARTING_SPELL_ID: &str = "search_files";

// AppState

pub struct AppState {
    pub inner: Mutex<AppInner>,
}

// AppInner (internal state)

#[derive(Debug)]
pub struct AppInner {
    pub status: AppStatus,
    pub spells: HashMap<String, Spell>,
    pub stack: Vec<Frame>,
    pub all_items: Vec<String>,
}

// StateSnapshot

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateSnapshot {
    pub status: AppStatus,
    #[serde(rename = "noOfSpells")]
    pub no_of_spells: usize,
    pub spell_names: Vec<String>,
    pub top_items: Vec<String>,
    #[serde(rename = "totalItems")]
    pub total_items: usize,
}

// AppStatus

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppStatus {
    Booting,
    Loading,
    Ready,
    Error,
}

// Frame

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub spell_id: String,
    pub query: String,
}

// Action

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(dead_code)]
pub enum Action {
    Cmd {
        #[serde(default)]
        name: Option<String>,
        #[serde(rename = "if", default)]
        condition: Option<String>,
        cmd: String,
    },
    Spell {
        #[serde(default)]
        name: Option<String>,
        #[serde(rename = "if", default)]
        condition: Option<String>,
        spell: String,
    },
}

// Spell

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Spell {
    pub name: String,
    pub id: String,
    pub enabled: bool,
    pub provider: String,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub is_streaming: Option<bool>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub fzf_options: Vec<String>,
    #[serde(default)]
    pub actions: Vec<Action>,
}

// SpellLoadError

#[derive(Debug)]
pub enum SpellLoadError {
    ResourceNotFound(std::path::PathBuf),
    Io(std::io::Error),
    Parse {
        path: std::path::PathBuf,
        error: serde_yaml::Error,
    },
}
