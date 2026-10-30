//! Deterministic text clean-up that runs before (or instead of) the LLM.

use once_cell::sync::Lazy;
use regex::Regex;

/// Pure hesitation sounds, safe to drop anywhere (every language).
static HESITATIONS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(^|[\s,.;!?…-])(h+ã+h*|h+a{2,}h*|h+a+h+|[aã]+h+|ahn+|hu+m+|hm+|u+h+m*|e+r+m+|é{2,}|e{3,}|ã{2,})([\s,.;!?…-]|$)").unwrap()
});

/// Portuguese tag words: "né", "tipo", "sabe", "tipo assim" when used as fillers (between commas / at clause end).
static PT_TAGS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:,?\s*\bné|,\s*(?:tipo assim|tipo|sabe|entendeu|tá ligado))\b\s*([,?]|$)").unwrap()
});
static PT_TAG_START: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(^|[.!?]\s+)(tipo assim|tipo|então tipo),\s*").unwrap());

static MULTI_SPACE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]{2,}").unwrap());
static SPACE_BEFORE_PUNCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+([,.;:!?])").unwrap());
static DOUBLE_COMMA: Lazy<Regex> = Lazy::new(|| Regex::new(r",\s*([,.;:!?])").unwrap());
/// Leading junk punctuation; a dot glued to a word is kept (".NET", ".env").
static LEADING_PUNCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?:[\s,;:…-]|\.+(?:\s|$))+").unwrap());
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
                // "…à tarde, né?" is a tag, not a real question: end the sentence instead.
                "?" if c[0].to_lowercase().contains("né") => ".".to_string(),
                // "…a regra, entendeu?" is a real question: keep it.
                "?" => c[0].to_string(),
                "," => ",".to_string(),
                _ => String::new(),
            })
            .into_owned();
    }
    t = collapse_stutter(&t);
    let t = tidy(&t);
    capitalize_first(&t)
}

/// Short words people repeat on purpose ("não não", "lá lá", "já já").
const INTENTIONAL_REPEATS: [&str; 12] = ["não", "nao", "sim", "lá", "já", "só", "tá", "oi", "ok", "no", "yes", "si"];

