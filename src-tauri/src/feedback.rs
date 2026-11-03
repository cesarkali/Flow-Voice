//! The log excerpt attached to a bug report: the tail of flowvoice.log, cleaned so it can be
//! posted publicly — no user name in paths, nothing that looks like a key, and no line that
//! could carry dictated text (those only exist in debug runs, but never leave the machine).

use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

/// Kept small: the excerpt travels inside the GitHub URL (which has a length limit).
const MAX_CHARS: usize = 2500;
/// Longest URL every Windows/browser path passes on whole (INTERNET_MAX_URL_LENGTH).
const MAX_URL: usize = 2083;

/// The issue form's URL with `params`, then the error message (`what`) and the log excerpt. Both
/// are cut (the log's oldest lines first, then the message's end) until the whole URL, once
/// percent-encoded, fits in MAX_URL: a longer one gets cut short on its way to the browser.
pub fn issue_url(base: &str, params: &[(&str, String)], what: Option<&str>, log: &str) -> anyhow::Result<reqwest::Url> {
    let mut what: Vec<char> = what.unwrap_or_default().chars().collect();
    let mut lines: Vec<&str> = if log.is_empty() { vec![] } else { log.lines().collect() };
    loop {
        let mut all = params.to_vec();
        if !what.is_empty() {
            all.push(("what", format!("Apareceu este erro:\n\n> {}\n\n", what.iter().collect::<String>())));
        }
        if !lines.is_empty() {
            all.push(("logs", lines.join("\n")));
        }
        let url = reqwest::Url::parse_with_params(base, &all)?;
        if url.as_str().len() <= MAX_URL {
            return Ok(url);
        }
        if !lines.is_empty() {
            lines.remove(0);
        } else if !what.is_empty() {
            what.truncate(what.len().saturating_sub(20));
        } else {
            return Ok(url);
        }
    }
}

static SECRETS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(gsk_[A-Za-z0-9]{8,}|AIza[0-9A-Za-z_\-]{16,}|sk-[A-Za-z0-9_\-]{12,}|tvly-[A-Za-z0-9_\-]{8,}|(?i:bearer)\s+[A-Za-z0-9._\-]{12,})").unwrap()
});

/// Lines that may contain what the user said (debug-only preview logs) or file contents.
const PRIVATE_MARKERS: [&str; 5] = ["preview text", "preview held", "preview extend", "preview local tail", "no speech in transcript:"];

pub fn log_excerpt(config_dir: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(config_dir.join("flowvoice.log")) else { return String::new() };
    clean(&text, std::env::var("USERPROFILE").ok().as_deref(), std::env::var("USERNAME").ok().as_deref())
}

fn clean(text: &str, profile: Option<&str>, user: Option<&str>) -> String {
    let mut kept: Vec<String> = vec![];
    let mut size = 0;
    for line in text.lines().rev() {
        if PRIVATE_MARKERS.iter().any(|m| line.contains(m)) {
            continue;
        }
        let mut l = SECRETS.replace_all(line, "[chave removida]").into_owned();
        if let Some(p) = profile.filter(|p| p.len() > 3) {
            l = l.replace(p, "%USERPROFILE%");
        }
        if let Some(u) = user.filter(|u| u.len() > 2) {
            l = l.replace(&format!("\\{u}\\"), "\\%USERNAME%\\");
        }
        size += l.chars().count() + 1;
        if size > MAX_CHARS {
            break;
        }
        kept.push(l);
    }
    kept.reverse();
    kept.join("\n")
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn removes_keys_paths_and_private_lines() {
        let log = "2026 INFO start\n\
                   2026 WARN stt groq failed: chave inválida (HTTP 401: Invalid API Key gsk_abcdefghijklmnop)\n\
                   2026 INFO preview text [groq] t+1s: o que eu falei\n\
                   2026 INFO data C:\\Users\\julio\\AppData\\Roaming\\x\n";
        let out = clean(log, Some("C:\\Users\\julio"), Some("julio"));
        assert!(!out.contains("gsk_"));
        assert!(out.contains("[chave removida]"));
        assert!(!out.contains("o que eu falei"));
        assert!(!out.contains("julio"));
        assert!(out.contains("%USERPROFILE%"));
        assert!(out.starts_with("2026 INFO start"));
    }

    #[test]
    fn issue_url_fits_the_url_limit() {
        let base = "https://github.com/cesarkali/Flow-Voice/issues/new";
        let params = [("template", "bug_report.yml".to_string()), ("version", "2.1.0".to_string())];
        // Accented text triples in size once percent-encoded.
        let log = (0..200).map(|i| format!("2026 WARN linha {i}: transcrição ação não")).collect::<Vec<_>>().join("\n");
        let url = super::issue_url(base, &params, Some("Falha na transcrição: Groq: sem conexão"), &log).unwrap();
        assert!(url.as_str().len() <= super::MAX_URL, "{}", url.as_str().len());
        let logs = url.query_pairs().find(|(k, _)| k == "logs").map(|(_, v)| v.into_owned()).unwrap();
        assert!(logs.ends_with("linha 199: transcrição ação não"), "the newest lines stay");
        assert!(url.query_pairs().any(|(k, v)| k == "what" && v.contains("sem conexão")));
        // A message too long even without a log is shortened too.
        let url = super::issue_url(base, &params, Some(&"ç".repeat(600)), "").unwrap();
        assert!(url.as_str().len() <= super::MAX_URL);
    }

    #[test]
    fn keeps_only_the_tail() {
        let log = (0..2000).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let out = clean(&log, None, None);
        assert!(out.ends_with("line 1999"));
        assert!(out.chars().count() <= super::MAX_CHARS);
    }
}
