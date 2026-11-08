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

/// Each match of `re` becomes its group 1 (the punctuation), unless that punctuation starts a word
/// (".NET", ".env", "5,5"): then it isn't loose and stays as it was.
fn drop_before_loose_punct<'a>(re: &Regex, t: &'a str) -> std::borrow::Cow<'a, str> {
    re.replace_all(t, |c: &regex::Captures| {
        let end = c.get(0).map_or(t.len(), |m| m.end());
        if t[end..].starts_with(char::is_alphanumeric) {
            c[0].to_string()
        } else {
            c[1].to_string()
        }
    })
}

fn tidy(t: &str) -> String {
    let t = drop_before_loose_punct(&DOUBLE_COMMA, t);
    let t = drop_before_loose_punct(&SPACE_BEFORE_PUNCT, &t);
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

/// True when a transcript is just a piece of the prompt it was given (Whisper repeats its
/// prompt on silence/noise): the whole transcript appears inside the prompt.
pub fn echoes_prompt(text: &str, prompt: &str) -> bool {
    let (t, p) = (normalize(text), normalize(prompt));
    if t.is_empty() || p.is_empty() {
        return false;
    }
    t.split(' ').count() >= 3 && p.contains(&t)
}

/// The answer the dictation prompt asks for when the transcript asks for its instructions
/// (settings::system_prompt, rule 7). It is Portuguese whatever language was spoken, so it is
/// never pasted: the cleaned transcript goes instead.
pub fn is_prompt_refusal(text: &str) -> bool {
    text.trim().trim_end_matches('.').trim_end() == "Não foi possível transcrever"
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

/// Line breaks as "\n". Windows apps copy a selection with "\r\n" and models answer with "\n";
/// paragraphs are compared and cut by "\n\n", so both sides must agree. (The paste turns them
/// back into "\r\n" on Windows.)
pub fn lf_newlines(text: &str) -> String {
    if !text.contains('\r') {
        return text.to_string();
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// A proofread that changed nothing: the model's answer is the selection, line breaks aside.
pub fn proofread_untouched(selection: &str, result: &str) -> bool {
    lf_newlines(selection).trim() == lf_newlines(result).trim()
}

/// A translated "Comando" result that still has the selection as it was before (or after) the
/// translation, the translation twice ("X", then "Translated: X"), a second labeled version
/// ("Improved Prompt: …"), or a "---" line and a comment after it: only the translation, once.
/// Trimmed (`clean_command_output` puts the selection's own spacing back afterwards).
pub fn drop_echoed_source(output: &str, selection: &str) -> String {
    let selection = lf_newlines(selection);
    let source = selection.trim();
    let mut all = output.trim().to_string();
    if let Some(rule) = all.find("\n---") {
        if !source.contains("---") && has_words(&all[..rule]) {
            all = all[..rule].trim().to_string();
        }
    }
    // Exactly two paragraphs, the second one ending with the first.
    if let Some(gap) = all.find("\n\n") {
        let first = all[..gap].trim();
        let second = &all[gap + 2..];
        if !second.contains("\n\n") && first.chars().count() >= 8 && second.trim().ends_with(first) {
            return first.to_string();
        }
    }
    if source.chars().count() >= 8 && all.len() > source.len() {
        let rest = all.strip_prefix(source).or_else(|| all.strip_suffix(source)).map(str::trim);
        if let Some(rest) = rest.filter(|r| has_words(r)) {
            all = rest.to_string();
        }
    }
    // (One line in: a paragraph after it that opens with a short label is another version. After
    // the echo is gone, or "selection\n\nPrompt: translation" would keep the selection.)
    if !source.contains('\n') {
        let mut from = 0;
        while let Some(i) = all[from..].find("\n\n") {
            let gap = from + i;
            if let Some((label, _)) = all[gap + 2..].trim().split_once(':') {
                let n = label.chars().count();
                if n > 0
                    && n <= 40
                    && label.matches(' ').count() < 4
                    && !label.contains(['.', '!', '?', '"', '“'])
                    && has_words(&all[..gap])
                {
                    return all[..gap].trim().to_string();
                }
            }
            from = gap + 2;
        }
    }
    all
}

/// Output of "Comando". It replaces the selection exactly, so: fences/quotes are only stripped
/// when the selection didn't have them, and the selection's own leading/trailing whitespace
/// (a trailing space, the first line's indentation, a final newline) is kept.
pub fn clean_command_output(out: &str, selection: &str) -> String {
    let selection = &lf_newlines(selection);
    let mut t = lf_newlines(out);
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
    // A note the model added at the end ("(Note: translated for clarity.)"), when the selection
    // didn't end with one.
    let is_note = |p: &str| (p.starts_with('(') && p.ends_with(')')) || p.starts_with("Note:") || p.starts_with("Nota:");
    let sel_last = sel.rfind("\n\n").map_or(sel, |g| &sel[g + 2..]).trim();
    if let Some(gap) = core.rfind("\n\n") {
        if has_words(&core[..gap]) && is_note(core[gap + 2..].trim()) && !is_note(sel_last) {
            core = core[..gap].trim().to_string();
        }
    }
    // Markdown hard breaks ("10h.  \n") would paste as stray spaces at the end of a paragraph,
    // unless the selection itself has lines like that (code, Markdown on purpose).
    if core.contains('\n') && !sel.lines().any(|l| l.ends_with([' ', '\t'])) {
        core = core.lines().map(|l| l.trim_end_matches([' ', '\t'])).collect::<Vec<_>>().join("\n");
    }
    let lead = &selection[..selection.len() - selection.trim_start().len()];
    let trail = &selection[selection.trim_end().len()..];
    format!("{lead}{core}{trail}")
}

/// Scripts written without spaces between words and sentences.
fn is_unspaced_script(c: char) -> bool {
    matches!(c as u32,
        0x0E00..=0x0EFF // Thai, Lao
        | 0x1000..=0x109F // Myanmar
        | 0x1780..=0x17FF // Khmer
        | 0x2E80..=0x9FFF // CJK, kana
        | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
}

/// Words for comparing two versions of a text: lowercase, punctuation dropped, one character per
/// word in scripts without spaces (so a comma fixed in Chinese changes nothing here).
fn compare_words(t: &str) -> Vec<String> {
    let is_punct = |c: char| c.is_ascii_punctuation() || "“”‘’«»…—–¿¡。，、！？：；（）·।「」『』".contains(c);
    let mut words = vec![];
    let mut cur = String::new();
    for c in t.chars() {
        if c.is_whitespace() || is_punct(c) || is_unspaced_script(c) {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            if is_unspaced_script(c) && !is_punct(c) {
                words.push(c.to_string());
            }
        } else {
            cur.extend(c.to_lowercase().map(fold_accent));
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}

/// "reuniao" and "reunião" are the same word with an accent fixed.
fn fold_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        _ => c,
    }
}

/// "Revisar" may only fix spelling and punctuation: true when the model changed more than that
/// (over ~30% of the words, and more than 3 of them), i.e. it rewrote the text. A text full of
/// typos can trip it; then nothing is replaced, which is the safe side.
pub fn proofread_drift(before: &str, after: &str) -> bool {
    let (a, b) = (compare_words(before), compare_words(after));
    if a.is_empty() || b.is_empty() {
        return a.len() != b.len();
    }
    // Longest common subsequence of words, two rows.
    let mut prev = vec![0u32; b.len() + 1];
    let mut row = vec![0u32; b.len() + 1];
    for wa in &a {
        for (j, wb) in b.iter().enumerate() {
            row[j + 1] = if wa == wb { prev[j] + 1 } else { row[j].max(prev[j + 1]) };
        }
        std::mem::swap(&mut prev, &mut row);
    }
    let common = prev[b.len()] as usize;
    let (gone, added) = (a.len() - common, b.len() - common);
    (gone + added) as f32 / (a.len() + b.len()) as f32 > 0.3 && gone.max(added) > 3
}

/// A dictation that continues the previous one in the same field gets a space in between
/// ("…na quinta. A reunião…"), unless the joint already has one, the previous one ends opening
/// something, the new one starts with punctuation, or the script has no spaces (Japanese,
/// Chinese, Thai). `previous_last` = last character of the previous dictation.
pub fn needs_join_space(previous_last: Option<char>, text: &str) -> bool {
    let (Some(prev), Some(first)) = (previous_last, text.chars().next()) else { return false };
    if prev.is_whitespace()
        || first.is_whitespace()
        || is_unspaced_script(prev)
        || is_unspaced_script(first)
        || "([{“‘«¿¡-/".contains(prev)
    {
        return false;
    }
    first.is_alphanumeric() || "\"“‘«¿¡([".contains(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proofread_drift_allows_fixes_but_not_rewrites() {
        let typed = "oi pessoal, concerteza a reuniao de amanha vai ser as 10 entao tragam os relatorio";
        let fixed = "Oi, pessoal! Com certeza a reunião de amanhã vai ser às 10, então tragam os relatórios.";
        assert!(!proofread_drift(typed, fixed));
        assert!(!proofread_drift("Tudo certo.", "Tudo certo."));
        // Short texts: a few words fixed is never a rewrite.
        assert!(!proofread_drift("vc vai hj", "Você vai hoje?"));
        let rewritten = "Prezados, confirmo que a reunião ocorrerá amanhã às 10h. Solicito que tragam os relatórios.";
        assert!(proofread_drift(typed, rewritten));
        // An answer instead of the text.
        assert!(proofread_drift("me manda o arquivo quando puder", "Claro! Aqui está o texto revisado: Me manda o arquivo quando puder. Corrigi a pontuação e a maiúscula inicial."));
        // Punctuation in Chinese and Japanese is not a word change.
        assert!(!proofread_drift("我想问一下那个会能不能改到周四下午开因为周三我真的去不了", "我想问一下，那个会能不能改到周四下午开？因为周三我真的去不了。"));
        assert!(!proofread_drift("あの会議ですが木曜日の午後にできますか", "あの会議ですが、木曜日の午後にできますか。"));
    }

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
        // Markdown hard breaks from the model (seen with Groq in Word): no spaces left at line ends,
        // the paragraph break itself kept; unless the selection had such lines.
        assert_eq!(
            clean_command_output("Oi, amanhã às 10h.  \nNota: segue em anexo.", "oi amanha as 10h\r\nNota: segue em anexo\r\n"),
            "Oi, amanhã às 10h.\nNota: segue em anexo.\n"
        );
        assert_eq!(clean_command_output("A.  \n\nB.", "a\n\nb"), "A.\n\nB.");
        assert_eq!(clean_command_output("linha 1  \nlinha 2", "linha 1  \nlinha 2"), "linha 1  \nlinha 2");
    }

    #[test]
    fn translated_command_drops_echo_and_second_versions() {
        let source = "Prompt: VAZOU UMA FOTO DO FLAGRA | 20 Anos";
        let drop = |out: &str| drop_echoed_source(out, source);
        assert_eq!(drop(&format!("{source}\n\nPrompt: A PHOTO OF FLAGRA LEAKED | 20 Years")), "Prompt: A PHOTO OF FLAGRA LEAKED | 20 Years");
        assert_eq!(drop(&format!("Good morning\n{source}")), "Good morning");
        assert_eq!(drop("A PHOTO LEAKED"), "A PHOTO LEAKED");
        // The translation twice, a second labeled version (seen with gpt-oss-20b).
        assert_eq!(drop("Prompt: A photo leaked | Comedy\n\nTranslated Prompt: A photo leaked | Comedy"), "Prompt: A photo leaked | Comedy");
        assert_eq!(
            drop("Prompt: A photo leaked | Comedy\n\nImproved Prompt: \"Photo leak, comedic, full HD movie\""),
            "Prompt: A photo leaked | Comedy"
        );
        // The echo goes first, then the labeled version.
        assert_eq!(drop(&format!("{source}\n\nPrompt: A photo leaked\n\nVersão melhorada: foto")), "Prompt: A photo leaked");
        // A "---" line and a comment after it.
        assert_eq!(drop("Prompt: A photo leaked\n\n--- \n\nImproved Prompt (English):  \n\"Photo leaked\""), "Prompt: A photo leaked");
        // Two different paragraphs stay, and so does a label from a selection of several lines.
        assert_eq!(drop("First paragraph here.\n\nSecond one, different."), "First paragraph here.\n\nSecond one, different.");
        assert_eq!(drop_echoed_source("Hi team,\n\nAgenda: budget", "Oi time,\n\nPauta: orçamento"), "Hi team,\n\nAgenda: budget");
        // Only the source back: kept (the caller decides).
        assert_eq!(drop(&format!("{source}  ")), source);
        // A "---" that was in the selection is content.
        assert_eq!(drop_echoed_source("Title\n---\nBody text", "Título\n---\nCorpo"), "Title\n---\nBody text");
    }

    #[test]
    fn command_output_drops_a_trailing_note() {
        assert_eq!(clean_command_output("Prompt: A photo\n\n(Note: The text is translated.)", "Prompt: uma foto"), "Prompt: A photo");
        assert_eq!(clean_command_output("Bom dia.\n\nNota: deixei mais formal.", "bom dia"), "Bom dia.");
        // The selection ended with one: it's content.
        assert_eq!(clean_command_output("Texto.\n\n(veja o anexo)", "texto\n\n(ver anexo)"), "Texto.\n\n(veja o anexo)");
        // Nothing but a note: kept.
        assert_eq!(clean_command_output("(vazio)", "x"), "(vazio)");
        assert_eq!(clean_command_output("Line", "  line\n"), "  Line\n");
    }

    #[test]
    fn command_cleanup_never_panics_on_odd_text() {
        // Multi-byte text everywhere: every cut must land on a char boundary.
        let alphabet: Vec<char> = "ab :.-\n\n\"“”()çã明ค مر---Note:Nota:".chars().collect();
        let mut seed = 0x2545F491u32;
        let mut text = |len: usize| -> String {
            (0..len)
                .map(|_| {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    alphabet[seed as usize % alphabet.len()]
                })
                .collect()
        };
        for _ in 0..3000 {
            let (a, b) = (text(120), text(50));
            let _ = drop_echoed_source(&a, &b);
            let _ = clean_command_output(&a, &b);
        }
    }

    #[test]
    fn dictations_in_a_row_get_a_space() {
        assert!(needs_join_space(Some('.'), "Then we go."));
        assert!(needs_join_space(Some('a'), "amanhã"));
        assert!(needs_join_space(Some('?'), "“Sim”, disse."));
        assert!(needs_join_space(Some('.'), "¿Vienes?"));
        assert!(needs_join_space(Some('.'), "مرحبا"));
        assert!(!needs_join_space(None, "Bom dia."));
        assert!(!needs_join_space(Some(' '), "Bom dia."));
        assert!(!needs_join_space(Some('\n'), "Bom dia."));
        assert!(!needs_join_space(Some('.'), " Bom dia."));
        assert!(!needs_join_space(Some('.'), "\nBom dia."));
        assert!(!needs_join_space(Some('a'), ", e depois"));
        assert!(!needs_join_space(Some('a'), "."));
        assert!(!needs_join_space(Some('('), "nota"));
        assert!(!needs_join_space(Some('.'), ""));
        assert!(!needs_join_space(Some('。'), "明日は"));
        assert!(!needs_join_space(Some('.'), "明日は"));
        assert!(!needs_join_space(Some('ค'), "ครับ"));
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
        assert!(echoes_prompt("Cidades, empresas e produtos.", "nomes de pessoas, cidades, empresas e produtos escritos"));
        assert!(!echoes_prompt("Abre o relatório de vendas", "nomes de pessoas, cidades, empresas e produtos"));
        assert!(is_hallucination_voiced("Obrigado!", 0.3));
    }

    #[test]
    fn fillers_keep_meaning() {
        assert_eq!(remove_fillers("Expliquei a regra, entendeu?", "pt"), "Expliquei a regra, entendeu?");
        assert_eq!(remove_fillers("ella ha comido", "es"), "Ella ha comido");
        assert_eq!(remove_fillers("não não, eu não vou", "pt"), "Não não, eu não vou");
        assert_eq!(strip_annotations(".NET é rápido"), ".NET é rápido");
        // Punctuation that starts a word isn't loose.
        assert_eq!(strip_annotations("feito com .NET e o arquivo .env , pronto ."), "feito com .NET e o arquivo .env, pronto.");
        assert_eq!(strip_annotations("a, .NET"), "a, .NET");
        assert_eq!(strip_annotations("sim, . depois"), "sim. depois");
        assert_eq!(strip_annotations(", olá"), "Olá".to_lowercase());
    }

    #[test]
    fn annotations() {
        assert_eq!(strip_annotations("[Música] Olá (risos) pessoal [BLANK_AUDIO]"), "Olá pessoal");
    }

    #[test]
    fn prompt_refusal() {
        assert!(is_prompt_refusal("Não foi possível transcrever."));
        assert!(is_prompt_refusal("  Não foi possível transcrever  "));
        assert!(!is_prompt_refusal("Não foi possível transcrever o áudio de ontem."));
    }

    #[test]
    fn llm_cleanup() {
        assert_eq!(clean_llm_output("\"Olá mundo\""), "Olá mundo");
        assert_eq!(clean_llm_output("<think>x</think>Oi"), "Oi");
    }

    // Windows apps copy the selection with "\r\n"; models answer with "\n".
    #[test]
    fn crlf_selection_matches_lf_output() {
        assert_eq!(lf_newlines("a\r\n\r\nb\rc"), "a\n\nb\nc");
        assert_eq!(lf_newlines("sem quebra"), "sem quebra");
        // The selection's own final note or parenthetical paragraph stays.
        let sel = "texto revisado\r\n\r\n(veja o anexo)";
        assert_eq!(clean_command_output("Texto revisado.\n\n(veja o anexo)", sel), "Texto revisado.\n\n(veja o anexo)");
        let sel = "Bom dia equipe, segue o relatorio.\r\n\r\nNota: amanha nao teremos reuniao.";
        let out = clean_command_output("Bom dia, equipe, segue o relatório.\n\nNota: amanhã não teremos reunião.", sel);
        assert!(out.ends_with("Nota: amanhã não teremos reunião."), "{out:?}");
        // Its trailing line break comes back (as "\n": the paste turns it into "\r\n").
        assert_eq!(clean_command_output("Oi.", "oi\r\n"), "Oi.\n");
        // A note the model added is still dropped.
        assert_eq!(clean_command_output("Oi, tudo bem?\n\n(Nota: traduzido.)", "oi tudo bem\r\n"), "Oi, tudo bem?\n");
        // An echoed CRLF selection is still recognized.
        assert_eq!(drop_echoed_source("Primeira linha.\nSegunda.\n\nFirst line.\nSecond.", "Primeira linha.\r\nSegunda."), "First line.\nSecond.");
        // A correct multi-paragraph selection: nothing to fix, nothing pasted.
        let sel = "Bom dia.\r\n\r\nTudo certo por aqui.";
        let out = clean_command_output("Bom dia.\n\nTudo certo por aqui.", sel);
        assert!(proofread_untouched(sel, &out));
        assert!(!proofread_untouched(sel, "Bom dia!\n\nTudo certo por aqui."));
    }
}
