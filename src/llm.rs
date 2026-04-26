// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

// Tauri commands require owned types; struct names intentionally mirror the module.
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::module_name_repetitions)]

use crate::commands::AppState;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager, State};

// ---------------------------------------------------------------------------
// Persisted settings
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
pub struct LlmSettings {
    pub ghe_host: String,
    pub client_id: String,
    pub llm_endpoint: String,
    pub llm_model: String,
    pub token: Option<String>,
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            ghe_host: String::new(),
            client_id: String::new(),
            llm_endpoint: String::new(),
            llm_model: "gpt-4o".to_owned(),
            token: None,
        }
    }
}

/// Sent to the frontend — token is never exposed, only a boolean flag.
#[derive(Serialize)]
pub struct LlmSettingsView {
    pub ghe_host: String,
    pub client_id: String,
    pub llm_endpoint: String,
    pub llm_model: String,
    pub has_token: bool,
}

/// Received from the frontend to update settings (token managed separately).
#[derive(Deserialize)]
pub struct LlmSettingsUpdate {
    pub ghe_host: String,
    pub client_id: String,
    pub llm_endpoint: String,
    pub llm_model: String,
}

// ---------------------------------------------------------------------------
// Settings persistence helpers
// ---------------------------------------------------------------------------

fn llm_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let cache_dir = app
        .path()
        .cache_dir()
        .map_err(|e| format!("Failed to get cache directory: {e}"))?;
    Ok(cache_dir.join("mdd-ui").join("llm-settings.json"))
}

