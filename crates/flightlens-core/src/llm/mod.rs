//! Pure preparation and decoding for optional summaries. No transport or storage.
mod digest;
mod ipc;
mod prompt;
mod provider;
mod settings;

pub use digest::*;
pub use ipc::*;
pub use prompt::{diff_prompt, inspector_prompt};

/// Bump whenever prompt text or its interpretation changes.
pub const PROMPT_VERSION: u32 = 1;

pub use provider::{build_request, parse_response, HttpRequest, Provider, Wire};
pub use settings::{validate_base_url, LlmSettings};

/// Prompt contents are deliberately absent from Debug output.
#[derive(Clone, PartialEq, Eq)]
pub struct Prompt {
    pub system: String,
    pub user: String,
}

impl std::fmt::Debug for Prompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Prompt { contents: [redacted] }")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Summary {
    pub text: String,
}

impl std::fmt::Debug for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Summary { text: [redacted] }")
    }
}

/// Errors contain no provider messages, URLs, keys, prompts or response bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmError {
    InvalidBaseUrl,
    InvalidModel,
    MissingKey,
    InvalidKey,
    RejectedKey,
    ModelNotFound,
    RateLimited,
    ProviderUnavailable,
    RequestRejected,
    InvalidResponse,
    EmptyResponse,
    Blocked,
    Truncated,
    Incomplete,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidBaseUrl => "Use an absolute HTTPS base URL, or HTTP on localhost or a loopback IP, without credentials, query or fragment.",
            Self::InvalidModel => "Enter a valid model identifier for the selected provider.",
            Self::MissingKey => "Configure an API key for this provider.",
            Self::InvalidKey => "The API key contains invalid header characters.",
            Self::RejectedKey => "The provider rejected the API key or denied access.",
            Self::ModelNotFound => "The model or endpoint was not found. Check the model identifier; it may have been renamed.",
            Self::RateLimited => "The provider rate limited this request. Try again later.",
            Self::ProviderUnavailable => "The provider is unavailable. Try again later.",
            Self::RequestRejected => "The provider rejected the request. Check the model and its support for temperature and max_tokens parameters.",
            Self::InvalidResponse => "The provider returned an invalid summary response.",
            Self::EmptyResponse => "The provider returned no summary text.",
            Self::Blocked => "The provider blocked the summary.",
            Self::Truncated => "The summary reached the provider's output limit and is incomplete.",
            Self::Incomplete => "The provider stopped before completing the summary.",
        })
    }
}

impl std::error::Error for LlmError {}
