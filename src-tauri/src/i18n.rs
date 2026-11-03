//! What the core says to the user (tray menu, errors and warnings shown in the UI), in the
//! interface language. The Portuguese text is the key: `tr("Nenhuma fala detectada.")`. The UI's
//! own strings are in src/lib/locales; the prompts sent to the AI stay in Portuguese on purpose.

use std::fmt::Display;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Same order as the columns of `T`.
pub const LANGS: [&str; 10] = ["pt", "en", "es", "fr", "de", "ja", "zh", "ru", "hi", "bn"];

/// Index in LANGS of the language in use.
static LANG: AtomicUsize = AtomicUsize::new(0);

/// `pref` is the setting: a language, or "auto" (Windows' display language).
pub fn set_language(pref: &str) {
    let code = if pref == "auto" { system_language() } else { pref.to_string() };
    let i = LANGS.iter().position(|l| *l == code).unwrap_or(1);
    LANG.store(i, Ordering::Relaxed);
}

/// Windows' display language, when the app has it; English otherwise.
pub fn system_language() -> String {
    let base = system_locale().split(['-', '_']).next().unwrap_or("").to_lowercase();
    if LANGS.contains(&base.as_str()) {
        base
    } else {
        "en".into()
    }
}

#[cfg(windows)]
fn system_locale() -> String {
    use windows::Win32::Globalization::{GetUserDefaultUILanguage, LCIDToLocaleName};
    let mut buf = [0u16; 85];
    let n = unsafe { LCIDToLocaleName(GetUserDefaultUILanguage() as u32, Some(&mut buf), 0) };
    String::from_utf16_lossy(&buf[..(n.max(1) as usize - 1)])
}

#[cfg(not(windows))]
fn system_locale() -> String {
    std::env::var("LC_ALL").or_else(|_| std::env::var("LANG")).unwrap_or_default()
}

/// The text in the language in use (the key itself when it isn't in the table).
pub fn tr(pt: &str) -> String {
    let i = LANG.load(Ordering::Relaxed);
    T.iter().find(|row| row[0] == pt).map_or(pt, |row| row[i]).to_string()
}

/// `tr` with each "{}" replaced by the next argument.
pub fn trf(pt: &str, args: &[&dyn Display]) -> String {
    let mut s = tr(pt);
    for a in args {
        s = s.replacen("{}", &a.to_string(), 1);
    }
    s
}

