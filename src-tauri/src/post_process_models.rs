//! The two catalogues behind the refinement model picker.
//!
//! They are deliberately separate types rather than one with optional fields.
//! A model on this machine and a model behind an API answer different
//! questions: the first "how much disk and memory does it want", the second
//! "what does it cost me". Squeezing both into one shape would leave half the
//! fields empty on either side and force the UI to guess which half is real.

use log::debug;
use serde::{Deserialize, Serialize};
use specta::Type;

/// A model installed in Ollama.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct LocalModel {
    /// Tag as Ollama knows it, e.g. `qwen3:4b` — this is what gets configured.
    pub name: String,
    pub size_bytes: u64,
    /// e.g. "8.0B". Ollama reports it as text, and it is shown as text.
    pub parameter_size: Option<String>,
    /// e.g. "Q4_K_M".
    pub quantization: Option<String>,
    pub family: Option<String>,
}

/// A model offered through OpenRouter.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct RemoteModel {
    pub id: String,
    pub name: String,
    pub context_length: u32,
    /// Price per *token*, as the catalogue states it. Converting to something
    /// human-sized is the display's job — and for Murmel that is not "per
    /// million" but "per dictation", which only the frontend can work out
    /// because only it knows how long the user's dictations actually are.
    pub prompt_price: f64,
    pub completion_price: f64,
    pub is_free: bool,
}

/// Ollama's own endpoint, which knows more than the OpenAI-compatible one.
///
/// `/v1/models` returns bare ids; `/api/tags` adds size, parameter count and
/// quantisation — the things that decide whether a model fits on this machine.
fn tags_url(base_url: &str) -> String {
    // The configured base URL points at the OpenAI-compatible surface
    // (…:11434/v1); the native API sits next to it, not below.
    let root = base_url
        .trim_end_matches('/')
        .strip_suffix("/v1")
        .unwrap_or_else(|| base_url.trim_end_matches('/'));

    format!("{root}/api/tags")
}

pub async fn fetch_local_models(base_url: &str) -> Result<Vec<LocalModel>, String> {
    let url = tags_url(base_url);
    debug!("Fetching local models from {url}");

    let response = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|err| format!("Could not reach the local service: {err}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "The local service answered with HTTP {}.",
            response.status()
        ));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|err| format!("Could not read the model list: {err}"))?;

    let mut models: Vec<LocalModel> = body
        .get("models")
        .and_then(|models| models.as_array())
        .map(|models| {
            models
                .iter()
                .filter_map(|model| {
                    let name = model.get("name")?.as_str()?.to_string();
                    let details = model.get("details");

                    Some(LocalModel {
                        name,
                        size_bytes: model.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
                        parameter_size: details
                            .and_then(|d| d.get("parameter_size"))
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        quantization: details
                            .and_then(|d| d.get("quantization_level"))
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        family: details
                            .and_then(|d| d.get("family"))
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Largest last: the small ones are the ones worth using for refinement, and
    // they should not be buried under 20 GB of roleplay models.
    models.sort_by_key(|model| model.size_bytes);

    Ok(models)
}

pub const OPENROUTER_CATALOG_URL: &str = "https://openrouter.ai/api/v1/models";

/// Fetch OpenRouter's catalogue.
///
/// Needs no API key, which is the point: the list can be browsed — prices
/// included — before anyone signs up for anything.
pub async fn fetch_remote_models(url: &str) -> Result<Vec<RemoteModel>, String> {
    debug!("Fetching remote catalogue from {url}");

    let response = reqwest::Client::new()
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|err| format!("Could not reach OpenRouter: {err}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "OpenRouter answered with HTTP {}.",
            response.status()
        ));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|err| format!("Could not read the catalogue: {err}"))?;

    Ok(parse_remote_catalog(&body))
}

/// Prices arrive as *strings* ("0.0000025"), and a missing or unparsable one
/// must read as zero rather than dropping the model from the list.
fn price(value: Option<&serde_json::Value>) -> f64 {
    value
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0)
}

pub fn parse_remote_catalog(body: &serde_json::Value) -> Vec<RemoteModel> {
    let Some(entries) = body.get("data").and_then(|data| data.as_array()) else {
        return Vec::new();
    };

    let mut models: Vec<RemoteModel> = entries
        .iter()
        .filter_map(|entry| {
            let id = entry.get("id")?.as_str()?.to_string();
            let name = entry
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&id)
                .to_string();

            let pricing = entry.get("pricing");
            let prompt_price = price(pricing.and_then(|p| p.get("prompt")));
            let completion_price = price(pricing.and_then(|p| p.get("completion")));

            Some(RemoteModel {
                id,
                name,
                context_length: entry
                    .get("context_length")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                prompt_price,
                completion_price,
                is_free: prompt_price == 0.0 && completion_price == 0.0,
            })
        })
        .collect();

    models.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    models
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_url_sits_next_to_the_openai_surface_not_below_it() {
        assert_eq!(
            tags_url("http://localhost:11434/v1"),
            "http://localhost:11434/api/tags"
        );
        assert_eq!(
            tags_url("http://localhost:11434/v1/"),
            "http://localhost:11434/api/tags"
        );
        // A base URL without the /v1 suffix still has to work.
        assert_eq!(
            tags_url("http://localhost:11434"),
            "http://localhost:11434/api/tags"
        );
    }

    #[test]
    fn prices_arrive_as_strings() {
        let body = serde_json::json!({
            "data": [
                {
                    "id": "meta/llama-3-8b:free",
                    "name": "Llama 3 8B (free)",
                    "context_length": 8192,
                    "pricing": { "prompt": "0", "completion": "0" }
                },
                {
                    "id": "openai/gpt-4o",
                    "name": "GPT-4o",
                    "context_length": 128000,
                    "pricing": { "prompt": "0.0000025", "completion": "0.00001" }
                }
            ]
        });

        let models = parse_remote_catalog(&body);
        assert_eq!(models.len(), 2);

        let free = models.iter().find(|m| m.id.ends_with(":free")).unwrap();
        assert!(free.is_free);

        let paid = models.iter().find(|m| m.id == "openai/gpt-4o").unwrap();
        assert!(!paid.is_free);
        assert_eq!(paid.completion_price, 0.00001);
        // Output costs four times the input here — the reason Murmel shows the
        // price of a dictation rather than an input-only rate.
        assert!(paid.completion_price > paid.prompt_price);
    }

    /// A model with missing or malformed pricing must still be listed, just
    /// treated as unpriced — dropping it would hide it without explanation.
    #[test]
    fn models_without_usable_pricing_survive() {
        let body = serde_json::json!({
            "data": [
                { "id": "a/b", "name": "No pricing block" },
                { "id": "c/d", "name": "Unparsable", "pricing": { "prompt": "n/a" } }
            ]
        });

        assert_eq!(parse_remote_catalog(&body).len(), 2);
    }

    #[test]
    fn an_empty_or_unexpected_body_yields_no_models() {
        assert!(parse_remote_catalog(&serde_json::json!({})).is_empty());
        assert!(parse_remote_catalog(&serde_json::json!({ "data": "nope" })).is_empty());
    }
}
