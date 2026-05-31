use crate::config::Config;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn translate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, String>;
}

pub fn build_provider(config: &Config) -> Result<Box<dyn LlmProvider>, String> {
    match config.provider_type.as_str() {
        "openai" => Ok(Box::new(OpenAiProvider::new(config)?)),
        "anthropic" => Ok(Box::new(AnthropicProvider::new(config)?)),
        other => Err(format!("Unknown provider_type: {}", other)),
    }
}

pub struct OpenAiProvider {
    client: Client,
    url: String,
    api_key: String,
    model: String,
    timeout_ms: u64,
}

impl OpenAiProvider {
    fn new(config: &Config) -> Result<Self, String> {
        let client = Client::new();
        let url = format!("{}/chat/completions", config.provider_url.trim_end_matches('/'));
        Ok(Self {
            client,
            url,
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            timeout_ms: config.timeout_ms,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn translate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, String> {
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.1,
            "max_tokens": 500
        });

        let mut req = self
            .client
            .post(&self.url)
            .json(&body)
            .timeout(std::time::Duration::from_millis(self.timeout_ms));

        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                "LLM timeout".into()
            } else if e.is_connect() {
                format!("Cannot reach provider at {}", self.url)
            } else {
                format!("Provider request failed: {}", e)
            }
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Provider returned {}: {}", status, body));
        }

        let json: Value = resp.json().await.map_err(|e| format!("Bad JSON response: {}", e))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Empty response from LLM".into())
    }
}

pub struct AnthropicProvider {
    client: Client,
    url: String,
    api_key: String,
    model: String,
    timeout_ms: u64,
}

impl AnthropicProvider {
    fn new(config: &Config) -> Result<Self, String> {
        if config.api_key.trim().is_empty() {
            return Err(
                "Config incomplete: api_key is required for Anthropic provider".into(),
            );
        }
        let client = Client::new();
        let url = format!("{}/messages", config.provider_url.trim_end_matches('/'));
        Ok(Self {
            client,
            url,
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            timeout_ms: config.timeout_ms,
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn translate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, String> {
        let body = json!({
            "model": self.model,
            "system": system_prompt,
            "messages": [
                {"role": "user", "content": user_prompt}
            ],
            "max_tokens": 500
        });

        let resp = self
            .client
            .post(&self.url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    "LLM timeout".into()
                } else if e.is_connect() {
                    format!("Cannot reach provider at {}", self.url)
                } else {
                    format!("Provider request failed: {}", e)
                }
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Provider returned {}: {}", status, body));
        }

        let json: Value = resp.json().await.map_err(|e| format!("Bad JSON response: {}", e))?;

        json["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Empty response from LLM".into())
    }
}