// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

// Tauri commands require owned types for JSON deserialization.
#![allow(clippy::needless_pass_by_value)]

use std::{collections::HashMap, fs, path::PathBuf};

use mdd_core::tree::{DetailRowType, DiffStatus, TreeNode};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::commands::AppState;

// ---------------------------------------------------------------------------
// Persisted settings
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub ghe_host: String,
    pub llm_endpoint: String,
    pub llm_model: String,
    /// One of: copilot, azure, openai, bedrock
    pub auth_method: String,
    pub token: Option<String>,
    /// Azure API version (e.g. "2024-10-21"); only used for Azure `OpenAI`.
    #[serde(default)]
    pub api_version: Option<String>,
    /// Short-lived Copilot API key obtained via token exchange.
    #[serde(default)]
    pub copilot_token: Option<String>,
    /// Unix timestamp (seconds) when `copilot_token` expires.
    #[serde(default)]
    pub copilot_expires_at: Option<i64>,
    /// API base URL returned by the Copilot token exchange endpoint.
    #[serde(default)]
    pub copilot_api_base: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ghe_host: String::new(),
            llm_endpoint: String::new(),
            llm_model: "gpt-4o".to_owned(),
            auth_method: "copilot".to_owned(),
            token: None,
            api_version: None,
            copilot_token: None,
            copilot_expires_at: None,
            copilot_api_base: None,
        }
    }
}

/// Sent to the frontend — raw token is never exposed, only a boolean flag.
#[derive(Serialize)]
pub struct SettingsView {
    pub ghe_host: String,
    pub llm_endpoint: String,
    pub llm_model: String,
    pub auth_method: String,
    pub has_token: bool,
    pub api_version: Option<String>,
}

