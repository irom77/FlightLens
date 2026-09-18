use super::{LlmError, LlmSettings, Prompt, Summary};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Gemini,
    Openai,
    Openrouter,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    Gemini,
    OpenAiCompatible,
}

impl Provider {
    pub fn wire(self) -> Wire {
        match self {
            Self::Gemini => Wire::Gemini,
            _ => Wire::OpenAiCompatible,
        }
    }

    /// Model documentation and selection evidence: docs/llm-summaries.md.
    /// Custom is deliberately empty: only the user knows which model is installed.
    pub fn default_model(self) -> &'static str {
        match self {
            Self::Gemini => "gemini-3.8-flash",
            Self::Openai => "gpt-4.1-mini",
            Self::Openrouter => "openai/gpt-4.1-mini",
            Self::Custom => "",
        }
    }
}

pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl std::fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not log even non-auth header values or user-supplied URL paths.
        f.debug_struct("HttpRequest")
            .field("url", &"[redacted]")
            .field("headers", &"[redacted]")
            .field("body", &"[redacted]")
            .finish()
    }
}

/// Prepares bytes only. The shell enforces enabled state, user consent and transport.
pub fn build_request(
    settings: &LlmSettings,
    key: Option<&str>,
    prompt: &Prompt,
) -> Result<HttpRequest, LlmError> {
    let settings = settings.validated()?;
    let key = key.filter(|k| !k.is_empty());
    if settings.provider != Provider::Custom && key.is_none() {
        return Err(LlmError::MissingKey);
    }
    if key.is_some_and(|k| !k.bytes().all(|c| (33..=126).contains(&c))) {
        return Err(LlmError::InvalidKey);
    }
    let base = match settings.provider {
        Provider::Gemini => "https://generativelanguage.googleapis.com",
        Provider::Openai => "https://api.openai.com",
        Provider::Openrouter => "https://openrouter.ai/api",
        Provider::Custom => settings
            .base_url
            .as_deref()
            .ok_or(LlmError::InvalidBaseUrl)?,
    };
    let mut headers = vec![("Content-Type".into(), "application/json".into())];
    if let Some(key) = key {
        headers.push(match settings.provider {
            Provider::Gemini => ("x-goog-api-key".into(), key.into()),
            _ => ("Authorization".into(), format!("Bearer {key}")),
        });
    }
    if settings.provider == Provider::Openrouter {
        headers.push(("X-Title".into(), "FlightLens".into()));
    }
    let (url, body) = match settings.provider.wire() {
        // Gemini counts internal thinking against this same output budget.
        // Leave room for both reasoning and the concise visible summary.
        Wire::Gemini => (
            format!("{base}/v1beta/models/{}:generateContent", settings.model),
            json!({
                "systemInstruction": { "parts": [{ "text": prompt.system }] },
                "contents": [{ "role": "user", "parts": [{ "text": prompt.user }] }],
                "generationConfig": { "temperature": 0.2, "maxOutputTokens": 8192 }
            }),
        ),
        Wire::OpenAiCompatible => (
            format!("{base}/v1/chat/completions"),
            json!({
                "model": settings.model,
                "messages": [{ "role": "system", "content": prompt.system }, { "role": "user", "content": prompt.user }],
                "temperature": 0.2,
                "max_tokens": 1024
            }),
        ),
    };
    Ok(HttpRequest {
        url,
        headers,
        body: body.to_string(),
    })
}

pub fn parse_response(provider: Provider, status: u16, body: &str) -> Result<Summary, LlmError> {
    if !(200..300).contains(&status) {
        return Err(match status {
            401 | 403 => LlmError::RejectedKey,
            404 => LlmError::ModelNotFound,
            429 => LlmError::RateLimited,
            500..=599 => LlmError::ProviderUnavailable,
            _ => LlmError::RequestRejected,
        });
    }
    // The shell also bounds streaming reads before allocating the response body.
    if body.len() > 1_048_576 {
        return Err(LlmError::InvalidResponse);
    }
    let data: Value = serde_json::from_str(body).map_err(|_| LlmError::InvalidResponse)?;
    if data.get("error").is_some_and(|value| !value.is_null()) {
        return Err(LlmError::RequestRejected);
    }
    let text = match provider.wire() {
        Wire::Gemini => {
            if data
                .pointer("/promptFeedback/blockReason")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty() && s != "BLOCK_REASON_UNSPECIFIED")
            {
                return Err(LlmError::Blocked);
            }
            let candidate = data
                .pointer("/candidates/0")
                .ok_or(LlmError::EmptyResponse)?;
            match candidate.get("finishReason").and_then(Value::as_str) {
                Some("STOP") => (),
                Some("MAX_TOKENS") => return Err(LlmError::Truncated),
                Some(
                    "SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII"
                    | "IMAGE_SAFETY",
                ) => return Err(LlmError::Blocked),
                _ => return Err(LlmError::Incomplete),
            }
            let parts = candidate
                .pointer("/content/parts")
                .and_then(Value::as_array)
                .ok_or(LlmError::EmptyResponse)?;
            let mut text = String::new();
            for part in parts {
                // Thought parts are not summary output.
                if part.get("thought").and_then(Value::as_bool) != Some(true) {
                    if let Some(value) = part.get("text") {
                        text.push_str(value.as_str().ok_or(LlmError::InvalidResponse)?);
                    }
                }
            }
            text
        }
        Wire::OpenAiCompatible => {
            let choice = data.pointer("/choices/0").ok_or(LlmError::EmptyResponse)?;
            match choice.get("finish_reason").and_then(Value::as_str) {
                Some("stop") => (),
                Some("length") => return Err(LlmError::Truncated),
                Some("content_filter") => return Err(LlmError::Blocked),
                _ => return Err(LlmError::Incomplete),
            }
            if choice
                .pointer("/message/refusal")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())
            {
                return Err(LlmError::Blocked);
            }
            choice
                .pointer("/message/content")
                .and_then(Value::as_str)
                .ok_or(LlmError::EmptyResponse)?
                .to_owned()
        }
    };
    if text.trim().is_empty() {
        return Err(LlmError::EmptyResponse);
    }
    Ok(Summary { text })
}
