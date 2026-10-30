//! Cloud speech-to-text and OpenAI-compatible chat, with key rotation and readable errors.

use anyhow::{anyhow, Result};
use base64::Engine;
use serde_json::{json, Value};
use std::time::Duration;

use crate::settings::ProviderSettings;

pub fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(90))
        .user_agent(concat!("FlowVoice/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("http client")
}

pub fn display_name(provider: &str) -> &'static str {
    match provider {
        "groq" => "Groq",
        "gemini" => "Gemini",
        "openai" => "OpenAI",
        "openrouter" => "OpenRouter",
        "mistral" => "Mistral",
        "ollama" => "Ollama",
        "custom" => "Personalizado",
        "tavily" => "Tavily",
        "local" => "Whisper local",
        _ => "IA",
    }
}

pub fn base_url(provider: &str, cfg: &ProviderSettings) -> String {
    let url = match provider {
        "groq" => "https://api.groq.com/openai/v1".to_string(),
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
        "openai" => "https://api.openai.com/v1".to_string(),
        "openrouter" => "https://openrouter.ai/api/v1".to_string(),
        "mistral" => "https://api.mistral.ai/v1".to_string(),
        "ollama" if cfg.base_url.trim().is_empty() => "http://localhost:11434/v1".to_string(),
        _ => cfg.base_url.clone(),
    };
    url.trim_end_matches('/').to_string()
}

/// Error from a provider call, with a short user-facing message and the raw detail.
#[derive(Debug)]
pub struct ProviderError {
    pub provider: String,
    pub message: String,
    pub detail: String,
    /// Worth trying the next key (auth/limit) — otherwise go to the next provider.
    pub key_problem: bool,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", display_name(&self.provider), self.message)
    }
}

fn friendly(provider: &str, status: u16, body: &str) -> ProviderError {
    let api_msg = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| {
            v.pointer("/error/message")
                .or_else(|| v.pointer("/0/error/message"))
                .or_else(|| v.get("message"))
                .or_else(|| v.get("detail"))
                .and_then(|m| m.as_str().map(str::to_string))
        })
        .unwrap_or_else(|| body.chars().take(300).collect());
    let lower = api_msg.to_lowercase();
    let (message, key_problem) = match status {
        401 | 403 => ("chave inválida, expirada ou sem permissão".to_string(), true),
        429 if lower.contains("credit") || lower.contains("billing") || lower.contains("quota") => {
            ("sem créditos ou cota esgotada nesta conta".to_string(), true)
        }
        429 => ("limite de uso atingido (tente de novo em instantes ou adicione outra chave)".to_string(), true),
        402 => ("sem créditos na conta".to_string(), true),
        404 => ("modelo não encontrado — confira o nome do modelo nas configurações".to_string(), false),
        400 if lower.contains("api key") || lower.contains("api_key") => ("chave inválida".to_string(), true),
        400 if lower.contains("model") => (format!("modelo recusado: {api_msg}"), false),
        413 => ("áudio grande demais para este provedor".to_string(), false),
        500..=599 => ("o serviço está instável agora".to_string(), false),
        _ => (api_msg.clone(), false),
    };
    ProviderError { provider: provider.into(), message, detail: format!("HTTP {status}: {api_msg}"), key_problem }
}

fn net_error(provider: &str, e: reqwest::Error) -> ProviderError {
    let message = if e.is_timeout() {
        "tempo esgotado".to_string()
    } else if e.is_connect() {
        if provider == "ollama" {
            "o Ollama não está rodando neste computador".to_string()
        } else {
            "sem conexão com a internet".to_string()
        }
    } else {
        "falha de rede".to_string()
    };
    ProviderError { provider: provider.into(), message, detail: e.to_string(), key_problem: false }
}

async fn read(provider: &str, resp: reqwest::Response) -> Result<Value, ProviderError> {
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|e| net_error(provider, e))?;
    if !(200..300).contains(&status) {
        return Err(friendly(provider, status, &body));
    }
    serde_json::from_str(&body).map_err(|_| ProviderError {
        provider: provider.into(),
        message: "resposta inesperada do serviço".into(),
        detail: body.chars().take(300).collect(),
        key_problem: false,
    })
}

