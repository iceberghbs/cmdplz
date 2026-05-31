use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub provider_type: String,
    pub provider_url: String,
    #[serde(default)]
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_daemon_port")]
    pub daemon_port: u16,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_session_history_lines")]
    pub session_history_lines: u8,
    #[serde(default = "default_include_env_vars")]
    pub include_env_vars: bool,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_daemon_port() -> u16 {
    11435
}
fn default_hotkey() -> String {
    "Ctrl+T".into()
}
fn default_session_history_lines() -> u8 {
    10
}
fn default_include_env_vars() -> bool {
    true
}
fn default_timeout_ms() -> u64 {
    5000
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = config_path();
        let raw = fs::read_to_string(&path).map_err(|e| {
            format!(
                "Cannot read config at {}: {}. Create it with provider_type, provider_url, api_key, and model.",
                path.display(),
                e
            )
        })?;

        let cfg: Config = serde_json::from_str(&raw)
            .map_err(|e| format!("Invalid config JSON: {}", e))?;

        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.provider_url.trim().is_empty() {
            return Err("Config incomplete: missing provider_url".into());
        }
        if self.provider_type != "openai" && self.provider_type != "anthropic" {
            return Err(format!(
                "Config invalid: provider_type must be 'openai' or 'anthropic', got '{}'",
                self.provider_type
            ));
        }
        if self.provider_type == "openai" && self.api_key.trim().is_empty() {
            tracing::warn!("api_key is empty — assuming local/no-auth OpenAI-compatible provider");
        }
        if self.model.trim().is_empty() {
            return Err("Config incomplete: missing model".into());
        }
        Ok(())
    }
}

fn config_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("cmd-engine").join("config.json")
}