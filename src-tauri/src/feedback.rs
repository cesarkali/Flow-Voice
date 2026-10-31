//! The log excerpt attached to a bug report: the tail of flowvoice.log, cleaned so it can be
//! posted publicly — no user name in paths, nothing that looks like a key, and no line that
//! could carry dictated text (those only exist in debug runs, but never leave the machine).

use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

/// Kept small: the excerpt travels inside the GitHub URL (which has a length limit).
const MAX_CHARS: usize = 2500;

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
    fn keeps_only_the_tail() {
        let log = (0..2000).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let out = clean(&log, None, None);
        assert!(out.ends_with("line 1999"));
        assert!(out.chars().count() <= super::MAX_CHARS);
    }
}