/// "eu eu", "que que", "o o" — short words repeated back to back.
fn collapse_stutter(t: &str) -> String {
    let strip = |w: &str| w.trim_end_matches(|c: char| c.is_ascii_punctuation()).to_lowercase();
    let mut out: Vec<String> = vec![];
    for word in t.split(' ') {
        if let Some(prev) = out.last_mut() {
            let core = strip(word);
            let prev_open = !prev.ends_with(|c: char| c.is_ascii_punctuation());
            let intentional = INTENTIONAL_REPEATS.contains(&core.as_str());
            if !core.is_empty() && !intentional && core.chars().count() <= 3 && prev_open && strip(prev) == core {
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

pub fn has_words(text: &str) -> bool {
    text.chars().any(char::is_alphanumeric)
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
/// Single outputs whisper invents on silence/noise. The first group are also real short replies
/// ("Obrigado!", "Tchau"), so they only count when there was barely any voiced audio.
const HALLUCINATION_WORDS: [&str; 9] = ["robert", "sonia", "sônia", "ruberti", "ruberty", "legendas", "legenda", "amara.org", "silêncio"];
const SHORT_REPLY_WORDS: [&str; 8] = ["you", "bye", "obrigado", "obrigada", "assistir", "tchau", "e aí", "thank you"];

fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if ".,!?\"':-…".contains(c) { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// True when whisper returned one of its classic "no speech" hallucinations. Used by the live
/// preview, where a short real reply is worth less than a wrong first guess.
pub fn is_hallucination(text: &str) -> bool {
    is_hallucination_voiced(text, 0.0)
}

/// Final-pass variant: `voiced_secs` is how much real voiced audio the recording had. With
/// enough of it, short replies such as "Obrigado!" are trusted.
pub fn is_hallucination_voiced(text: &str, voiced_secs: f32) -> bool {
    let norm = normalize(text);
    if norm.is_empty() {
        return true;
    }
    let same = |w: &&str| normalize(w) == norm;
    if HALLUCINATION_WORDS.iter().any(same) {
        return true;
    }
    if voiced_secs < 1.2 && SHORT_REPLY_WORDS.iter().any(same) {
        return true;
    }
    // Phrases only count when they ARE the output ("Legendas por Fulano de Tal"), not when they
    // appear inside a real sentence ("preciso da transcrição por e-mail").
    let words = norm.split(' ').count();
    HALLUCINATION_PHRASES.iter().any(|p| {
        let p = normalize(p);
        norm.starts_with(&p) && words <= p.split(' ').count() + 3
    })
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
    let t = t.lines().map(str::trim_end).collect::<Vec<_>>().join("
");
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

/// Output of "Comando". It replaces the selection exactly, so: fences/quotes are only stripped
/// when the selection didn't have them, and the selection's own leading/trailing whitespace
/// (a trailing space, the first line's indentation, a final newline) is kept.
pub fn clean_command_output(out: &str, selection: &str) -> String {
    let mut t = out.to_string();
    if let (Some(a), Some(b)) = (t.find("<think>"), t.find("</think>")) {
        if b > a {
            t.replace_range(a..b + "</think>".len(), "");
        }
    }
    for tag in ["<texto>", "</texto>", "<instrucao>", "</instrucao>"] {
        t = t.replace(tag, "");
    }
    let mut core = t.trim().to_string();
    let sel = selection.trim();
    if !sel.starts_with("```") && core.starts_with("```") && core.ends_with("```") && core.len() > 6 {
        let inner = &core[3..core.len() - 3];
        // drop the language tag line ("```markdown")
        let inner = match inner.find('\n') {
            Some(i) if !inner[..i].contains(' ') => &inner[i + 1..],
            _ => inner,
        };
        core = inner.trim().to_string();
    }
    let quoted = |s: &str| (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('“') && s.ends_with('”'));
    if quoted(&core) && !quoted(sel) && core.chars().count() > 2 {
        let inner: String = core.chars().skip(1).take(core.chars().count() - 2).collect();
        if !inner.contains(['"', '“', '”']) {
            core = inner.trim().to_string();
        }
    }
    let lead = &selection[..selection.len() - selection.trim_start().len()];
    let trail = &selection[selection.trim_end().len()..];
    format!("{lead}{core}{trail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_output_keeps_selection_shape() {
        assert_eq!(clean_command_output("Olá, tudo bem?", "oi tudo bem "), "Olá, tudo bem? ");
        assert_eq!(clean_command_output("<think>hmm</think>\n<texto>\nBom dia.\n</texto>", "bom dia"), "Bom dia.");
        assert_eq!(clean_command_output("```markdown\n- a\n- b\n```", "a, b"), "- a\n- b");
        assert_eq!(clean_command_output("```js\nlet x = 1;\n```", "```js\nvar x = 1;\n```"), "```js\nlet x = 1;\n```");
        assert_eq!(clean_command_output("\"Bom dia\"", "bom dia"), "Bom dia");
        assert_eq!(clean_command_output("\"Bom dia\"", "\"bom dia\""), "\"Bom dia\"");
        // Indented code: the first line's indentation comes from the selection.
        assert_eq!(clean_command_output("foo();\n    bar();\n", "    foo()\n    bar()\n"), "    foo();\n    bar();\n");
    }

    #[test]
    fn fillers_pt() {
        assert_eq!(remove_fillers("hã eu queria, tipo, saber se dá, né?", "pt"), "Eu queria, saber se dá.");
        assert_eq!(remove_fillers("marcar na quinta, né? Porque na quarta não dá", "pt"), "Marcar na quinta. Porque na quarta não dá");
        assert_eq!(remove_fillers("eu eu vou lá né", "pt"), "Eu vou lá");
        assert_eq!(remove_fillers("que tipo de coisa é essa", "pt"), "Que tipo de coisa é essa");
        assert_eq!(remove_fillers("um carro novo", "pt"), "Um carro novo");
        assert_eq!(remove_fillers("gosto desse tipo", "pt"), "Gosto desse tipo");
        assert_eq!(remove_fillers("você sabe?", "pt"), "Você sabe?");
    }

    #[test]
    fn hesitation_only_is_empty() {
        assert!(!has_words(&remove_fillers("Ah.", "pt")));
        assert!(!has_words(&remove_fillers("Hum... hã", "pt")));
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
        assert!(is_hallucination("Inscreva-se no canal!"));
        assert!(is_hallucination("Amara.org"));
        assert!(!is_hallucination("Preciso da transcrição por e-mail"));
        assert!(!is_hallucination("Coloca a legenda por cima do vídeo"));
        assert!(!is_hallucination_voiced("Obrigado!", 1.5));
        assert!(is_hallucination_voiced("Obrigado!", 0.3));
    }

    #[test]
    fn fillers_keep_meaning() {
        assert_eq!(remove_fillers("Expliquei a regra, entendeu?", "pt"), "Expliquei a regra, entendeu?");
        assert_eq!(remove_fillers("ella ha comido", "es"), "Ella ha comido");
        assert_eq!(remove_fillers("não não, eu não vou", "pt"), "Não não, eu não vou");
        assert_eq!(strip_annotations(".NET é rápido"), ".NET é rápido");
        assert_eq!(strip_annotations(", olá"), "Olá".to_lowercase());
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
