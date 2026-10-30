//! The app a dictation is going to: exe-name normalization, friendly names, web apps open in a
//! browser (from the window title) and the per-app style rules ("Estilos por aplicativo").
//! Keep KNOWN/ALIASES in sync with src/lib/apps.ts (the settings UI uses the same ids).

use crate::settings::Settings;

/// The app that had focus when a recording started.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppContext {
    /// Normalized id ("outlook", "slack", "code", "gmail"…): what rules match against.
    pub id: String,
    /// Friendly name for the UI and the LLM ("Outlook", "VS Code").
    pub name: String,
    /// For a web app detected in a browser tab: the browser's id (rules for it still apply).
    pub browser: Option<String>,
}

/// Same family, different exe names → one id.
const ALIASES: &[(&str, &str)] = &[
    ("olk", "outlook"),
    ("hxoutlook", "outlook"),
    ("ms-teams", "teams"),
    ("msteams", "teams"),
    ("microsoft teams", "teams"),
    ("whatsapp.root", "whatsapp"),
    ("whatsappdesktop", "whatsapp"),
    ("telegram desktop", "telegram"),
    ("discordptb", "discord"),
    ("discordcanary", "discord"),
    ("winword", "word"),
    ("code - insiders", "code"),
    ("vscode", "code"),
    ("vs code", "code"),
    ("visual studio code", "code"),
    ("google chrome", "chrome"),
    ("microsoft edge", "msedge"),
    ("edge", "msedge"),
    ("mozilla firefox", "firefox"),
    ("brave browser", "brave"),
    ("chatgpt desktop", "chatgpt"),
    ("claude desktop", "claude"),
    ("notepad++", "notepad++"),
];

/// Friendly names of common apps (anything else is shown capitalized).
const KNOWN: &[(&str, &str)] = &[
    ("outlook", "Outlook"),
    ("thunderbird", "Thunderbird"),
    ("gmail", "Gmail"),
    ("mailspring", "Mailspring"),
    ("slack", "Slack"),
    ("discord", "Discord"),
    ("whatsapp", "WhatsApp"),
    ("telegram", "Telegram"),
    ("teams", "Microsoft Teams"),
    ("signal", "Signal"),
    ("code", "VS Code"),
    ("cursor", "Cursor"),
    ("windsurf", "Windsurf"),
    ("zed", "Zed"),
    ("claude", "Claude"),
    ("chatgpt", "ChatGPT"),
    ("gemini", "Gemini"),
    ("chrome", "Chrome"),
    ("msedge", "Edge"),
    ("firefox", "Firefox"),
    ("brave", "Brave"),
    ("opera", "Opera"),
    ("vivaldi", "Vivaldi"),
    ("arc", "Arc"),
    ("word", "Word"),
    ("excel", "Excel"),
    ("powerpnt", "PowerPoint"),
    ("onenote", "OneNote"),
    ("notion", "Notion"),
    ("obsidian", "Obsidian"),
    ("linkedin", "LinkedIn"),
    ("notepad", "Bloco de Notas"),
    ("notepad++", "Notepad++"),
    ("windowsterminal", "Terminal"),
    ("explorer", "Explorador de Arquivos"),
];

const BROWSERS: &[&str] = &["chrome", "msedge", "firefox", "brave", "opera", "vivaldi", "arc", "chromium", "zen", "librewolf"];

/// Web apps recognized by a whole segment of the tab title ("Inbox (3) - me@x.com - Gmail").
const WEB_APPS: &[(&str, &[&str])] = &[
    ("gmail", &["gmail"]),
    ("outlook", &["outlook", "outlook web"]),
    ("whatsapp", &["whatsapp", "whatsapp web"]),
    ("slack", &["slack"]),
    ("discord", &["discord"]),
    ("chatgpt", &["chatgpt"]),
    ("claude", &["claude"]),
    ("gemini", &["gemini", "google gemini"]),
    ("teams", &["microsoft teams", "teams"]),
    ("telegram", &["telegram", "telegram web"]),
    ("notion", &["notion"]),
    ("linkedin", &["linkedin"]),
];

