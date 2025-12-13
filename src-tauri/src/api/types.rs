use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

pub const STARTING_SPELL_ID: &str = "search_files";

// AppState

#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<RwLock<AppInner>>,
}

// AppInner (internal state)

#[derive(Debug)]
pub struct AppInner {
    pub status: AppStatus,
    pub spells: HashMap<String, Spell>,
    pub stack: Vec<Frame>,
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
    pub query: String,
    #[serde(rename = "selectedIndex")]
    pub selected_index: usize,
    #[serde(rename = "selectedItem")]
    pub selected_item: Option<String>,
    #[serde(rename = "totalItems")]
    pub total_items: usize,
}

// AppStatus

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppStatus {
    NotStarted,
    Booting,
    Loading,
    Ready,
    Error,
}

// Frame

#[derive(Debug, Clone)]
pub struct Frame {
    pub spell_id: String,
    pub query: String,
    pub all_items: Vec<String>,
    pub filtered_items: Vec<String>,
    pub selected_idx: usize,
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

// SearchConfig

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchScheme {
    #[default]
    Plain,
    Path,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    #[default]
    Fuzzy,
    Exact,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_field")]
    pub field: usize, // 1-indexed
    #[serde(default)]
    pub scheme: SearchScheme,
    #[serde(default)]
    pub mode: SearchMode,
}

fn default_field() -> usize {
    1
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            field: 1,
            scheme: SearchScheme::Plain,
            mode: SearchMode::Fuzzy,
        }
    }
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
    pub search: Option<SearchConfig>,
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
