use serde::Deserialize;
use std::{
    collections::HashMap,
    fmt, fs,
    path::{Path, PathBuf},
};

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

#[derive(Debug)]
pub enum SpellLoadError {
    ResourceNotFound(PathBuf),
    Io(std::io::Error),
    Parse {
        path: PathBuf,
        error: serde_yaml::Error,
    },
}

impl fmt::Display for SpellLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpellLoadError::ResourceNotFound(path) => {
                write!(f, "spells directory not found at {}", path.display())
            }
            SpellLoadError::Io(err) => write!(f, "io error while loading spells: {err}"),
            SpellLoadError::Parse { path, error } => {
                write!(f, "failed to parse {}: {error}", path.display())
            }
        }
    }
}

impl From<std::io::Error> for SpellLoadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn load_spells_from_dir(dir: &Path) -> Result<HashMap<String, Spell>, SpellLoadError> {
    if !dir.exists() {
        return Err(SpellLoadError::ResourceNotFound(dir.to_path_buf()));
    }

    let mut spells = HashMap::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        match path.extension().and_then(|ext| ext.to_str()) {
            Some("yml") | Some("yaml") => {
                let content = fs::read_to_string(&path)?;
                let spell: Spell =
                    serde_yaml::from_str(&content).map_err(|error| SpellLoadError::Parse {
                        path: path.clone(),
                        error,
                    })?;
                spells.insert(spell.id.clone(), spell);
            }
            _ => continue,
        }
    }

    Ok(spells)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_dev_spells() {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.push("resources/spells");
        let spells = load_spells_from_dir(&dir).expect("failed to load spells from dev resources");
        assert!(!spells.is_empty(), "expected at least one spell");
    }
}
