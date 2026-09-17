//! Non-secret shell/renderer contract. Credentials are write-only IPC arguments.
use super::{DiffRowInput, LlmSettings, Provider, Truncation};
use crate::feedback::Redaction;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SummaryRequest {
    Inspector {
        config_id: String,
        rate_profile: u8,
        pid_profile: u8,
    },
    Diff {
        slots: Vec<SummarySlot>,
        baseline: Option<usize>,
        rows: Vec<DiffRowInput>,
    },
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SummarySlot {
    pub config_id: String,
    pub rate_profile: u8,
    pub pid_profile: u8,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LlmStatus {
    pub settings: LlmSettings,
    pub has_key: bool,
    pub key_hint: Option<String>,
    pub session_only: bool,
    pub credential_problem: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LlmPreview {
    pub system_prompt: String,
    pub user_prompt: String,
    pub payload_json: String,
    pub excluded: Vec<Redaction>,
    pub label_map: Vec<(String, String)>,
    pub bytes: usize,
    pub truncated: Option<Truncation>,
    pub problems: Vec<String>,
    pub destination: String,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LlmSummary {
    pub text: String,
    pub provider: Provider,
    pub model: String,
    pub prompt_version: u32,
    // Unix seconds as a decimal string; renderer formats local time for display.
    pub generated_at: String,
    pub cached: bool,
}