/// "C:\\Program Files\\Microsoft Office\\OUTLOOK.EXE" / "Slack.exe" / "Outlook" → "outlook".
pub fn normalize(exe_or_name: &str) -> String {
    let base = exe_or_name.trim().rsplit(['\\', '/']).next().unwrap_or("").trim().to_lowercase();
    let base = base.strip_suffix(".exe").unwrap_or(&base).trim().to_string();
    match ALIASES.iter().find(|(from, _)| *from == base) {
        Some((_, to)) => to.to_string(),
        None => base,
    }
}

pub fn friendly_name(id: &str) -> String {
    if let Some((_, name)) = KNOWN.iter().find(|(k, _)| *k == id) {
        return name.to_string();
    }
    let mut c = id.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn is_browser(id: &str) -> bool {
    BROWSERS.contains(&id)
}

/// Terminals: Ctrl+C there interrupts the running program instead of copying, and a paste
/// doesn't replace a selection, so "Comando" refuses them.
pub fn is_terminal(id: &str) -> bool {
    const TERMINALS: &[&str] = &[
        "windowsterminal",
        "openconsole",
        "conhost",
        "cmd",
        "powershell",
        "pwsh",
        "mintty",
        "putty",
        "wezterm",
        "wezterm-gui",
        "alacritty",
        "kitty",
        "konsole",
        "gnome-terminal-server",
        "gnome-terminal",
        "xterm",
        "terminator",
        "tilix",
        "xfce4-terminal",
        "tabby",
        "hyper",
        "warp",
        "ghostty",
    ];
    TERMINALS.contains(&id)
}

/// A known web app open in the active browser tab, from a whole title segment. Conservative on
/// purpose: "How to use Slack - Google Search" is not Slack.
pub fn web_app_from_title(title: &str) -> Option<&'static str> {
    let mut unified = title.to_string();
    for sep in [" - ", " | ", " – ", " — ", " · "] {
        unified = unified.replace(sep, "\u{1}");
    }
    let segs: Vec<String> = unified
        .split('\u{1}')
        .map(|s| {
            // unread counters: "(3) WhatsApp", "WhatsApp (3)"
            let s = s.trim().trim_start_matches(|c: char| c == '(' || c.is_ascii_digit() || c == ')' || c == '+').trim();
            let s = s.trim_end_matches(|c: char| c == ')' || c.is_ascii_digit() || c == '(' || c == '+').trim();
            s.replace('\u{200b}', "").to_lowercase()
        })
        .filter(|s| !s.is_empty())
        .collect();
    if segs.is_empty() {
        return None;
    }
    // First segment, or one of the last three (the browser name and a profile come last).
    let candidates = std::iter::once(&segs[0]).chain(segs.iter().rev().take(3));
    for seg in candidates {
        if let Some((id, _)) = WEB_APPS.iter().find(|(_, names)| names.contains(&seg.as_str())) {
            return Some(id);
        }
    }
    None
}

/// Builds the context from the focused process' exe and window title.
pub fn detect(exe: &str, title: &str) -> Option<AppContext> {
    let id = normalize(exe);
    if id.is_empty() {
        return None;
    }
    if is_browser(&id) {
        if let Some(web) = web_app_from_title(title) {
            return Some(AppContext { id: web.to_string(), name: friendly_name(web), browser: Some(id) });
        }
    }
    Some(AppContext { name: friendly_name(&id), id, browser: None })
}

