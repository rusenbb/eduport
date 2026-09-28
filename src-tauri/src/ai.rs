use serde::{Deserialize, Serialize};

use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub trust_mode: String,
    #[serde(default)]
    pub delegated_types: Vec<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".into(),
            model: String::new(),
            trust_mode: "controlled".into(),
            delegated_types: Vec::new(),
        }
    }
}

impl AiConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.provider.as_str(), "ollama" | "openrouter") {
            return Err("Choose Ollama or OpenRouter.".into());
        }
        if self.model.trim().is_empty() || self.model.len() > 120 {
            return Err("Choose a model name.".into());
        }
        if !matches!(self.trust_mode.as_str(), "controlled" | "delegated") {
            return Err("Unknown trust mode.".into());
        }
        Ok(())
    }
}

pub fn load_config(db: &Db) -> Result<AiConfig, String> {
    match db.get_setting("ai_config").map_err(|e| e.to_string())? {
        Some(value) => serde_json::from_value(value).map_err(|e| e.to_string()),
        None => Ok(AiConfig::default()),
    }
}

pub fn save_config(db: &mut Db, config: &AiConfig) -> Result<(), String> {
    config.validate()?;
    let known: std::collections::HashSet<String> = db
        .list_types()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|kind| kind.key)
        .collect();
    if config
        .delegated_types
        .iter()
        .any(|kind| !known.contains(kind))
    {
        return Err("Unrestricted permission references an unknown entity type.".into());
    }
    db.put_setting(
        "ai_config",
        &serde_json::to_value(config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn save_openrouter_key(key: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("API key is empty.".into());
    }
    let entry = keyring::Entry::new("io.github.rusenbb.eduport", "openrouter")
        .map_err(|e| e.to_string())?;
    entry.set_password(key.trim()).map_err(|e| e.to_string())
}

pub fn has_openrouter_key() -> Result<bool, String> {
    let entry = keyring::Entry::new("io.github.rusenbb.eduport", "openrouter")
        .map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn get_openrouter_key() -> Result<String, String> {
    let entry = keyring::Entry::new("io.github.rusenbb.eduport", "openrouter")
        .map_err(|e| e.to_string())?;
    entry
        .get_password()
        .map_err(|_| "Set an OpenRouter API key in Settings first.".to_string())
}