#[rustfmt::skip]
const T: &[[&str; 10]] = &[
    // Tray menu
    ["Ditar agora", "Dictate now", "Dictar ahora", "Dicter maintenant", "Jetzt diktieren", "今すぐ音声入力", "立即听写", "Диктовать сейчас", "अभी डिक्टेट करें", "এখনই ডিকটেট করুন"],
    ["Colar último ditado", "Paste last dictation", "Pegar el último dictado", "Coller la dernière dictée", "Letztes Diktat einfügen", "最後の音声入力を貼り付け", "粘贴上一次听写", "Вставить последнюю диктовку", "पिछला डिक्टेशन पेस्ट करें", "শেষ ডিকটেশন পেস্ট করুন"],
    ["Estilo", "Style", "Estilo", "Style", "Stil", "スタイル", "风格", "Стиль", "शैली", "স্টাইল"],
    ["Configurações", "Settings", "Ajustes", "Paramètres", "Einstellungen", "設定", "设置", "Настройки", "सेटिंग्स", "সেটিংস"],
    ["Histórico", "History", "Historial", "Historique", "Verlauf", "履歴", "历史记录", "История", "इतिहास", "ইতিহাস"],
    ["Refazer configuração inicial", "Run the initial setup again", "Repetir la configuración inicial", "Refaire la configuration initiale", "Ersteinrichtung wiederholen", "初期設定をやり直す", "重新进行初始设置", "Повторить начальную настройку", "शुरुआती सेटअप दोबारा करें", "প্রাথমিক সেটআপ আবার করুন"],
    ["Reportar problema ou sugerir melhoria", "Report a problem or suggest an improvement", "Informar un problema o sugerir una mejora", "Signaler un problème ou suggérer une amélioration", "Problem melden oder Verbesserung vorschlagen", "問題の報告・改善の提案", "报告问题或提出改进建议", "Сообщить о проблеме или предложить улучшение", "समस्या बताएं या सुधार सुझाएं", "সমস্যা জানান বা উন্নতির পরামর্শ দিন"],
    ["Sair do FlowVoice", "Quit FlowVoice", "Salir de FlowVoice", "Quitter FlowVoice", "FlowVoice beenden", "FlowVoice を終了", "退出 FlowVoice", "Выйти из FlowVoice", "FlowVoice बंद करें", "FlowVoice বন্ধ করুন"],
    ["FlowVoice — ditado por voz", "FlowVoice — voice dictation", "FlowVoice — dictado por voz", "FlowVoice — dictée vocale", "FlowVoice — Sprachdiktat", "FlowVoice — 音声入力", "FlowVoice — 语音听写", "FlowVoice — голосовой ввод", "FlowVoice — वॉइस डिक्टेशन", "FlowVoice — ভয়েস ডিকটেশন"],
    // Built-in styles
    ["Natural", "Natural", "Natural", "Naturel", "Natürlich", "自然", "自然", "Естественный", "स्वाभाविक", "স্বাভাবিক"],
    ["Profissional", "Professional", "Profesional", "Professionnel", "Professionell", "ビジネス", "专业", "Деловой", "पेशेवर", "পেশাদার"],
    ["Prompt", "Prompt", "Prompt", "Prompt", "Prompt", "プロンプト", "提示词", "Промпт", "प्रॉम्प्ट", "প্রম্পট"],
    ["Cru", "Raw", "Crudo", "Brut", "Roh", "そのまま", "原始", "Без обработки", "कच्चा", "কাঁচা"],
    ["Só transcrição", "Transcript only", "Solo transcripción", "Transcription seule", "Nur Transkript", "文字起こしのみ", "仅转写", "Только расшифровка", "केवल ट्रांसक्रिप्ट", "শুধু ট্রান্সক্রিপ্ট"],
    ["Corrige e limpa a fala mantendo seu jeito de falar.", "Cleans up your speech and keeps your way of talking.", "Corrige y limpia lo que dices y mantiene tu forma de hablar.", "Corrige et nettoie vos propos en gardant votre façon de parler.", "Korrigiert und glättet das Gesprochene und behält deine Art zu reden bei.", "話し方を残したまま、発話を修正して整えます。", "修正并整理你说的话,保留你的说话方式。", "Исправляет и очищает речь, сохраняя вашу манеру говорить.", "आपके बोलने के अंदाज़ को बनाए रखते हुए बोली को सुधारता और साफ़ करता है।", "আপনার কথা বলার ধরন বজায় রেখে কথা ঠিক ও পরিষ্কার করে।"],
    ["Texto formal e organizado, pronto para e-mail e trabalho.", "Formal, organized text, ready for email and work.", "Texto formal y ordenado, listo para el correo y el trabajo.", "Texte formel et structuré, prêt pour les e-mails et le travail.", "Formeller, strukturierter Text, bereit für E-Mail und Arbeit.", "メールや仕事にそのまま使える、整った丁寧な文章にします。", "正式、有条理的文字,可直接用于邮件和工作。", "Официальный и упорядоченный текст, готовый для почты и работы.", "औपचारिक और व्यवस्थित टेक्स्ट, ईमेल और काम के लिए तैयार।", "আনুষ্ঠানিক ও গোছানো লেখা, ইমেল ও কাজের জন্য প্রস্তুত।"],
    ["Enxuga a fala para mandar a uma IA: menos tokens, mesmo contexto.", "Trims your speech to send to an AI: fewer tokens, same context.", "Resume lo que dices para enviarlo a una IA: menos tokens, mismo contexto.", "Resserre vos propos pour les envoyer à une IA : moins de jetons, même contexte.", "Kürzt das Gesprochene für eine KI: weniger Tokens, gleicher Kontext.", "AI に送るために発話を簡潔にします。トークンは少なく、文脈はそのまま。", "精简你说的话以发送给 AI:更少的令牌,相同的上下文。", "Сжимает речь для отправки ИИ: меньше токенов, тот же контекст.", "बोली को AI को भेजने के लिए छोटा करता है: कम टोकन, वही संदर्भ।", "AI-কে পাঠাতে কথা সংক্ষিপ্ত করে: কম টোকেন, একই প্রসঙ্গ।"],
    ["Só as palavras: sem pontuação, sem quebras de linha, sem IA.", "Only the words: no punctuation, no line breaks, no AI.", "Solo las palabras: sin puntuación, sin saltos de línea, sin IA.", "Seulement les mots : ni ponctuation, ni sauts de ligne, ni IA.", "Nur die Wörter: keine Satzzeichen, keine Zeilenumbrüche, keine KI.", "言葉だけ。句読点も改行も AI もなし。", "只有文字:没有标点、没有换行、不用 AI。", "Только слова: без пунктуации, переносов строк и ИИ.", "सिर्फ़ शब्द: न विराम चिह्न, न नई पंक्तियां, न AI।", "শুধু শব্দ: যতিচিহ্ন, লাইন বিরতি বা AI ছাড়া।"],
    ["A transcrição exatamente como saiu, sem passar por IA.", "The transcript exactly as it came, without AI.", "La transcripción tal como salió, sin pasar por IA.", "La transcription telle quelle, sans passer par l'IA.", "Das Transkript genau so, wie es kam, ohne KI.", "文字起こしをそのまま、AI を通さずに使います。", "转写结果原样输出,不经过 AI。", "Расшифровка как есть, без ИИ.", "ट्रांसक्रिप्ट ठीक जैसा आया वैसा, बिना AI के।", "ট্রান্সক্রিপ্ট ঠিক যেমন এসেছে, AI ছাড়া।"],
    // Dictation and Command
    ["Selecione um texto antes de usar o Comando.", "Select some text before using Command.", "Selecciona un texto antes de usar Comando.", "Sélectionnez du texte avant d'utiliser Commande.", "Wähle Text aus, bevor du den Befehl verwendest.", "コマンドを使う前にテキストを選択してください。", "使用命令前请先选择文字。", "Выделите текст, прежде чем использовать команду.", "कमांड इस्तेमाल करने से पहले कोई टेक्स्ट चुनें।", "কমান্ড ব্যবহারের আগে কিছু লেখা নির্বাচন করুন।"],
    ["O Comando não funciona em terminais (lá o Ctrl+C interrompe o programa). Use num editor ou campo de texto.", "Command doesn't work in terminals (Ctrl+C stops the program there). Use it in an editor or a text field.", "Comando no funciona en terminales (allí Ctrl+C detiene el programa). Úsalo en un editor o un campo de texto.", "Commande ne fonctionne pas dans les terminaux (Ctrl+C y arrête le programme). Utilisez-la dans un éditeur ou un champ de texte.", "Der Befehl funktioniert nicht in Terminals (Strg+C beendet dort das Programm). Nutze ihn in einem Editor oder Textfeld.", "コマンドはターミナルでは使えません(Ctrl+C でプログラムが止まるため)。エディターやテキスト欄で使ってください。", "命令在终端中不可用(Ctrl+C 会中断程序)。请在编辑器或文本框中使用。", "Команда не работает в терминалах (там Ctrl+C останавливает программу). Используйте её в редакторе или текстовом поле.", "कमांड टर्मिनल में काम नहीं करता (वहां Ctrl+C प्रोग्राम रोक देता है)। इसे एडिटर या टेक्स्ट फ़ील्ड में इस्तेमाल करें।", "কমান্ড টার্মিনালে কাজ করে না (সেখানে Ctrl+C প্রোগ্রাম থামিয়ে দেয়)। এডিটর বা টেক্সট ফিল্ডে ব্যবহার করুন।"],
    ["A seleção é grande demais para o Comando (máximo de {} mil caracteres).", "The selection is too long for Command ({} thousand characters at most).", "La selección es demasiado larga para Comando (máximo {} mil caracteres).", "La sélection est trop longue pour Commande ({} mille caractères au maximum).", "Die Auswahl ist zu lang für den Befehl (höchstens {} Tausend Zeichen).", "選択範囲がコマンドには長すぎます(最大 {} 千文字)。", "所选内容对命令来说太长(最多 {} 千字符)。", "Выделенный текст слишком длинный для команды (максимум {} тыс. символов).", "चयन कमांड के लिए बहुत लंबा है (अधिकतम {} हज़ार अक्षर)।", "নির্বাচন কমান্ডের জন্য খুব দীর্ঘ (সর্বোচ্চ {} হাজার অক্ষর)।"],
    ["Demorou demais — os serviços não responderam. Tente de novo.", "It took too long: the services didn't respond. Try again.", "Tardó demasiado: los servicios no respondieron. Inténtalo de nuevo.", "C'est trop long : les services n'ont pas répondu. Réessayez.", "Das hat zu lange gedauert: Die Dienste haben nicht geantwortet. Versuche es erneut.", "時間がかかりすぎました。サービスが応答しません。もう一度お試しください。", "耗时过长:服务没有响应。请重试。", "Слишком долго: сервисы не ответили. Попробуйте ещё раз.", "बहुत समय लगा: सेवाओं ने जवाब नहीं दिया। फिर से कोशिश करें।", "খুব বেশি সময় লেগেছে: পরিষেবাগুলো সাড়া দেয়নি। আবার চেষ্টা করুন।"],
    ["Demorou demais para transcrever — os serviços não responderam. Tente de novo.", "Transcribing took too long: the services didn't respond. Try again.", "La transcripción tardó demasiado: los servicios no respondieron. Inténtalo de nuevo.", "La transcription a pris trop de temps : les services n'ont pas répondu. Réessayez.", "Das Transkribieren hat zu lange gedauert: Die Dienste haben nicht geantwortet. Versuche es erneut.", "文字起こしに時間がかかりすぎました。サービスが応答しません。もう一度お試しください。", "转写耗时过长:服务没有响应。请重试。", "Расшифровка заняла слишком много времени: сервисы не ответили. Попробуйте ещё раз.", "ट्रांसक्राइब करने में बहुत समय लगा: सेवाओं ने जवाब नहीं दिया। फिर से कोशिश करें।", "ট্রান্সক্রাইব করতে খুব বেশি সময় লেগেছে: পরিষেবাগুলো সাড়া দেয়নি। আবার চেষ্টা করুন।"],
    ["Demorou demais para transcrever — os serviços não responderam.", "Transcribing took too long: the services didn't respond.", "La transcripción tardó demasiado: los servicios no respondieron.", "La transcription a pris trop de temps : les services n'ont pas répondu.", "Das Transkribieren hat zu lange gedauert: Die Dienste haben nicht geantwortet.", "文字起こしに時間がかかりすぎました。サービスが応答しません。", "转写耗时过长:服务没有响应。", "Расшифровка заняла слишком много времени: сервисы не ответили.", "ट्रांसक्राइब करने में बहुत समय लगा: सेवाओं ने जवाब नहीं दिया।", "ট্রান্সক্রাইব করতে খুব বেশি সময় লেগেছে: পরিষেবাগুলো সাড়া দেয়নি।"],
    ["Não deu para colar nessa janela — o texto está copiado, cole com Ctrl+V.", "Couldn't paste into that window: the text is copied, paste it with Ctrl+V.", "No se pudo pegar en esa ventana: el texto está copiado, pégalo con Ctrl+V.", "Impossible de coller dans cette fenêtre : le texte est copié, collez-le avec Ctrl+V.", "Einfügen in dieses Fenster nicht möglich: Der Text ist kopiert, füge ihn mit Strg+V ein.", "そのウィンドウには貼り付けられませんでした。テキストはコピー済みです。Ctrl+V で貼り付けてください。", "无法粘贴到该窗口:文字已复制,请按 Ctrl+V 粘贴。", "Не удалось вставить в это окно: текст скопирован, вставьте его через Ctrl+V.", "उस विंडो में पेस्ट नहीं हो सका: टेक्स्ट कॉपी हो गया है, Ctrl+V से पेस्ट करें।", "সেই উইন্ডোতে পেস্ট করা যায়নি: লেখা কপি করা আছে, Ctrl+V দিয়ে পেস্ট করুন।"],
    ["Não deu para substituir: o texto mudou depois da colagem ou a janela não aceita. A nova versão está copiada, cole com Ctrl+V.", "Couldn't replace it: the text changed after the paste or the window doesn't allow it. The new version is copied, paste it with Ctrl+V.", "No se pudo reemplazar: el texto cambió después de pegarlo o la ventana no lo permite. La nueva versión está copiada, pégala con Ctrl+V.", "Impossible de remplacer : le texte a changé après le collage ou la fenêtre ne l'accepte pas. La nouvelle version est copiée, collez-la avec Ctrl+V.", "Ersetzen nicht möglich: Der Text hat sich nach dem Einfügen geändert oder das Fenster erlaubt es nicht. Die neue Version ist kopiert, füge sie mit Strg+V ein.", "置き換えられませんでした。貼り付け後にテキストが変わったか、ウィンドウが対応していません。新しい版はコピー済みです。Ctrl+V で貼り付けてください。", "无法替换:粘贴后文字已更改,或该窗口不允许。新版本已复制,请按 Ctrl+V 粘贴。", "Не удалось заменить: текст изменился после вставки или окно этого не позволяет. Новая версия скопирована, вставьте её через Ctrl+V.", "बदला नहीं जा सका: पेस्ट के बाद टेक्स्ट बदल गया या विंडो अनुमति नहीं देती। नया संस्करण कॉपी हो गया है, Ctrl+V से पेस्ट करें।", "প্রতিস্থাপন করা যায়নি: পেস্টের পরে লেখা বদলেছে বা উইন্ডো অনুমতি দেয় না। নতুন সংস্করণ কপি করা আছে, Ctrl+V দিয়ে পেস্ট করুন।"],
    ["O microfone escolhido não foi encontrado — gravado com o microfone padrão do sistema.", "The chosen microphone wasn't found: the system's default one recorded.", "No se encontró el micrófono elegido: se grabó con el predeterminado del sistema.", "Le micro choisi est introuvable : l'enregistrement a utilisé celui par défaut du système.", "Das gewählte Mikrofon wurde nicht gefunden: Aufgenommen wurde mit dem Standardmikrofon.", "選択したマイクが見つかりません。システムの既定のマイクで録音しました。", "未找到所选麦克风:已使用系统默认麦克风录音。", "Выбранный микрофон не найден: запись велась с микрофона по умолчанию.", "चुना गया माइक्रोफ़ोन नहीं मिला: सिस्टम के डिफ़ॉल्ट माइक्रोफ़ोन से रिकॉर्ड किया गया।", "নির্বাচিত মাইক্রোফোন পাওয়া যায়নি: সিস্টেমের ডিফল্ট মাইক্রোফোনে রেকর্ড হয়েছে।"],
    ["O microfone falhou ou foi desconectado durante a gravação — o áudio pode estar incompleto.", "The microphone failed or was disconnected while recording: the audio may be incomplete.", "El micrófono falló o se desconectó durante la grabación: el audio puede estar incompleto.", "Le micro a échoué ou a été débranché pendant l'enregistrement : l'audio est peut-être incomplet.", "Das Mikrofon ist während der Aufnahme ausgefallen oder wurde getrennt: Das Audio ist möglicherweise unvollständig.", "録音中にマイクが故障または切断されました。音声が不完全な可能性があります。", "录音时麦克风出错或断开:音频可能不完整。", "Микрофон отказал или был отключён во время записи: аудио может быть неполным.", "रिकॉर्डिंग के दौरान माइक्रोफ़ोन विफल हुआ या डिस्कनेक्ट हो गया: ऑडियो अधूरा हो सकता है।", "রেকর্ডিংয়ের সময় মাইক্রোফোন ব্যর্থ বা বিচ্ছিন্ন হয়েছে: অডিও অসম্পূর্ণ হতে পারে।"],
    ["Erro interno ao processar o ditado. Tente de novo.", "Internal error while processing the dictation. Try again.", "Error interno al procesar el dictado. Inténtalo de nuevo.", "Erreur interne lors du traitement de la dictée. Réessayez.", "Interner Fehler beim Verarbeiten des Diktats. Versuche es erneut.", "音声入力の処理中に内部エラーが発生しました。もう一度お試しください。", "处理听写时发生内部错误。请重试。", "Внутренняя ошибка при обработке диктовки. Попробуйте ещё раз.", "डिक्टेशन प्रोसेस करते समय आंतरिक त्रुटि। फिर से कोशिश करें।", "ডিকটেশন প্রক্রিয়া করার সময় অভ্যন্তরীণ ত্রুটি। আবার চেষ্টা করুন।"],
    ["Nenhuma fala detectada — verifique o microfone.", "No speech detected. Check the microphone.", "No se detectó voz. Revisa el micrófono.", "Aucune parole détectée. Vérifiez le micro.", "Keine Sprache erkannt. Prüfe das Mikrofon.", "音声が検出されませんでした。マイクを確認してください。", "未检测到语音。请检查麦克风。", "Речь не обнаружена. Проверьте микрофон.", "कोई आवाज़ नहीं मिली। माइक्रोफ़ोन जांचें।", "কোনো কথা শনাক্ত হয়নি। মাইক্রোফোন পরীক্ষা করুন।"],
    ["Nenhuma fala detectada.", "No speech detected.", "No se detectó voz.", "Aucune parole détectée.", "Keine Sprache erkannt.", "音声が検出されませんでした。", "未检测到语音。", "Речь не обнаружена.", "कोई आवाज़ नहीं मिली।", "কোনো কথা শনাক্ত হয়নি।"],
    ["nenhuma IA configurada", "no AI provider set up", "ninguna IA configurada", "aucune IA configurée", "keine KI eingerichtet", "AI が未設定", "未配置 AI", "ИИ не настроен", "कोई AI सेट नहीं है", "কোনো AI সেট করা নেই"],
    ["Sem refinamento por IA ({}). Texto entregue limpo, sem IA.", "No AI rewrite ({}): the text was delivered cleaned, without AI.", "Sin refinamiento por IA ({}): el texto se entregó limpio, sin IA.", "Pas de réécriture par l'IA ({}) : le texte a été livré nettoyé, sans IA.", "Keine KI-Überarbeitung ({}): Der Text wurde bereinigt, ohne KI, geliefert.", "AI による書き直しなし({})。AI を使わず整えたテキストを出力しました。", "未进行 AI 润色({}):已输出清理后的文字,未使用 AI。", "Без обработки ИИ ({}): текст выдан очищенным, без ИИ.", "AI से दोबारा नहीं लिखा गया ({}): टेक्स्ट बिना AI के साफ़ करके दिया गया।", "AI দিয়ে পুনর্লিখন হয়নি ({}): AI ছাড়া পরিষ্কার করা লেখা দেওয়া হয়েছে।"],
    ["O Comando precisa de uma IA. Adicione uma chave grátis (Groq ou Gemini) em Configurações → Provedores.", "Command needs an AI. Add a free key (Groq or Gemini) in Settings → Providers.", "Comando necesita una IA. Añade una clave gratis (Groq o Gemini) en Ajustes → Proveedores.", "Commande a besoin d'une IA. Ajoutez une clé gratuite (Groq ou Gemini) dans Paramètres → Fournisseurs.", "Der Befehl braucht eine KI. Füge unter Einstellungen → Anbieter einen kostenlosen Schlüssel hinzu (Groq oder Gemini).", "コマンドには AI が必要です。設定 → プロバイダーで無料のキー(Groq または Gemini)を追加してください。", "命令需要 AI。请在 设置 → 服务商 中添加免费密钥(Groq 或 Gemini)。", "Для команды нужен ИИ. Добавьте бесплатный ключ (Groq или Gemini) в разделе Настройки → Провайдеры.", "कमांड को AI चाहिए। सेटिंग्स → प्रदाता में मुफ़्त कुंजी (Groq या Gemini) जोड़ें।", "কমান্ডের জন্য AI দরকার। সেটিংস → প্রদানকারীতে একটি বিনামূল্যের কী (Groq বা Gemini) যোগ করুন।"],
    ["Não foi possível aplicar o comando: {}", "Couldn't apply the command: {}", "No se pudo aplicar el comando: {}", "Impossible d'appliquer la commande : {}", "Der Befehl konnte nicht angewendet werden: {}", "コマンドを適用できませんでした: {}", "无法应用命令:{}", "Не удалось применить команду: {}", "कमांड लागू नहीं हो सका: {}", "কমান্ড প্রয়োগ করা যায়নি: {}"],
    ["nada para refazer", "nothing to redo", "nada que rehacer", "rien à refaire", "nichts zu wiederholen", "やり直すものがありません", "没有可重做的内容", "нечего переделывать", "दोबारा करने के लिए कुछ नहीं", "আবার করার মতো কিছু নেই"],
    ["esse ditado não está mais no histórico", "this dictation isn't in the history anymore", "este dictado ya no está en el historial", "cette dictée n'est plus dans l'historique", "dieses Diktat ist nicht mehr im Verlauf", "この音声入力はもう履歴にありません", "这条听写已不在历史记录中", "этой диктовки больше нет в истории", "यह डिक्टेशन अब इतिहास में नहीं है", "এই ডিকটেশন আর ইতিহাসে নেই"],
    ["perguntas não podem ser refeitas", "questions can't be redone", "las preguntas no se pueden rehacer", "les questions ne peuvent pas être refaites", "Fragen können nicht wiederholt werden", "質問はやり直せません", "问题无法重做", "вопросы нельзя переделать", "सवाल दोबारा नहीं किए जा सकते", "প্রশ্ন আবার করা যায় না"],
    ["a transcrição falhou: use \"Tentar de novo\"", "the transcription failed: use \"Try again\"", "la transcripción falló: usa \"Intentar de nuevo\"", "la transcription a échoué : utilisez « Réessayer »", "die Transkription ist fehlgeschlagen: nutze „Erneut versuchen“", "文字起こしに失敗しました。「再試行」を使ってください", "转写失败:请使用“重试”", "расшифровка не удалась: нажмите «Повторить»", "ट्रांसक्रिप्शन विफल रहा: \"फिर से कोशिश करें\" इस्तेमाल करें", "ট্রান্সক্রিপশন ব্যর্থ হয়েছে: \"আবার চেষ্টা করুন\" ব্যবহার করুন"],
    ["Nenhum ditado para colar ainda.", "Nothing dictated to paste yet.", "Todavía no hay ningún dictado que pegar.", "Aucune dictée à coller pour l'instant.", "Noch kein Diktat zum Einfügen.", "貼り付ける音声入力はまだありません。", "还没有可粘贴的听写。", "Пока нечего вставлять.", "पेस्ट करने के लिए अभी कोई डिक्टेशन नहीं है।", "পেস্ট করার মতো কোনো ডিকটেশন এখনও নেই।"],
    ["Nenhum modelo local baixado. Baixe um em Configurações → Transcrição.", "No local model downloaded. Download one in Settings → Transcription.", "Ningún modelo local descargado. Descarga uno en Ajustes → Transcripción.", "Aucun modèle local téléchargé. Téléchargez-en un dans Paramètres → Transcription.", "Kein lokales Modell heruntergeladen. Lade eines unter Einstellungen → Transkription herunter.", "ローカルモデルがありません。設定 → 文字起こしからダウンロードしてください。", "尚未下载本地模型。请在 设置 → 转写 中下载。", "Локальная модель не загружена. Скачайте её в разделе Настройки → Расшифровка.", "कोई लोकल मॉडल डाउनलोड नहीं है। सेटिंग्स → ट्रांसक्रिप्शन में एक डाउनलोड करें।", "কোনো লোকাল মডেল ডাউনলোড করা নেই। সেটিংস → ট্রান্সক্রিপশনে একটি ডাউনলোড করুন।"],
    ["Configure uma chave (Groq é grátis) ou baixe um modelo local para transcrever.", "Set up a key (Groq is free) or download a local model to transcribe.", "Configura una clave (Groq es gratis) o descarga un modelo local para transcribir.", "Configurez une clé (Groq est gratuit) ou téléchargez un modèle local pour transcrire.", "Richte einen Schlüssel ein (Groq ist kostenlos) oder lade ein lokales Modell zum Transkribieren herunter.", "文字起こしにはキーの設定(Groq は無料)か、ローカルモデルのダウンロードが必要です。", "请配置密钥(Groq 免费)或下载本地模型以进行转写。", "Настройте ключ (Groq бесплатен) или скачайте локальную модель для расшифровки.", "ट्रांसक्राइब करने के लिए कुंजी सेट करें (Groq मुफ़्त है) या लोकल मॉडल डाउनलोड करें।", "ট্রান্সক্রাইব করতে একটি কী সেট করুন (Groq বিনামূল্যে) বা লোকাল মডেল ডাউনলোড করুন।"],
    ["Falha na transcrição: {}", "Transcription failed: {}", "Falló la transcripción: {}", "Échec de la transcription : {}", "Transkription fehlgeschlagen: {}", "文字起こしに失敗しました: {}", "转写失败:{}", "Сбой расшифровки: {}", "ट्रांसक्रिप्शन विफल: {}", "ট্রান্সক্রিপশন ব্যর্থ: {}"],
    ["Nenhuma IA configurada. Adicione uma chave grátis (Groq ou Gemini) em Configurações → Provedores.", "No AI set up. Add a free key (Groq or Gemini) in Settings → Providers.", "Ninguna IA configurada. Añade una clave gratis (Groq o Gemini) en Ajustes → Proveedores.", "Aucune IA configurée. Ajoutez une clé gratuite (Groq ou Gemini) dans Paramètres → Fournisseurs.", "Keine KI eingerichtet. Füge unter Einstellungen → Anbieter einen kostenlosen Schlüssel hinzu (Groq oder Gemini).", "AI が未設定です。設定 → プロバイダーで無料のキー(Groq または Gemini)を追加してください。", "未配置 AI。请在 设置 → 服务商 中添加免费密钥(Groq 或 Gemini)。", "ИИ не настроен. Добавьте бесплатный ключ (Groq или Gemini) в разделе Настройки → Провайдеры.", "कोई AI सेट नहीं है। सेटिंग्स → प्रदाता में मुफ़्त कुंजी (Groq या Gemini) जोड़ें।", "কোনো AI সেট করা নেই। সেটিংস → প্রদানকারীতে একটি বিনামূল্যের কী (Groq বা Gemini) যোগ করুন।"],
    ["Não consegui responder agora: {}", "I couldn't answer right now: {}", "No pude responder ahora: {}", "Impossible de répondre pour l'instant : {}", "Ich konnte gerade nicht antworten: {}", "今は回答できませんでした: {}", "现在无法回答:{}", "Сейчас не удалось ответить: {}", "अभी जवाब नहीं दे सका: {}", "এখন উত্তর দেওয়া যায়নি: {}"],
    // Settings
    ["Não foi possível ativar: {}", "Couldn't turn on: {}", "No se pudo activar: {}", "Impossible d'activer : {}", "Konnte nicht aktiviert werden: {}", "有効にできませんでした: {}", "无法启用:{}", "Не удалось включить: {}", "चालू नहीं हो सका: {}", "চালু করা যায়নি: {}"],
    ["nenhuma chave salva", "no key saved", "ninguna clave guardada", "aucune clé enregistrée", "kein Schlüssel gespeichert", "保存されたキーがありません", "没有已保存的密钥", "нет сохранённого ключа", "कोई कुंजी सेव नहीं है", "কোনো কী সংরক্ষিত নেই"],
    ["não foi possível verificar: {}", "couldn't check: {}", "no se pudo comprobar: {}", "vérification impossible : {}", "Prüfung nicht möglich: {}", "確認できませんでした: {}", "无法检查:{}", "не удалось проверить: {}", "जांच नहीं हो सकी: {}", "যাচাই করা যায়নি: {}"],
    ["endereço inválido", "invalid address", "dirección no válida", "adresse non valide", "ungültige Adresse", "無効なアドレス", "地址无效", "недопустимый адрес", "अमान्य पता", "অবৈধ ঠিকানা"],
    ["atalho inválido \"{}\": {}", "invalid shortcut \"{}\": {}", "atajo no válido \"{}\": {}", "raccourci non valide « {} » : {}", "ungültiges Tastenkürzel „{}“: {}", "無効なショートカット「{}」: {}", "快捷键“{}”无效:{}", "недопустимое сочетание «{}»: {}", "अमान्य शॉर्टकट \"{}\": {}", "অবৈধ শর্টকাট \"{}\": {}"],
    ["use Ctrl, Alt ou Win junto (Shift sozinho só com tecla F) ou uma tecla F", "use Ctrl, Alt or Win with it (Shift alone only with an F key) or an F key", "usa Ctrl, Alt o Win junto (Shift solo con una tecla F) o una tecla F", "utilisez Ctrl, Alt ou Win avec (Maj seul uniquement avec une touche F) ou une touche F", "nutze Strg, Alt oder Win dazu (Umschalt allein nur mit einer F-Taste) oder eine F-Taste", "Ctrl、Alt、Win のいずれかと組み合わせるか(Shift のみは F キーとだけ)、F キーを使ってください", "请配合 Ctrl、Alt 或 Win 使用(仅 Shift 只能搭配 F 键),或使用 F 键", "используйте вместе с Ctrl, Alt или Win (только Shift — лишь с клавишей F) или клавишу F", "इसके साथ Ctrl, Alt या Win इस्तेमाल करें (सिर्फ़ Shift केवल F कुंजी के साथ) या कोई F कुंजी", "এর সঙ্গে Ctrl, Alt বা Win ব্যবহার করুন (শুধু Shift কেবল F কী-র সঙ্গে) অথবা একটি F কী"],
    // Microphone
    ["O Windows bloqueou o microfone. Permita que apps da área de trabalho o usem em Configurações > Privacidade e segurança > Microfone.", "Windows blocked the microphone. Allow desktop apps to use it in Settings > Privacy & security > Microphone.", "Windows bloqueó el micrófono. Permite que las aplicaciones de escritorio lo usen en Configuración > Privacidad y seguridad > Micrófono.", "Windows a bloqué le micro. Autorisez les applications de bureau à l'utiliser dans Paramètres > Confidentialité et sécurité > Microphone.", "Windows hat das Mikrofon blockiert. Erlaube Desktop-Apps den Zugriff unter Einstellungen > Datenschutz und Sicherheit > Mikrofon.", "Windows がマイクをブロックしました。設定 > プライバシーとセキュリティ > マイク で、デスクトップ アプリの使用を許可してください。", "Windows 阻止了麦克风。请在 设置 > 隐私和安全性 > 麦克风 中允许桌面应用使用。", "Windows заблокировала микрофон. Разрешите классическим приложениям его использовать в разделе Параметры > Конфиденциальность и защита > Микрофон.", "Windows ने माइक्रोफ़ोन ब्लॉक कर दिया। सेटिंग्स > निजता और सुरक्षा > माइक्रोफ़ोन में डेस्कटॉप ऐप्स को इसकी अनुमति दें।", "Windows মাইক্রোফোন ব্লক করেছে। সেটিংস > গোপনীয়তা ও নিরাপত্তা > মাইক্রোফোনে ডেস্কটপ অ্যাপকে অনুমতি দিন।"],
    ["Outro app está usando o microfone com exclusividade.", "Another app is using the microphone exclusively.", "Otra aplicación está usando el micrófono en exclusiva.", "Une autre application utilise le micro en mode exclusif.", "Eine andere App nutzt das Mikrofon exklusiv.", "別のアプリがマイクを占有しています。", "另一个应用正在独占麦克风。", "Другое приложение использует микрофон монопольно.", "कोई दूसरा ऐप माइक्रोफ़ोन का विशेष रूप से इस्तेमाल कर रहा है।", "অন্য একটি অ্যাপ মাইক্রোফোন একচেটিয়াভাবে ব্যবহার করছে।"],
    ["Nenhum microfone encontrado.", "No microphone found.", "No se encontró ningún micrófono.", "Aucun micro trouvé.", "Kein Mikrofon gefunden.", "マイクが見つかりません。", "未找到麦克风。", "Микрофон не найден.", "कोई माइक्रोफ़ोन नहीं मिला।", "কোনো মাইক্রোফোন পাওয়া যায়নি।"],
    ["o microfone não respondeu", "the microphone didn't respond", "el micrófono no respondió", "le micro n'a pas répondu", "das Mikrofon hat nicht reagiert", "マイクが応答しませんでした", "麦克风没有响应", "микрофон не ответил", "माइक्रोफ़ोन ने जवाब नहीं दिया", "মাইক্রোফোন সাড়া দেয়নি"],
    ["não foi possível iniciar o microfone: {}", "couldn't start the microphone: {}", "no se pudo iniciar el micrófono: {}", "impossible de démarrer le micro : {}", "das Mikrofon konnte nicht gestartet werden: {}", "マイクを開始できませんでした: {}", "无法启动麦克风:{}", "не удалось запустить микрофон: {}", "माइक्रोफ़ोन शुरू नहीं हो सका: {}", "মাইক্রোফোন চালু করা যায়নি: {}"],
    ["microfone indisponível: {}", "microphone unavailable: {}", "micrófono no disponible: {}", "micro indisponible : {}", "Mikrofon nicht verfügbar: {}", "マイクを使用できません: {}", "麦克风不可用:{}", "микрофон недоступен: {}", "माइक्रोफ़ोन उपलब्ध नहीं: {}", "মাইক্রোফোন অনুপলব্ধ: {}"],
    ["formato de áudio não suportado: {}", "unsupported audio format: {}", "formato de audio no compatible: {}", "format audio non pris en charge : {}", "nicht unterstütztes Audioformat: {}", "対応していない音声形式: {}", "不支持的音频格式:{}", "неподдерживаемый формат аудио: {}", "असमर्थित ऑडियो फ़ॉर्मेट: {}", "অসমর্থিত অডিও ফরম্যাট: {}"],
    ["não foi possível abrir o microfone: {}", "couldn't open the microphone: {}", "no se pudo abrir el micrófono: {}", "impossible d'ouvrir le micro : {}", "das Mikrofon konnte nicht geöffnet werden: {}", "マイクを開けませんでした: {}", "无法打开麦克风:{}", "не удалось открыть микрофон: {}", "माइक्रोफ़ोन खुल नहीं सका: {}", "মাইক্রোফোন খোলা যায়নি: {}"],
    // Providers
    ["chave inválida, expirada ou sem permissão", "invalid, expired or unauthorized key", "clave no válida, caducada o sin permiso", "clé non valide, expirée ou non autorisée", "ungültiger, abgelaufener oder nicht berechtigter Schlüssel", "キーが無効、期限切れ、または権限がありません", "密钥无效、已过期或无权限", "ключ недействителен, просрочен или без доступа", "कुंजी अमान्य, समाप्त या बिना अनुमति की है", "কী অবৈধ, মেয়াদোত্তীর্ণ বা অনুমতিহীন"],
    ["sem créditos ou cota esgotada nesta conta", "no credits, or the quota is used up on this account", "sin créditos o cuota agotada en esta cuenta", "plus de crédits ou quota épuisé sur ce compte", "kein Guthaben oder Kontingent aufgebraucht", "このアカウントはクレジットまたは割り当てを使い切りました", "此账户没有额度或配额已用完", "на этом аккаунте нет кредитов или исчерпана квота", "इस खाते में क्रेडिट नहीं हैं या कोटा खत्म हो गया", "এই অ্যাকাউন্টে ক্রেডিট নেই বা কোটা শেষ"],
    ["limite de uso atingido (tente de novo em instantes ou adicione outra chave)", "usage limit reached (try again in a moment or add another key)", "límite de uso alcanzado (inténtalo en un momento o añade otra clave)", "limite d'utilisation atteinte (réessayez dans un instant ou ajoutez une autre clé)", "Nutzungslimit erreicht (versuche es gleich noch einmal oder füge einen weiteren Schlüssel hinzu)", "利用上限に達しました(少し待って再試行するか、別のキーを追加してください)", "已达使用限额(请稍后重试或添加其他密钥)", "достигнут лимит использования (повторите чуть позже или добавьте другой ключ)", "उपयोग सीमा पूरी हो गई (थोड़ी देर में फिर कोशिश करें या दूसरी कुंजी जोड़ें)", "ব্যবহারের সীমা শেষ (একটু পরে আবার চেষ্টা করুন বা আরেকটি কী যোগ করুন)"],
    ["sem créditos na conta", "no credits on the account", "sin créditos en la cuenta", "plus de crédits sur le compte", "kein Guthaben auf dem Konto", "アカウントにクレジットがありません", "账户没有额度", "на аккаунте нет кредитов", "खाते में क्रेडिट नहीं हैं", "অ্যাকাউন্টে ক্রেডিট নেই"],
    ["modelo não encontrado — confira o nome do modelo nas configurações", "model not found: check the model's name in the settings", "modelo no encontrado: revisa el nombre del modelo en los ajustes", "modèle introuvable : vérifiez son nom dans les paramètres", "Modell nicht gefunden: Prüfe den Modellnamen in den Einstellungen", "モデルが見つかりません。設定でモデル名を確認してください", "未找到模型:请在设置中检查模型名称", "модель не найдена: проверьте её название в настройках", "मॉडल नहीं मिला: सेटिंग्स में मॉडल का नाम जांचें", "মডেল পাওয়া যায়নি: সেটিংসে মডেলের নাম দেখুন"],
    ["chave inválida", "invalid key", "clave no válida", "clé non valide", "ungültiger Schlüssel", "無効なキー", "密钥无效", "недействительный ключ", "अमान्य कुंजी", "অবৈধ কী"],
    ["modelo recusado: {}", "model refused: {}", "modelo rechazado: {}", "modèle refusé : {}", "Modell abgelehnt: {}", "モデルが拒否されました: {}", "模型被拒绝:{}", "модель отклонена: {}", "मॉडल अस्वीकृत: {}", "মডেল প্রত্যাখ্যাত: {}"],
    ["áudio grande demais para este provedor", "the audio is too long for this provider", "el audio es demasiado largo para este proveedor", "l'audio est trop long pour ce fournisseur", "das Audio ist zu lang für diesen Anbieter", "このプロバイダーには音声が長すぎます", "音频对此服务商来说太长", "аудио слишком длинное для этого провайдера", "यह ऑडियो इस प्रदाता के लिए बहुत लंबा है", "এই প্রদানকারীর জন্য অডিও খুব দীর্ঘ"],
    ["o serviço está instável agora", "the service is unstable right now", "el servicio está inestable ahora", "le service est instable pour le moment", "der Dienst ist gerade instabil", "サービスが現在不安定です", "服务目前不稳定", "сервис сейчас работает нестабильно", "सेवा अभी अस्थिर है", "পরিষেবা এখন অস্থিতিশীল"],
    ["modelo descontinuado ou indisponível", "model discontinued or unavailable", "modelo descontinuado o no disponible", "modèle abandonné ou indisponible", "Modell eingestellt oder nicht verfügbar", "モデルは提供終了または利用不可です", "模型已停用或不可用", "модель снята с поддержки или недоступна", "मॉडल बंद या अनुपलब्ध है", "মডেল বন্ধ বা অনুপলব্ধ"],
    ["tempo esgotado", "timed out", "tiempo agotado", "délai dépassé", "Zeitüberschreitung", "タイムアウトしました", "超时", "время ожидания истекло", "समय समाप्त", "সময় শেষ"],
    ["o Ollama não está rodando neste computador", "Ollama isn't running on this computer", "Ollama no se está ejecutando en este equipo", "Ollama ne tourne pas sur cet ordinateur", "Ollama läuft nicht auf diesem Computer", "このコンピューターで Ollama が動いていません", "此电脑上未运行 Ollama", "Ollama не запущена на этом компьютере", "इस कंप्यूटर पर Ollama नहीं चल रहा", "এই কম্পিউটারে Ollama চলছে না"],
    ["sem conexão com a internet", "no internet connection", "sin conexión a internet", "pas de connexion Internet", "keine Internetverbindung", "インターネットに接続されていません", "没有网络连接", "нет подключения к интернету", "इंटरनेट कनेक्शन नहीं है", "ইন্টারনেট সংযোগ নেই"],
    ["falha de rede", "network failure", "fallo de red", "erreur réseau", "Netzwerkfehler", "ネットワークエラー", "网络故障", "сбой сети", "नेटवर्क विफलता", "নেটওয়ার্ক ব্যর্থতা"],
    ["resposta inesperada do serviço", "unexpected answer from the service", "respuesta inesperada del servicio", "réponse inattendue du service", "unerwartete Antwort des Dienstes", "サービスから予期しない応答がありました", "服务返回了意外的响应", "неожиданный ответ сервиса", "सेवा से अप्रत्याशित जवाब", "পরিষেবা থেকে অপ্রত্যাশিত উত্তর"],
    ["nenhum modelo configurado", "no model set up", "ningún modelo configurado", "aucun modèle configuré", "kein Modell eingerichtet", "モデルが未設定です", "未配置模型", "модель не настроена", "कोई मॉडल सेट नहीं है", "কোনো মডেল সেট করা নেই"],
    ["a IA devolveu uma resposta vazia", "the AI returned an empty answer", "la IA devolvió una respuesta vacía", "l'IA a renvoyé une réponse vide", "die KI hat eine leere Antwort geliefert", "AI が空の応答を返しました", "AI 返回了空回复", "ИИ вернул пустой ответ", "AI ने खाली जवाब दिया", "AI খালি উত্তর দিয়েছে"],
    // Clipboard and the selection
    ["área de transferência indisponível", "clipboard unavailable", "portapapeles no disponible", "presse-papiers indisponible", "Zwischenablage nicht verfügbar", "クリップボードを使用できません", "剪贴板不可用", "буфер обмена недоступен", "क्लिपबोर्ड उपलब्ध नहीं", "ক্লিপবোর্ড অনুপলব্ধ"],
    ["não foi possível copiar: {}", "couldn't copy: {}", "no se pudo copiar: {}", "impossible de copier : {}", "Kopieren nicht möglich: {}", "コピーできませんでした: {}", "无法复制:{}", "не удалось скопировать: {}", "कॉपी नहीं हो सका: {}", "কপি করা যায়নি: {}"],
    ["O app que está usando a área de transferência não responde. Tente de novo em instantes.", "The app holding the clipboard isn't responding. Try again in a moment.", "La aplicación que usa el portapapeles no responde. Inténtalo en un momento.", "L'application qui utilise le presse-papiers ne répond pas. Réessayez dans un instant.", "Die App, die die Zwischenablage belegt, reagiert nicht. Versuche es gleich noch einmal.", "クリップボードを使用中のアプリが応答しません。少し待ってからもう一度お試しください。", "正在占用剪贴板的应用没有响应。请稍后重试。", "Приложение, занявшее буфер обмена, не отвечает. Попробуйте чуть позже.", "क्लिपबोर्ड इस्तेमाल कर रहा ऐप जवाब नहीं दे रहा। थोड़ी देर में फिर कोशिश करें।", "ক্লিপবোর্ড ব্যবহারকারী অ্যাপ সাড়া দিচ্ছে না। একটু পরে আবার চেষ্টা করুন।"],
    ["Este sistema não permite ler o texto selecionado (instale o xdotool ou o wtype).", "This system can't read the selected text (install xdotool or wtype).", "Este sistema no permite leer el texto seleccionado (instala xdotool o wtype).", "Ce système ne peut pas lire le texte sélectionné (installez xdotool ou wtype).", "Dieses System kann den ausgewählten Text nicht lesen (installiere xdotool oder wtype).", "このシステムでは選択テキストを読み取れません(xdotool または wtype をインストールしてください)。", "此系统无法读取所选文字(请安装 xdotool 或 wtype)。", "Эта система не может прочитать выделенный текст (установите xdotool или wtype).", "यह सिस्टम चुना गया टेक्स्ट नहीं पढ़ सकता (xdotool या wtype इंस्टॉल करें)।", "এই সিস্টেম নির্বাচিত লেখা পড়তে পারে না (xdotool বা wtype ইনস্টল করুন)।"],
    ["Esse app roda como administrador — o Windows não deixa o FlowVoice ler a seleção dele.", "That app runs as administrator: Windows doesn't let FlowVoice read its selection.", "Esa aplicación se ejecuta como administrador: Windows no deja que FlowVoice lea su selección.", "Cette application tourne en administrateur : Windows empêche FlowVoice de lire sa sélection.", "Diese App läuft als Administrator: Windows lässt FlowVoice ihre Auswahl nicht lesen.", "そのアプリは管理者として実行中のため、Windows が FlowVoice による選択テキストの読み取りを許可しません。", "该应用以管理员身份运行:Windows 不允许 FlowVoice 读取其选中内容。", "Это приложение запущено от имени администратора: Windows не даёт FlowVoice прочитать выделение.", "वह ऐप एडमिनिस्ट्रेटर के रूप में चल रहा है: Windows FlowVoice को उसका चयन पढ़ने नहीं देता।", "সেই অ্যাপটি অ্যাডমিনিস্ট্রেটর হিসেবে চলছে: Windows FlowVoice-কে তার নির্বাচন পড়তে দেয় না।"],
    ["A janela com o texto selecionado perdeu o foco.", "The window with the selected text lost the focus.", "La ventana con el texto seleccionado perdió el foco.", "La fenêtre contenant le texte sélectionné a perdu le focus.", "Das Fenster mit dem ausgewählten Text hat den Fokus verloren.", "選択テキストのあるウィンドウがフォーカスを失いました。", "包含所选文字的窗口失去了焦点。", "Окно с выделенным текстом потеряло фокус.", "चुने गए टेक्स्ट वाली विंडो का फ़ोकस चला गया।", "নির্বাচিত লেখার উইন্ডো ফোকাস হারিয়েছে।"],
    // Local models
    ["modelo desconhecido", "unknown model", "modelo desconocido", "modèle inconnu", "unbekanntes Modell", "不明なモデル", "未知模型", "неизвестная модель", "अज्ञात मॉडल", "অজানা মডেল"],
    ["este processador não tem AVX2, necessário para o modo local — use a transcrição na nuvem", "this processor doesn't have AVX2, which local mode needs: use cloud transcription", "este procesador no tiene AVX2, necesario para el modo local: usa la transcripción en la nube", "ce processeur n'a pas AVX2, nécessaire au mode local : utilisez la transcription dans le cloud", "dieser Prozessor hat kein AVX2, das der lokale Modus braucht: Nutze die Cloud-Transkription", "このプロセッサーにはローカルモードに必要な AVX2 がありません。クラウドの文字起こしを使ってください", "此处理器没有本地模式所需的 AVX2:请使用云端转写", "у этого процессора нет AVX2, нужного для локального режима: используйте облачную расшифровку", "इस प्रोसेसर में लोकल मोड के लिए ज़रूरी AVX2 नहीं है: क्लाउड ट्रांसक्रिप्शन इस्तेमाल करें", "এই প্রসেসরে লোকাল মোডের জন্য দরকারি AVX2 নেই: ক্লাউড ট্রান্সক্রিপশন ব্যবহার করুন"],
    ["modelo local desconhecido: {}", "unknown local model: {}", "modelo local desconocido: {}", "modèle local inconnu : {}", "unbekanntes lokales Modell: {}", "不明なローカルモデル: {}", "未知的本地模型:{}", "неизвестная локальная модель: {}", "अज्ञात लोकल मॉडल: {}", "অজানা লোকাল মডেল: {}"],
    ["o modelo local \"{}\" não foi baixado", "the local model \"{}\" wasn't downloaded", "el modelo local \"{}\" no se descargó", "le modèle local « {} » n'a pas été téléchargé", "das lokale Modell „{}“ wurde nicht heruntergeladen", "ローカルモデル「{}」はダウンロードされていません", "本地模型“{}”尚未下载", "локальная модель «{}» не загружена", "लोकल मॉडल \"{}\" डाउनलोड नहीं हुआ", "লোকাল মডেল \"{}\" ডাউনলোড করা হয়নি"],
    ["falha ao carregar o modelo {}: {}", "couldn't load the model {}: {}", "no se pudo cargar el modelo {}: {}", "impossible de charger le modèle {} : {}", "Modell {} konnte nicht geladen werden: {}", "モデル {} を読み込めませんでした: {}", "无法加载模型 {}:{}", "не удалось загрузить модель {}: {}", "मॉडल {} लोड नहीं हो सका: {}", "মডেল {} লোড করা যায়নি: {}"],
];

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCES: [&str; 10] = [
        include_str!("pipeline.rs"),
        include_str!("commands.rs"),
        include_str!("audio.rs"),
        include_str!("cloud.rs"),
        include_str!("hotkeys.rs"),
        include_str!("platform.rs"),
        include_str!("local_stt.rs"),
        include_str!("tray.rs"),
        include_str!("settings.rs"),
        include_str!("windows.rs"),
    ];

    #[test]
    fn every_row_is_complete_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for row in T {
            assert!(seen.insert(row[0]), "duplicate: {}", row[0]);
            let holes = row[0].matches("{}").count();
            for (i, cell) in row.iter().enumerate() {
                assert!(!cell.trim().is_empty(), "empty {} in {}", LANGS[i], row[0]);
                assert_eq!(cell.matches("{}").count(), holes, "{} placeholders in {}", LANGS[i], row[0]);
            }
        }
    }

    #[test]
    fn every_text_the_code_asks_for_is_in_the_table() {
        let call = regex::Regex::new(r#"\b(?:tr|trf)\(\s*"((?:[^"\\]|\\.)*)""#).unwrap();
        let mut n = 0;
        for src in SOURCES {
            for c in call.captures_iter(src) {
                let text = c[1].replace("\\\"", "\"");
                assert!(T.iter().any(|r| r[0] == text), "not in the table: {text}");
                n += 1;
            }
        }
        assert!(n > 50, "only {n} calls found");
    }

    #[test]
    fn placeholders_are_filled_in_order() {
        assert_eq!(trf("falha ao carregar o modelo {}: {}", &[&"tiny", &"erro"]), "falha ao carregar o modelo tiny: erro");
    }

    #[test]
    fn unknown_text_comes_back_as_is() {
        assert_eq!(tr("texto que não está na tabela"), "texto que não está na tabela");
    }
}
