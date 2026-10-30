//! Deterministic text clean-up that runs before (or instead of) the LLM.

use once_cell::sync::Lazy;
use regex::Regex;

/// Pure hesitation sounds, safe to drop anywhere (every language).
static HESITATIONS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(^|[\s,.;!?…-])(h+[aã]+h*|[aã]+h+|ahn+|hu+m+|hm+|u+h+m*|e+r+m+|é{2,}|e{3,}|ã{2,})([\s,.;!?…-]|$)").unwrap()
});

/// Portuguese tag words: "né", "tipo", "sabe", "tipo assim" when used as fillers (between commas / at clause end).
static PT_TAGS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:,?\s*\bné|,\s*(?:tipo assim|tipo|sabe|entendeu|tá ligado))\b\s*([,?]|$)").unwrap()
});
static PT_TAG_START: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(^|[.!?]\s+)(tipo assim|tipo|então tipo),\s*").unwrap());

static MULTI_SPACE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]{2,}").unwrap());
static SPACE_BEFORE_PUNCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+([,.;:!?])").unwrap());
static DOUBLE_COMMA: Lazy<Regex> = Lazy::new(|| Regex::new(r",\s*([,.;:!?])").unwrap());
static LEADING_PUNCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[\s,;:.…-]+").unwrap());
/// Non-speech annotations from whisper: [Música], (risos), [BLANK_AUDIO], *suspiro*.
static ANNOTATIONS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[[^\]]*\]|\([^)]*(risos|música|music|aplausos|applause|silêncio|silence|inaudível|inaudible)[^)]*\)|\*[^*]{1,30}\*").unwrap());

pub fn strip_annotations(text: &str) -> String {
    tidy(&ANNOTATIONS.replace_all(text, " "))
}

/// Removes fillers conservatively. `lang` is the speech language ("pt", "en", "auto"...).
pub fn remove_fillers(text: &str, lang: &str) -> String {
    let mut t = text.to_string();
    // Run twice: adjacent fillers share delimiters.
    for _ in 0..2 {
        t = HESITATIONS.replace_all(&t, "$1$3").into_owned();
    }
    if lang == "pt" || lang == "auto" {
        t = PT_TAG_START.replace_all(&t, "$1").into_owned();
        t = PT_TAGS
            .replace_all(&t, |c: &regex::Captures| match &c[1] {
                "?" => "?".to_string(),
                "," => ",".to_string(),
                _ => String::new(),
            })
            .into_owned();
    }
    t = collapse_stutter(&t);
    let t = tidy(&t);
    capitalize_first(&t)
}

/// "eu eu", "que que", "o o" — short words repeated back to back.
fn collapse_stutter(t: &str) -> String {
    let strip = |w: &str| w.trim_end_matches(|c: char| c.is_ascii_punctuation()).to_lowercase();
    let mut out: Vec<String> = vec![];
    for word in t.split(' ') {
        if let Some(prev) = out.last_mut() {
            let core = strip(word);
            let prev_open = !prev.ends_with(|c: char| c.is_ascii_punctuation());
            if !core.is_empty() && core.chars().count() <= 3 && prev_open && strip(prev) == core {
                // Drop the repeat but keep its trailing punctuation ("eu eu," → "eu,").
                prev.push_str(&word[word.trim_end_matches(|c: char| c.is_ascii_punctuation()).len()..]);
                continue;
            }
        }
        out.push(word.to_string());
    }
    out.join(" ")
}

fn capitalize_first(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn tidy(t: &str) -> String {
    let t = DOUBLE_COMMA.replace_all(t, "$1");
    let t = SPACE_BEFORE_PUNCT.replace_all(&t, "$1");
    let t = MULTI_SPACE.replace_all(&t, " ");
    let t = LEADING_PUNCT.replace(&t, "");
    t.lines().map(str::trim).collect::<Vec<_>>().join("\n").trim().to_string()
}

/// "Cru" style: only words — no punctuation at word boundaries, no line breaks.
pub fn raw_style(text: &str) -> String {
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !(c.is_alphanumeric() || c == '@' || c == '#' || c == '%' || c == '$')))
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