// ---------------------------------------------------------------- speech to text

/// Transcribes WAV bytes with one key. `prompt` biases spelling (dictionary terms).
pub async fn transcribe(
    client: &reqwest::Client,
    provider: &str,
    cfg: &ProviderSettings,
    key: &str,
    wav: &[u8],
    lang: &str,
    prompt: &str,
) -> Result<String, ProviderError> {
    if provider == "gemini" {
        return transcribe_gemini(client, cfg, key, wav, lang, prompt).await;
    }
    let url = format!("{}/audio/transcriptions", base_url(provider, cfg));
    let part = reqwest::multipart::Part::bytes(wav.to_vec())
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .expect("mime");
    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", cfg.stt_model.clone())
        .text("response_format", "json")
        .text("temperature", "0");
    if lang != "auto" && provider != "openai" {
        form = form.text("language", lang.to_string());
    }
    if provider == "openai" && lang != "auto" {
        form = form.text("languages", lang.to_string());
    }
    if !prompt.trim().is_empty() && provider != "mistral" {
        form = form.text("prompt", prompt.to_string());
    }
    let resp = client
        .post(url)
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| net_error(provider, e))?;
    let v = read(provider, resp).await?;
    Ok(v["text"].as_str().unwrap_or_default().trim().to_string())
}

async fn transcribe_gemini(
    client: &reqwest::Client,
    cfg: &ProviderSettings,
    key: &str,
    wav: &[u8],
    lang: &str,
    prompt: &str,
) -> Result<String, ProviderError> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(wav);
    let lang_hint = if lang == "auto" {
        "no idioma falado".to_string()
    } else {
        format!("no idioma \"{lang}\"")
    };
    let mut instruction = format!(
        "Transcreva este áudio literalmente {lang_hint}, palavra por palavra, com pontuação. Não resuma, não traduza, não comente e nunca censure palavrões. Responda somente com a transcrição. Se não houver fala, responda vazio."
    );
    if !prompt.trim().is_empty() {
        instruction.push_str(&format!(" Termos que podem aparecer: {prompt}"));
    }
    let body = json!({
        "model": cfg.stt_model,
        "temperature": 0,
        "messages": [{
            "role": "user",
            "content": [
                { "type": "text", "text": instruction },
                { "type": "input_audio", "input_audio": { "data": b64, "format": "wav" } }
            ]
        }]
    });
    let url = format!("{}/chat/completions", base_url("gemini", cfg));
    let resp = client.post(url).bearer_auth(key).json(&body).send().await.map_err(|e| net_error("gemini", e))?;
    let v = read("gemini", resp).await?;
    Ok(message_text(&v))
}

// ---------------------------------------------------------------- chat

#[derive(Clone, Debug)]
pub struct ChatMsg {
    pub role: String,
    pub content: String,
}

impl ChatMsg {
    pub fn new(role: &str, content: impl Into<String>) -> Self {
        Self { role: role.into(), content: content.into() }
    }
}

fn message_text(v: &Value) -> String {
    let content = &v["choices"][0]["message"]["content"];
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join(""),
        _ => String::new(),
    };
    text.trim().to_string()
}

/// Reasoning knob per provider: we want fast, non-thinking rewrites.
fn reasoning_param(provider: &str, model: &str) -> Option<(&'static str, Value)> {
    let m = model.to_lowercase();
    match provider {
        "groq" if m.contains("gpt-oss") => Some(("reasoning_effort", json!("low"))),
        "openai" if m.starts_with("gpt-5") || m.starts_with("gpt-6") || m.starts_with('o') => {
            Some(("reasoning_effort", json!("low")))
        }
        "gemini" => Some(("reasoning_effort", json!("low"))),
        _ => None,
    }
}