fn load_settings(app: &AppHandle) -> LlmSettings {
    let Ok(path) = llm_settings_path(app) else {
        return LlmSettings::default();
    };
    let Ok(content) = fs::read_to_string(&path) else {
        return LlmSettings::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

fn persist_settings(app: &AppHandle, settings: &LlmSettings) -> Result<(), String> {
    let path = llm_settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create cache directory: {e}"))?;
    }
    let json =
        serde_json::to_string(settings).map_err(|e| format!("Serialize error: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands — settings
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_llm_settings(app: AppHandle) -> LlmSettingsView {
    let s = load_settings(&app);
    LlmSettingsView {
        ghe_host: s.ghe_host,
        client_id: s.client_id,
        llm_endpoint: s.llm_endpoint,
        llm_model: s.llm_model,
        has_token: s.token.is_some(),
    }
}

#[tauri::command]
pub fn save_llm_settings(
    settings: LlmSettingsUpdate,
    app: AppHandle,
) -> Result<(), String> {
    let mut current = load_settings(&app);
    current.ghe_host = settings.ghe_host;
    current.client_id = settings.client_id;
    current.llm_endpoint = settings.llm_endpoint;
    current.llm_model = settings.llm_model;
    persist_settings(&app, &current)
}

#[tauri::command]
pub fn clear_llm_token(app: AppHandle) -> Result<(), String> {
    let mut settings = load_settings(&app);
    settings.token = None;
    persist_settings(&app, &settings)
}

// ---------------------------------------------------------------------------
// GitHub Enterprise Device Flow
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct DeviceFlowStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Deserialize)]
struct GheDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[tauri::command]
pub async fn start_ghe_device_flow(
    ghe_host: String,
    client_id: String,
) -> Result<DeviceFlowStart, String> {
    let client = reqwest::Client::new();
    let url = format!("https://{ghe_host}/login/device/code");
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .form(&[("client_id", client_id.as_str()), ("scope", "read:user")])
        .send()
        .await
        .map_err(|e| format!("Device flow request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("GHE returned {status}: {body}"));
    }

    let data: GheDeviceCodeResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse device flow response: {e}"))?;

    Ok(DeviceFlowStart {
        device_code: data.device_code,
        user_code: data.user_code,
        verification_uri: data.verification_uri,
        expires_in: data.expires_in,
        interval: data.interval,
    })
}

#[derive(Serialize)]
pub struct PollResult {
    pub status: String,
}

#[derive(Deserialize)]
struct GheTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

#[tauri::command]
pub async fn poll_ghe_device_flow(
    ghe_host: String,
    client_id: String,
    device_code: String,
    app: AppHandle,
) -> Result<PollResult, String> {
    let client = reqwest::Client::new();
    let url = format!("https://{ghe_host}/login/oauth/access_token");
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("device_code", device_code.as_str()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .map_err(|e| format!("Poll request failed: {e}"))?;

    let data: GheTokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {e}"))?;

    if let Some(token) = data.access_token {
        let mut settings = load_settings(&app);
        settings.token = Some(token);
        persist_settings(&app, &settings)?;
        return Ok(PollResult {
            status: "authorized".to_owned(),
        });
    }

    let status = match data.error.as_deref() {
        Some("authorization_pending") => "pending",
        Some("slow_down") => "slow_down",
        Some("expired_token") => "expired",
        _ => "error",
    };
    Ok(PollResult {
        status: status.to_owned(),
    })
}

// ---------------------------------------------------------------------------
// Available models
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

#[tauri::command]
pub async fn fetch_llm_models(app: AppHandle) -> Result<Vec<String>, String> {
    let settings = load_settings(&app);
    let token = settings
        .token
        .ok_or_else(|| "Not authenticated.".to_owned())?;
    if settings.llm_endpoint.is_empty() {
        return Err("LLM endpoint not configured.".to_owned());
    }
    let client = reqwest::Client::new();
    let url = format!("{}/models", settings.llm_endpoint.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Models request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Models API returned {status}: {body}"));
    }

    let data: OpenAiModelsResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse models response: {e}"))?;

    let mut ids: Vec<String> = data.data.into_iter().map(|m| m.id).collect();
    ids.sort();
    Ok(ids)
}

// ---------------------------------------------------------------------------
// LLM Chat
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatResult {
    pub content: String,
}

#[derive(Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[tauri::command]
pub async fn llm_chat(
    messages: Vec<ChatMessage>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ChatResult, String> {
    let settings = load_settings(&app);
    let token = settings
        .token
        .ok_or_else(|| "Not authenticated. Please log in with GitHub Enterprise.".to_owned())?;
    let endpoint = settings.llm_endpoint;
    let model = settings.llm_model;

    if endpoint.is_empty() {
        return Err("LLM endpoint not configured. Please open settings.".to_owned());
    }

    // Build context from the currently loaded MDD file (drop the lock before await).
    let context = {
        let core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        if core.ecu_name.is_empty() {
            String::new()
        } else {
            build_mdd_context(&core)
        }
    };

    let mut all_messages: Vec<ChatMessage> = Vec::new();
    if !context.is_empty() {
        all_messages.push(ChatMessage {
            role: "system".to_owned(),
            content: context,
        });
    }
    all_messages.extend(messages);

    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
    let body = OpenAiRequest {
        model: &model,
        messages: all_messages,
        stream: false,
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err(format!("LLM API returned {status}: {body_text}"));
    }

    let data: OpenAiResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse LLM response: {e}"))?;

    let content = data
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .unwrap_or_default();

    Ok(ChatResult { content })
}

fn build_mdd_context(core: &crate::commands::CoreState) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(
        "You are an expert automotive diagnostics engineer assistant.".to_owned(),
    );
    lines.push(
        "The user is viewing an MDD (Master Diagnostic Data) database in the MDD UI tool."
            .to_owned(),
    );
    lines.push(String::new());
    lines.push(format!("ECU: {}", core.ecu_name));
    lines.push(format!("Total nodes: {}", core.all_nodes.len()));
    lines.push(String::new());
    lines.push("Structure overview (top-level containers and services):".to_owned());
    for node in &core.all_nodes {
        if node.depth <= 1 {
            let indent = "  ".repeat(node.depth);
            lines.push(format!("{indent}- {}", node.text));
        }
    }
    lines.join("\n")
}
