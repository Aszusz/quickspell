use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

pub const STARTING_SPELL_ID: &str = "quickspell";

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
    pub next_frame_id: u64,
}

// StateSnapshot

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateSnapshot {
    pub status: AppStatus,
    #[serde(rename = "noOfSpells")]
    pub no_of_spells: usize,
    pub spell_names: Vec<String>,
    pub top_items: Vec<Item>,
    pub query: String,
    #[serde(rename = "isFiltering")]
    pub is_filtering: bool,
    #[serde(rename = "selectedIndex")]
    pub selected_index: usize,
    #[serde(rename = "selectedItem")]
    pub selected_item: Option<Item>,
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
    pub id: u64,
    pub spell_id: String,
    pub query: String,
    pub all_items: Vec<Item>,
    pub filtered_items: Vec<Item>,
    pub is_filtering: bool,
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

// Item

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Item {
    #[serde(rename = "Type")]
    pub item_type: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Data")]
    pub data: String,
}

impl Item {
    pub fn from_line(line: &str) -> Option<Self> {
        let mut fields = line.split('\t');
        let item_type = fields.next()?;
        let name = fields.next()?;
        let data = fields.next()?;

        Some(Self {
            item_type: item_type.to_string(),
            name: name.to_string(),
            data: data.to_string(),
        })
    }

    pub fn field(&self, idx: usize) -> &str {
        match idx {
            0 => &self.item_type,
            1 => &self.name,
            2 => &self.data,
            _ => &self.name,
        }
    }

    pub fn raw(&self) -> String {
        format!("{}\t{}\t{}", self.item_type, self.name, self.data)
    }
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