/// Style a per-app rule picks for this app (rule for the web app first, then for its browser).
/// None when rules are off, nothing matches or the rule points at a style that no longer exists.
pub fn rule_style<'a>(s: &'a Settings, ctx: &AppContext) -> Option<&'a str> {
    if !s.app_styles_enabled {
        return None;
    }
    let find = |id: &str| {
        s.app_styles
            .iter()
            .find(|r| r.app == id && s.styles.iter().any(|st| st.id == r.style_id))
            .map(|r| r.style_id.as_str())
    };
    find(&ctx.id).or_else(|| ctx.browser.as_deref().and_then(find))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AppStyle;

    #[test]
    fn normalizes_exe_paths_and_aliases() {
        assert_eq!(normalize(r"C:\Program Files\Microsoft Office\root\Office16\OUTLOOK.EXE"), "outlook");
        assert_eq!(normalize("olk.exe"), "outlook");
        assert_eq!(normalize("Slack.exe"), "slack");
        assert_eq!(normalize("  Code.exe "), "code");
        assert_eq!(normalize("Code - Insiders.exe"), "code");
        assert_eq!(normalize("/usr/lib/slack/slack"), "slack");
        assert_eq!(normalize("ms-teams.exe"), "teams");
        assert_eq!(normalize("Google Chrome"), "chrome");
        assert_eq!(normalize("WINWORD.EXE"), "word");
        assert_eq!(normalize("Notion"), "notion");
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn friendly_names() {
        assert_eq!(friendly_name("code"), "VS Code");
        assert_eq!(friendly_name("whatsapp"), "WhatsApp");
        assert_eq!(friendly_name("figma"), "Figma");
        assert_eq!(friendly_name(""), "");
    }

    #[test]
    fn web_apps_from_titles() {
        assert_eq!(web_app_from_title("Caixa de entrada (3) - julio@gmail.com - Gmail - Google Chrome"), Some("gmail"));
        assert_eq!(web_app_from_title("(2) WhatsApp - Google Chrome"), Some("whatsapp"));
        assert_eq!(web_app_from_title("WhatsApp"), Some("whatsapp"));
        assert_eq!(web_app_from_title("Plano do projeto - Claude - Pessoal - Microsoft\u{200b} Edge"), Some("claude"));
        assert_eq!(web_app_from_title("geral - Acme - Slack — Mozilla Firefox"), Some("slack"));
        assert_eq!(web_app_from_title("Discord | #general | Servidor - Brave"), Some("discord"));
        assert_eq!(web_app_from_title("How to use Slack - Google Search - Google Chrome"), None);
        assert_eq!(web_app_from_title("Nova guia - Google Chrome"), None);
        assert_eq!(web_app_from_title(""), None);
    }

    #[test]
    fn detect_prefers_web_app_in_browsers_only() {
        let c = detect("chrome.exe", "Inbox - Gmail - Google Chrome").unwrap();
        assert_eq!((c.id.as_str(), c.name.as_str(), c.browser.as_deref()), ("gmail", "Gmail", Some("chrome")));
        // A desktop app's title never turns it into something else.
        let c = detect("slack.exe", "Gmail - Slack").unwrap();
        assert_eq!((c.id.as_str(), c.browser), ("slack", None));
        assert_eq!(detect("", "x"), None);
    }

    #[test]
    fn terminals() {
        assert!(is_terminal(&normalize("WindowsTerminal.exe")));
        assert!(is_terminal(&normalize("/usr/libexec/gnome-terminal-server")));
        assert!(!is_terminal(&normalize("Code.exe")));
    }

    fn settings_with(rules: &[(&str, &str)]) -> Settings {
        Settings {
            app_styles: rules.iter().map(|(a, st)| AppStyle { app: a.to_string(), style_id: st.to_string() }).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn rule_matching() {
        let s = settings_with(&[("outlook", "professional"), ("chrome", "natural"), ("slack", "gone-style")]);
        let ctx = |exe: &str, title: &str| detect(exe, title).unwrap();
        assert_eq!(rule_style(&s, &ctx("OUTLOOK.EXE", "")), Some("professional"));
        assert_eq!(rule_style(&s, &ctx("olk.exe", "")), Some("professional"));
        // Web Outlook in Chrome: the Outlook rule wins over the Chrome one.
        assert_eq!(rule_style(&s, &ctx("chrome.exe", "Email - Outlook - Google Chrome")), Some("professional"));
        // Any other tab: the browser rule.
        assert_eq!(rule_style(&s, &ctx("chrome.exe", "Notícias - Google Chrome")), Some("natural"));
        // Rule pointing at a deleted style is ignored.
        assert_eq!(rule_style(&s, &ctx("slack.exe", "")), None);
        assert_eq!(rule_style(&s, &ctx("notepad.exe", "")), None);
        let mut off = s.clone();
        off.app_styles_enabled = false;
        assert_eq!(rule_style(&off, &ctx("outlook.exe", "")), None);
    }
}
