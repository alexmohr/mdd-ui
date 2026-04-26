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
    pub auth_method: String,
    pub token: Option<String>,
}

impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            ghe_host: String::new(),
            client_id: String::new(),
            llm_endpoint: String::new(),
            llm_model: "gpt-4o".to_owned(),
            auth_method: "ghe".to_owned(),
            token: None,
        }
    }
}

/// Sent to the frontend — raw token is never exposed, only a boolean flag.
#[derive(Serialize)]
pub struct LlmSettingsView {
    pub ghe_host: String,
    pub client_id: String,
    pub llm_endpoint: String,
    pub llm_model: String,
    pub auth_method: String,
    pub has_token: bool,
}

/// Received from the frontend to update settings.
#[derive(Deserialize)]
pub struct LlmSettingsUpdate {
    pub ghe_host: String,
    pub client_id: String,
    pub llm_endpoint: String,
    pub llm_model: String,
    pub auth_method: String,
    /// Only used for auth_method == "token"; leave None/empty to keep existing token.
    pub api_token: Option<String>,
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
        auth_method: s.auth_method,
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
    current.auth_method = settings.auth_method.clone();
    if settings.auth_method == "token" {
        if let Some(tok) = settings.api_token.filter(|t| !t.is_empty()) {
            current.token = Some(tok);
        }
    } else if settings.auth_method == "none" {
        current.token = None;
    }
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
        .header("Content-Type", "application/json")
        .header("User-Agent", "mdd-ui")
        .json(&serde_json::json!({"client_id": client_id, "scope": "read:user"}))
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
        .header("Content-Type", "application/json")
        .header("User-Agent", "mdd-ui")
        .json(&serde_json::json!({
            "client_id": client_id,
            "device_code": device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
        }))
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
    if settings.llm_endpoint.is_empty() {
        return Err("LLM endpoint not configured.".to_owned());
    }
    let auth_header = build_auth_header(&settings)?;
    let client = reqwest::Client::new();
    let url = format!("{}/models", settings.llm_endpoint.trim_end_matches('/'));
    let mut req = client.get(&url).header("User-Agent", "mdd-ui");
    if let Some(h) = auth_header {
        req = req.header("Authorization", h);
    }
    let resp = req
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
    let endpoint = settings.llm_endpoint.clone();
    let model = settings.llm_model.clone();

    if endpoint.is_empty() {
        return Err("LLM endpoint not configured. Please open settings.".to_owned());
    }

    let auth_header = build_auth_header(&settings)?;

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

    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "mdd-ui")
        .header("Openai-Intent", "conversation-edits")
        .header("x-initiator", "user")
        .json(&body);
    if let Some(h) = auth_header {
        req = req.header("Authorization", h);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err(format!("LLM API returned {status}: {body_text}"));
    }

    let body_text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read LLM response body: {e}"))?;

    if body_text.trim_start().starts_with('<') {
        return Err(
            "LLM endpoint returned an HTML page instead of JSON — \
             this usually means the request was redirected to an SSO login page. \
             Check that your token is SAML-authorized for the organization \
             (Settings → Tokens → Authorize) and that the API Base URL is correct."
                .to_owned(),
        );
    }

    let data: OpenAiResponse = serde_json::from_str(&body_text)
        .map_err(|e| format!("Failed to parse LLM response: {e}\nRaw body: {body_text}"))?;

    let content = data
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .unwrap_or_default();

    Ok(ChatResult { content })
}

#[tauri::command]
pub async fn import_gh_cli_token(ghe_host: String, app: AppHandle) -> Result<(), String> {
    // Fast path: gh is already authenticated — grab the stored token.
    let token = gh_get_token(&ghe_host).await.ok().flatten();

    let token = if let Some(t) = token {
        t
    } else {
        // Slow path: open the browser for gh auth login (handles SAML SSO, no app ID needed).
        let host = ghe_host.clone();
        let status = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new("gh")
                .args([
                    "auth",
                    "login",
                    "--hostname",
                    &host,
                    "--git-protocol",
                    "https",
                    "--web",
                ])
                .status()
        })
        .await
        .map_err(|e| format!("Task error: {e}"))?
        .map_err(|e| format!("gh CLI not found: {e}. Install from https://cli.github.com"))?;

        if !status.success() {
            return Err(format!(
                "gh auth login failed. Try manually: gh auth login --hostname {ghe_host} --web"
            ));
        }

        gh_get_token(&ghe_host)
            .await?
            .ok_or_else(|| "gh auth login succeeded but returned no token.".to_owned())?
    };

    let mut settings = load_settings(&app);
    settings.token = Some(token);
    persist_settings(&app, &settings)
}

async fn gh_get_token(ghe_host: &str) -> Result<Option<String>, String> {
    let host = ghe_host.to_owned();
    let output = tauri::async_runtime::spawn_blocking(move || {
        std::process::Command::new("gh")
            .args(["auth", "token", "--hostname", &host])
            .output()
    })
    .await
    .map_err(|e| format!("Task error: {e}"))?
    .map_err(|e| format!("gh CLI not found: {e}. Install from https://cli.github.com"))?;

    if !output.status.success() {
        return Ok(None);
    }
    let t = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok(if t.is_empty() { None } else { Some(t) })
}

fn build_auth_header(settings: &LlmSettings) -> Result<Option<String>, String> {
    match settings.auth_method.as_str() {
        "none" => Ok(None),
        _ => {
            let token = settings
                .token
                .as_ref()
                .ok_or_else(|| "Not authenticated. Please configure authentication in settings.".to_owned())?;
            Ok(Some(format!("Bearer {token}")))
        }
    }
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
    lines.push(
        "IMPORTANT: Only answer questions using the MDD data provided below. \
        Do not invent, assume, or hallucinate any services, parameters, or properties \
        that are not explicitly listed here. If the data does not contain enough information \
        to answer the question, say so clearly. \
        Markdown is fully supported in your responses — use headings, bold, lists, and code blocks where appropriate."
            .to_owned(),
    );
    lines.push(String::new());
    lines.push(
        "When referencing any node, service, parameter, or diagnostic object by name, \
        always wrap it in double square brackets, e.g. [[ServiceName]] or [[ParameterName]]. \
        Copy the name character-for-character exactly as it appears in the MDD structure below — \
        do not rephrase, shorten, or change capitalisation. \
        This allows the user to click on them for direct navigation in the UI."
            .to_owned(),
    );
    lines.push(String::new());
    lines.push(format!("ECU: {}", core.ecu_name));
    lines.push(format!("Total nodes: {}", core.all_nodes.len()));
    lines.push(String::new());
    lines.push("MDD structure (containers, services, and sub-services):".to_owned());
    for node in &core.all_nodes {
        if node.depth <= 3 {
            let indent = "  ".repeat(node.depth);
            lines.push(format!("{indent}- [{:?}] {}", node.node_type, node.text));
        }
    }
    lines.join("\n")
}
