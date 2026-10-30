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
    /// The model id was retired/unknown (heal::heal can pick a replacement).
    pub model_gone: bool,
    /// Seconds the provider asked us to wait (429 retry-after), when it said so.
    pub retry_after: Option<f32>,
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
    let gone = model_gone(status, &lower);
    let (message, key_problem) = if gone {
        ("modelo descontinuado ou indisponível".to_string(), false)
    } else {
        (message, key_problem)
    };
    ProviderError {
        provider: provider.into(),
        message,
        detail: format!("HTTP {status}: {api_msg}"),
        key_problem,
        model_gone: gone,
        retry_after: None,
    }
}

/// True when the error says the model id itself is gone (retired, renamed, unknown).
pub fn model_gone(status: u16, lower_msg: &str) -> bool {
    const GONE: [&str; 12] = [
        "not found", "does not exist", "decommissioned", "deprecated", "not supported", "invalid model",
        "no longer", "unknown model", "not available", "retired", "model_not_found", "unsupported model",
    ];
    if lower_msg.contains("model_not_found") || lower_msg.contains("no endpoints found") {
        return true;
    }
    match status {
        404 => true,
        400 | 403 | 422 => lower_msg.contains("model") && GONE.iter().any(|w| lower_msg.contains(w)),
        _ => false,
    }
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
    ProviderError { provider: provider.into(), message, detail: e.to_string(), key_problem: false, model_gone: false, retry_after: None }
}

async fn read(provider: &str, resp: reqwest::Response) -> Result<Value, ProviderError> {
    let status = resp.status().as_u16();
    let retry_after = retry_after(resp.headers());
    let body = resp.text().await.map_err(|e| net_error(provider, e))?;
    if !(200..300).contains(&status) {
        let mut e = friendly(provider, status, &body);
        if status == 429 {
            e.retry_after = retry_after.or_else(|| retry_from_body(&body));
        }
        return Err(e);
    }
    serde_json::from_str(&body).map_err(|_| ProviderError {
        provider: provider.into(),
        message: "resposta inesperada do serviço".into(),
        detail: body.chars().take(300).collect(),
        key_problem: false,
        model_gone: false,
        retry_after: None,
    })
}

/// `retry-after: 7` / `retry-after: 1.5` (seconds) on 429s.
fn retry_after(h: &reqwest::header::HeaderMap) -> Option<f32> {
    h.get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<f32>().ok())
}

/// Gemini puts it in the body: `"retryDelay": "21s"` / "Please retry in 21.4s".
fn retry_from_body(body: &str) -> Option<f32> {
    let lower = body.to_lowercase();
    for marker in ["\"retrydelay\": \"", "retry in ", "try again in "] {
        if let Some(i) = lower.find(marker) {
            let rest = &lower[i + marker.len()..];
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(v) = num.parse::<f32>() {
                let unit = &rest[num.len()..];
                if unit.starts_with("ms") {
                    return Some(v / 1000.0);
                }
                if let Some(secs) = unit.strip_prefix('m') {
                    // Groq: "try again in 1m2.5s"
                    let s: String = secs.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
                    return Some(v * 60.0 + s.parse::<f32>().unwrap_or(0.0));
                }
                return Some(v);
            }
        }
    }
    None
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
fn reasoning_param(provider: &str, model: &str, effort: &'static str) -> Option<(&'static str, Value)> {
    let m = model.to_lowercase();
    match provider {
        "groq" if m.contains("gpt-oss") => Some(("reasoning_effort", json!(effort))),
        "openai" if m.starts_with("gpt-5") || m.starts_with("gpt-6") || m.starts_with('o') => {
            Some(("reasoning_effort", json!(effort)))
        }
        "gemini" => Some(("reasoning_effort", json!(effort))),
        _ => None,
    }
}

/// Stronger model per provider for the voice assistant (answers need more than rewrites).
pub fn strong_model(provider: &str, configured: &str) -> String {
    match provider {
        "groq" if configured.contains("gpt-oss-20b") => "openai/gpt-oss-120b".into(),
        "gemini" if configured.contains("flash-lite") => "gemini-3.8-flash".into(),
        _ => configured.to_string(),
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
    chat_with_effort(client, provider, cfg, key, messages, temperature, max_tokens, "low").await
}

#[allow(clippy::too_many_arguments)]
pub async fn chat_with_effort(
    client: &reqwest::Client,
    provider: &str,
    cfg: &ProviderSettings,
    key: &str,
    messages: &[ChatMsg],
    temperature: f32,
    max_tokens: u32,
    effort: &'static str,
) -> Result<String, ProviderError> {
    let model = cfg.model.trim();
    if model.is_empty() {
        return Err(ProviderError {
            provider: provider.into(),
            message: "nenhum modelo configurado".into(),
            detail: String::new(),
            key_problem: false,
            model_gone: false,
            retry_after: None,
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
    if let Some((k, v)) = reasoning_param(provider, model, effort) {
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
        Err(e) if e.detail.starts_with("HTTP 400") && !e.key_problem && !e.model_gone => {
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
            model_gone: false,
            retry_after: None,
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
        "search_depth": "advanced",
        "max_results": 6,
        "include_answer": "advanced",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_retired_models() {
        let g = |s: u16, m: &str| friendly("groq", s, &format!(r#"{{"error":{{"message":"{m}"}}}}"#)).model_gone;
        assert!(g(400, "The model `llama3-8b-8192` has been decommissioned and is no longer supported."));
        assert!(g(404, "The model `whisper-large-v2` does not exist or you do not have access to it."));
        assert!(g(400, "models/gemini-1.5-flash is not found for API version v1beta, or is not supported for generateContent."));
        assert!(g(404, "No endpoints found for meta-llama/llama-3-8b-instruct:free."));
        assert!(!g(400, "response_format is not supported"));
        assert!(!g(401, "Invalid API Key"));
        assert!(!g(429, "Rate limit reached for model whisper-large-v3"));
        let e = friendly("groq", 400, r#"{"error":{"message":"The model x has been decommissioned"}}"#);
        assert!(!e.key_problem);
        let e = friendly("gemini", 403, r#"{"error":{"message":"Permission denied on API key"}}"#);
        assert!(e.key_problem && !e.model_gone);
    }

    #[test]
    fn retry_delay_from_body() {
        assert_eq!(retry_from_body(r#"[{"error":{"details":[{"retryDelay": "21s"}]}}]"#), Some(21.0));
        assert_eq!(retry_from_body("Please try again in 7.5s"), Some(7.5));
        assert_eq!(retry_from_body("Please retry in 2.5s."), Some(2.5));
        assert_eq!(retry_from_body("try again in 1m2.5s"), Some(62.5));
    }
}