const HALLUCINATION_PHRASES: [&str; 16] = [
    "legendas por",
    "legenda por",
    "legendado por",
    "subtitles by",
    "subs by",
    "obrigado por assistir",
    "obrigada por assistir",
    "thank you for watching",
    "thanks for watching",
    "transcrição por",
    "transcricao por",
    "transcrição e legendas",
    "transcricao e legendas",
    "adriana zanotto",
    "pedro negri",
    "inscreva-se no canal",
];
const HALLUCINATION_WORDS: [&str; 16] = [
    "robert", "sonia", "sônia", "ruberti", "ruberty", "legendas", "legenda", "you", "bye", "obrigado", "obrigada",
    "assistir", "tchau", "e aí", "amara.org", "silêncio",
];

/// True when whisper returned one of its classic "no speech" hallucinations.
pub fn is_hallucination(text: &str) -> bool {
    let norm: String = text
        .to_lowercase()
        .chars()
        .map(|c| if ".,!?\"':-…".contains(c) { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if norm.is_empty() {
        return true;
    }
    if HALLUCINATION_WORDS.contains(&norm.as_str()) {
        return true;
    }
    // Phrases only count when they are (almost) the whole output.
    HALLUCINATION_PHRASES.iter().any(|p| norm.contains(p) && norm.len() < p.len() + 40)
}

/// Strips wrappers some models add despite instructions.
pub fn clean_llm_output(text: &str) -> String {
    let mut t = text.trim().to_string();
    for tag in ["<transcricao>", "</transcricao>"] {
        t = t.replace(tag, "");
    }
    // <think>…</think> blocks from reasoning models served without a separate field.
    if let (Some(a), Some(b)) = (t.find("<think>"), t.find("</think>")) {
        if b > a {
            t.replace_range(a..b + "</think>".len(), "");
        }
    }
    let t = t.trim();
    let t = t.strip_prefix("```").and_then(|x| x.strip_suffix("```")).unwrap_or(t).trim();
    let quoted = (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('“') && t.ends_with('”'));
    if quoted && t.chars().count() > 2 {
        let inner: String = t.chars().skip(1).take(t.chars().count() - 2).collect();
        if !inner.contains('"') {
            return inner.trim().to_string();
        }
    }
    t.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fillers_pt() {
        assert_eq!(remove_fillers("hã eu queria, tipo, saber se dá, né?", "pt"), "Eu queria, saber se dá?");
        assert_eq!(remove_fillers("eu eu vou lá né", "pt"), "Eu vou lá");
        assert_eq!(remove_fillers("que tipo de coisa é essa", "pt"), "Que tipo de coisa é essa");
        assert_eq!(remove_fillers("um carro novo", "pt"), "Um carro novo");
        assert_eq!(remove_fillers("gosto desse tipo", "pt"), "Gosto desse tipo");
        assert_eq!(remove_fillers("você sabe?", "pt"), "Você sabe?");
    }

    #[test]
    fn raw() {
        assert_eq!(raw_style("Olá, tudo bem?\nEu uso Next.js."), "Olá tudo bem Eu uso Next.js");
    }

    #[test]
    fn hallucinations() {
        assert!(is_hallucination("Legendas por Adriana Zanotto"));
        assert!(is_hallucination("Obrigado."));
        assert!(!is_hallucination("Obrigado pela ajuda com o relatório de ontem"));
    }

    #[test]
    fn annotations() {
        assert_eq!(strip_annotations("[Música] Olá (risos) pessoal [BLANK_AUDIO]"), "Olá pessoal");
    }

    #[test]
    fn llm_cleanup() {
        assert_eq!(clean_llm_output("\"Olá mundo\""), "Olá mundo");
        assert_eq!(clean_llm_output("<think>x</think>Oi"), "Oi");
    }
}