/// Received from the frontend to update settings.
#[derive(Deserialize)]
pub struct SettingsUpdate {
    pub ghe_host: String,
    pub llm_endpoint: String,
    pub llm_model: String,
    pub auth_method: String,
    /// API key / token for non-Copilot providers; leave None/empty to keep existing.
    pub api_token: Option<String>,
    /// Azure API version (e.g. "2024-10-21").
    pub api_version: Option<String>,
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

fn load_settings(app: &AppHandle) -> Settings {
    let Ok(path) = llm_settings_path(app) else {
        return Settings::default();
    };
    let Ok(content) = fs::read_to_string(&path) else {
        return Settings::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

fn persist_settings(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = llm_settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create cache directory: {e}"))?;
    }
    let json = serde_json::to_string(settings).map_err(|e| format!("Serialize error: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands — settings
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_llm_settings(app: AppHandle) -> SettingsView {
    let s = load_settings(&app);
    SettingsView {
        ghe_host: s.ghe_host,
        llm_endpoint: s.llm_endpoint,
        llm_model: s.llm_model,
        auth_method: s.auth_method,
        has_token: s.token.is_some(),
        api_version: s.api_version,
    }
}

#[tauri::command]
pub fn save_llm_settings(settings: SettingsUpdate, app: AppHandle) -> Result<(), String> {
    let mut current = load_settings(&app);
    current.ghe_host = settings.ghe_host;
    current.llm_endpoint = settings.llm_endpoint;
    current.llm_model = settings.llm_model;
    current.auth_method.clone_from(&settings.auth_method);
    current.api_version = settings.api_version;
    // For non-Copilot providers, store the API key/token if provided.
    match settings.auth_method.as_str() {
        "azure" | "openai" | "bedrock" => {
            if let Some(tok) = settings.api_token.filter(|t| !t.is_empty()) {
                current.token = Some(tok);
            }
        }
        _ => {} // "copilot": token is set by the device flow, not here
    }
    persist_settings(&app, &current)
}

#[tauri::command]
pub fn clear_llm_token(app: AppHandle) -> Result<(), String> {
    let mut settings = load_settings(&app);
    settings.token = None;
    settings.copilot_token = None;
    settings.copilot_expires_at = None;
    settings.copilot_api_base = None;
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
// Copilot token exchange
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CopilotTokenResponse {
    token: String,
    expires_at: i64,
    #[serde(default)]
    endpoints: HashMap<String, String>,
}

/// Exchange an OAuth access token for a short-lived Copilot API key.
async fn exchange_copilot_token(
    ghe_host: &str,
    oauth_token: &str,
) -> Result<(String, i64, Option<String>), String> {
    let client = reqwest::Client::new();
    let url = format!("https://api.{ghe_host}/copilot_internal/v2/token");
    let resp = client
        .get(&url)
        .header("Authorization", format!("token {oauth_token}"))
        .header("Accept", "application/json")
        .header("User-Agent", "mdd-ui")
        .header("Editor-Version", "vscode/1.85.1")
        .header("Editor-Plugin-Version", "copilot/1.155.0")
        .send()
        .await
        .map_err(|e| format!("Copilot token exchange failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "Copilot token exchange returned {status}: {body}\nHint: verify that Copilot is \
             enabled for your GHE account."
        ));
    }

    let data: CopilotTokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Copilot token response: {e}"))?;

    let api_base = data.endpoints.get("api").cloned();
    Ok((data.token, data.expires_at, api_base))
}

/// Ensure a valid Copilot API key is available, refreshing it if expired.
/// Returns `(copilot_api_key, api_base_url)`.
async fn ensure_copilot_key(app: &AppHandle) -> Result<(String, String), String> {
    let settings = load_settings(app);
    let oauth_token = settings
        .token
        .as_ref()
        .ok_or_else(|| "Not authenticated. Please log in first.".to_owned())?;

    // Check if existing copilot token is still valid (5 min buffer).
    if let (Some(copilot_token), Some(expires_at)) =
        (&settings.copilot_token, settings.copilot_expires_at)
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .cast_signed();
        if expires_at > now.saturating_add(300) {
            let api_base = settings
                .copilot_api_base
                .clone()
                .unwrap_or_else(|| format!("https://copilot-api.{}", settings.ghe_host));
            return Ok((copilot_token.clone(), api_base));
        }
    }

    // Token expired or missing — exchange.
    let (copilot_token, expires_at, api_base) =
        exchange_copilot_token(&settings.ghe_host, oauth_token).await?;

    let mut settings = load_settings(app);
    settings.copilot_token = Some(copilot_token.clone());
    settings.copilot_expires_at = Some(expires_at);
    if let Some(ref base) = api_base {
        settings.copilot_api_base = Some(base.clone());
    }
    persist_settings(app, &settings)?;

    let endpoint = api_base.unwrap_or_else(|| format!("https://copilot-api.{}", settings.ghe_host));
    Ok((copilot_token, endpoint))
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

    let (endpoint, auth) = if settings.auth_method == "copilot" {
        let (key, base) = ensure_copilot_key(&app).await?;
        (base, ("Authorization".to_owned(), format!("Bearer {key}")))
    } else {
        if settings.llm_endpoint.is_empty() {
            return Err("LLM endpoint not configured.".to_owned());
        }
        (settings.llm_endpoint.clone(), build_auth_header(&settings)?)
    };

    let client = reqwest::Client::new();
    let url = format!("{}/models", endpoint.trim_end_matches('/'));
    let mut req = client.get(&url).header("User-Agent", "mdd-ui");
    req = req.header(&auth.0, &auth.1);
    if settings.auth_method == "copilot" {
        req = req
            .header("Editor-Version", "vscode/1.85.1")
            .header("Editor-Plugin-Version", "copilot/1.155.0");
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
    let model = settings.llm_model.clone();

    let (endpoint, auth) = if settings.auth_method == "copilot" {
        let (key, base) = ensure_copilot_key(&app).await?;
        (base, ("Authorization".to_owned(), format!("Bearer {key}")))
    } else {
        if settings.llm_endpoint.is_empty() {
            return Err("LLM endpoint not configured. Please open settings.".to_owned());
        }
        (settings.llm_endpoint.clone(), build_auth_header(&settings)?)
    };

    // Build context from the currently loaded MDD file (drop the lock before await).
    let context = {
        let manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        manager
            .active_core()
            .ok()
            .filter(|core| !core.ecu_name.is_empty())
            .map_or_else(String::new, build_mdd_context)
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
    req = req.header(&auth.0, &auth.1);
    if settings.auth_method == "copilot" {
        req = req
            .header("Editor-Version", "vscode/1.85.1")
            .header("Editor-Plugin-Version", "copilot/1.155.0");
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
            "LLM endpoint returned an HTML page instead of JSON — this usually means the request \
             was redirected to an SSO login page. Check that your token is SAML-authorized for \
             the organization (Settings → Tokens → Authorize) and that the API Base URL is \
             correct."
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

/// Resolve auth header (name, value) for non-Copilot providers.
fn build_auth_header(settings: &Settings) -> Result<(String, String), String> {
    let token = settings.token.as_ref().ok_or_else(|| {
        "Not authenticated. Please configure authentication in settings.".to_owned()
    })?;
    match settings.auth_method.as_str() {
        "azure" => Ok(("api-key".to_owned(), token.clone())),
        "openai" | "bedrock" => Ok(("Authorization".to_owned(), format!("Bearer {token}"))),
        other => Err(format!("Unknown auth method: {other}")),
    }
}

fn append_byte_patterns(all_nodes: &[TreeNode], lines: &mut Vec<String>) {
    let mut header_added = false;
    for node in all_nodes {
        let patterns: Vec<String> = node
            .detail_sections
            .iter()
            .filter_map(|s| s.byte_pattern_rows.as_ref())
            .flat_map(|rows| {
                rows.iter()
                    .filter(|r| !matches!(r.row_type, DetailRowType::Header))
                    .map(|row| {
                        row.cells
                            .iter()
                            .map(|c| c.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" | ")
                    })
            })
            .collect();

        if patterns.is_empty() {
            continue;
        }

        if !header_added {
            lines.push(String::new());
            lines.push("Byte patterns (Offset | Bits | Hex | Binary | Name | Type):".to_owned());
            header_added = true;
        }

        lines.push(format!("\n  [[{}]]:", node.text));
        for p in &patterns {
            lines.push(format!("    {p}"));
        }
    }
}

fn build_mdd_context(core: &crate::commands::CoreState) -> String {
    const MAX_CONTEXT_CHARS: usize = 40_000;
    let mut lines: Vec<String> = Vec::new();
    lines.push("You are an expert automotive diagnostics engineer assistant.".to_owned());
    lines.push(
        "The user is viewing an MDD (Master Diagnostic Data) database in the MDD UI tool."
            .to_owned(),
    );
    lines.push(String::new());
    lines.push(
        "IMPORTANT: Only answer questions using the MDD data provided below. Do not invent, \
         assume, or hallucinate any services, parameters, or properties that are not explicitly \
         listed here. If the data does not contain enough information to answer the question, say \
         so clearly. Markdown is fully supported in your responses — use headings, bold, lists, \
         and code blocks where appropriate."
            .to_owned(),
    );
    lines.push(String::new());
    lines.push(
        "When referencing any node, service, parameter, or diagnostic object by name, always wrap \
         it in double square brackets, e.g. [[ServiceName]] or [[ParameterName]]. Copy the name \
         character-for-character exactly as it appears in the MDD structure below — do not \
         rephrase, shorten, or change capitalisation. This allows the user to click on them for \
         direct navigation in the UI."
            .to_owned(),
    );
    lines.push(String::new());
    lines.push(format!("ECU: {}", core.ecu_name));
    lines.push(format!("Total nodes: {}", core.all_nodes.len()));
    lines.push(String::new());

    if core.is_diff_mode {
        lines.push(
            "MDD diff (only changed nodes shown with ancestors for context; + added, - removed, ~ \
             modified):"
                .to_owned(),
        );

        // Determine which nodes to show: changed nodes + their ancestors.
        // This mirrors the MCP diff_mdd tool which annotates the diff tree and
        // supports max_depth filtering — here we filter by diff status instead.
        let node_count = core.all_nodes.len();
        let mut show = vec![false; node_count];

        // Pass 1: mark all changed (non-Unchanged) nodes.
        for (i, node) in core.all_nodes.iter().enumerate() {
            if !matches!(node.diff_status, Some(DiffStatus::Unchanged) | None)
                && let Some(slot) = show.get_mut(i)
            {
                *slot = true;
            }
        }

        // Pass 2: mark ancestors of changed nodes so the LLM has path context.
        let max_depth = core.all_nodes.iter().map(|n| n.depth).max().unwrap_or(0);
        let mut parent_at_depth = vec![0usize; max_depth.saturating_add(1)];
        for (i, node) in core.all_nodes.iter().enumerate() {
            if let Some(slot) = parent_at_depth.get_mut(node.depth) {
                *slot = i;
            }
            if show.get(i).copied().unwrap_or(false) && node.depth > 0 {
                for d in (0..node.depth).rev() {
                    let Some(&ancestor) = parent_at_depth.get(d) else {
                        break;
                    };
                    if show.get(ancestor).copied().unwrap_or(false) {
                        break;
                    }
                    if let Some(slot) = show.get_mut(ancestor) {
                        *slot = true;
                    }
                }
            }
        }

        // Emit visible nodes with diff markers.
        for (i, node) in core.all_nodes.iter().enumerate() {
            if !show.get(i).copied().unwrap_or(false) {
                continue;
            }
            let diff_marker = match node.diff_status {
                Some(DiffStatus::Added) => "+ ",
                Some(DiffStatus::Removed) => "- ",
                Some(DiffStatus::Modified) => "~ ",
                Some(DiffStatus::Unchanged) | None => "  ",
            };
            let indent = "  ".repeat(node.depth);
            lines.push(format!(
                "{diff_marker}{indent}[{:?}] {}",
                node.node_type, node.text
            ));
        }
    } else {
        lines.push("MDD structure (containers, services, and sub-services):".to_owned());
        for node in &core.all_nodes {
            if node.depth <= 3 {
                let indent = "  ".repeat(node.depth);
                lines.push(format!("{indent}- [{:?}] {}", node.node_type, node.text));
            }
        }
    }

    append_byte_patterns(&core.all_nodes, &mut lines);

    // Apply a character budget to prevent token-limit errors regardless of database size.
    let result = lines.join("\n");
    if result.len() > MAX_CONTEXT_CHARS {
        let cut = result
            .char_indices()
            .map(|(i, _)| i)
            .nth(MAX_CONTEXT_CHARS)
            .unwrap_or(result.len());
        let mut truncated = result.get(..cut).unwrap_or(&result).to_owned();
        truncated.push_str(
            "\n\n[Context truncated — database too large to fit in one request. Ask specific \
             questions about services or nodes by name.]",
        );
        truncated
    } else {
        result
    }
}