pub async fn chat(
    client: &reqwest::Client,
    provider: &str,
    cfg: &ProviderSettings,
    key: &str,
    messages: &[ChatMsg],
    temperature: f32,
    max_tokens: u32,
) -> Result<String, ProviderError> {
    let model = cfg.model.trim();
    if model.is_empty() {
        return Err(ProviderError {
            provider: provider.into(),
            message: "nenhum modelo configurado".into(),
            detail: String::new(),
            key_problem: false,
        });
    }
    let url = format!("{}/chat/completions", base_url(provider, cfg));
    let msgs: Vec<Value> = messages.iter().map(|m| json!({ "role": m.role, "content": m.content })).collect();
    let mut body = json!({
        "model": model,
        "messages": msgs,
        "temperature": temperature,
        "max_tokens": max_tokens,
        "stream": false,
    });
    if provider == "openai" {
        // Newer OpenAI models reject max_tokens/temperature on reasoning models.
        body.as_object_mut().unwrap().remove("max_tokens");
        body["max_completion_tokens"] = json!(max_tokens);
        body.as_object_mut().unwrap().remove("temperature");
    }
    if provider == "groq" {
        body["include_reasoning"] = json!(false);
    }
    if let Some((k, v)) = reasoning_param(provider, model) {
        body[k] = v;
    }
    let send = |body: Value| {
        let mut req = client.post(&url).json(&body);
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
        if provider == "openrouter" {
            req = req
                .header("HTTP-Referer", "https://github.com/cesarkali/Flow-Voice")
                .header("X-Title", "FlowVoice");
        }
        req.send()
    };
    let resp = send(body.clone()).await.map_err(|e| net_error(provider, e))?;
    let v = match read(provider, resp).await {
        Ok(v) => v,
        // Some models reject the optional knobs — retry once with a bare request.
        Err(e) if e.detail.starts_with("HTTP 400") && !e.key_problem => {
            let o = body.as_object_mut().unwrap();
            for k in ["reasoning_effort", "include_reasoning", "temperature"] {
                o.remove(k);
            }
            let resp = send(body).await.map_err(|e| net_error(provider, e))?;
            read(provider, resp).await?
        }
        Err(e) => return Err(e),
    };
    let text = message_text(&v);
    if text.is_empty() {
        return Err(ProviderError {
            provider: provider.into(),
            message: "a IA devolveu uma resposta vazia".into(),
            detail: v["choices"][0]["finish_reason"].to_string(),
            key_problem: false,
        });
    }
    Ok(text)
}

pub async fn list_models(client: &reqwest::Client, provider: &str, cfg: &ProviderSettings, key: &str) -> Result<Vec<String>> {
    let url = format!("{}/models", base_url(provider, cfg));
    let mut req = client.get(url);
    if !key.is_empty() {
        req = req.bearer_auth(key);
    }
    let resp = req.send().await?;
    let v: Value = resp.error_for_status()?.json().await?;
    let mut ids: Vec<String> = v["data"]
        .as_array()
        .ok_or_else(|| anyhow!("lista vazia"))?
        .iter()
        .filter_map(|m| m["id"].as_str().map(|s| s.trim_start_matches("models/").to_string()))
        .collect();
    if provider == "openrouter" {
        ids.retain(|id| id.ends_with(":free") || id == "openrouter/free");
    }
    ids.sort();
    Ok(ids)
}

// ---------------------------------------------------------------- web search

pub struct SearchResult {
    pub answer: Option<String>,
    pub results: Vec<(String, String, String)>, // title, url, content
}

pub async fn tavily(client: &reqwest::Client, key: &str, query: &str) -> Result<SearchResult, ProviderError> {
    let body = json!({
        "query": query,
        "search_depth": "basic",
        "max_results": 5,
        "include_answer": true,
    });
    let resp = client
        .post("https://api.tavily.com/search")
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| net_error("tavily", e))?;
    let v = read("tavily", resp).await?;
    let results = v["results"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|r| {
                    (
                        r["title"].as_str().unwrap_or_default().to_string(),
                        r["url"].as_str().unwrap_or_default().to_string(),
                        r["content"].as_str().unwrap_or_default().chars().take(600).collect(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(SearchResult { answer: v["answer"].as_str().map(str::to_string).filter(|a| !a.is_empty()), results })
}
