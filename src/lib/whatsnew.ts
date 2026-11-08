// "What's new": the highlights shown once after an update (in the interface's language), and the
// full CHANGELOG.md (Portuguese) for the history in About. A version without highlights shows its
// CHANGELOG section instead.
import changelog from "../../CHANGELOG.md?raw";
import type { IconName } from "./icons/paths";
import type { UiLanguage } from "./types";

export type Highlight = { icon: IconName; title: string; text: string };

const HIGHLIGHTS: Record<string, Record<UiLanguage, Highlight[]>> = {
  "2.2.1": {
    pt: [{ icon: "history", title: "Esc não apaga mais o ditado", text: "Cancelou sem querer? O que a prévia mostrou vai para o histórico, marcado como cancelado, e o cartão oferece Transcrever a gravação." }],
    en: [{ icon: "history", title: "Esc no longer loses your dictation", text: "Cancelled by accident? What the live preview showed goes to the history, marked as cancelled, and the card offers to transcribe the recording." }],
    es: [{ icon: "history", title: "Esc ya no borra el dictado", text: "¿Cancelaste sin querer? Lo que mostró la vista previa va al historial, marcado como cancelado, y la tarjeta ofrece transcribir la grabación." }],
    fr: [{ icon: "history", title: "Échap ne fait plus perdre la dictée", text: "Annulé par erreur ? Ce que l'aperçu affichait va dans l'historique, marqué comme annulé, et la carte propose de transcrire l'enregistrement." }],
    de: [{ icon: "history", title: "Esc löscht das Diktat nicht mehr", text: "Versehentlich abgebrochen? Was die Live-Vorschau zeigte, landet als abgebrochen markiert im Verlauf, und die Karte bietet an, die Aufnahme zu transkribieren." }],
    ja: [{ icon: "history", title: "Esc で音声入力が消えなくなりました", text: "うっかりキャンセルしても、ライブプレビューの内容が「キャンセル済み」として履歴に残り、カードから録音を文字起こしできます。" }],
    zh: [{ icon: "history", title: "按 Esc 不再丢失听写", text: "误按取消？实时预览显示的内容会以“已取消”标记保存到历史记录，卡片还可以转写录音。" }],
    ru: [{ icon: "history", title: "Esc больше не стирает диктовку", text: "Отменили случайно? То, что показал живой черновик, попадает в историю с пометкой «Отменено», а карточка предлагает расшифровать запись." }],
    hi: [{ icon: "history", title: "Esc से अब डिक्टेशन नहीं खोता", text: "गलती से रद्द कर दिया? लाइव प्रीव्यू में जो दिखा, वह “रद्द” निशान के साथ इतिहास में जाता है, और कार्ड रिकॉर्डिंग ट्रांसक्राइब करने का विकल्प देता है।" }],
    bn: [{ icon: "history", title: "Esc চাপলে আর ডিকটেশন হারায় না", text: "ভুল করে বাতিল করেছেন? লাইভ প্রিভিউতে যা দেখা গেছে তা “বাতিল” চিহ্নসহ ইতিহাসে যায়, আর কার্ড থেকে রেকর্ডিং ট্রান্সক্রাইব করা যায়।" }],
  },
  "2.2.0": {
    pt: [
      { icon: "grip", title: "A cápsula vai aonde você quiser", text: "Arraste a cápsula enquanto grava, até para outro monitor, ou escolha uma posição fixa numa miniatura da tela." },
      { icon: "refresh", title: "Tentar de novo", text: "Se a transcrição falhar, a gravação fica guardada na memória e um clique refaz o ditado." },
      { icon: "edit", title: "Revisar a seleção", text: "Selecione um texto e use o atalho Revisar (Ctrl + Alt + J) ou o chip do Comando: corrige ortografia e pontuação sem mudar as palavras." },
      { icon: "globe", title: "10 idiomas", text: "A interface agora também em francês, alemão, japonês, chinês, russo, hindi e bengali, e segue o idioma do Windows." },
      { icon: "keyboard", title: "Atalhos sem conflito", text: "Colar de novo vem sem atalho (Ctrl + Alt + V é do Office) e o app respeita o AltGr do seu teclado." },
      { icon: "lock", title: "Privacidade", text: "Uma seção nova em Sobre mostra o que fica no PC e o que vai para cada provedor." },
      { icon: "check", title: "Correções", text: "Iniciar com o Windows mais confiável, e o som do PC volta se o app fechar no meio de uma gravação." },
    ],
    en: [
      { icon: "grip", title: "The capsule goes where you want", text: "Drag the capsule while recording, even to another monitor, or pick a fixed spot on a thumbnail of the screen." },
      { icon: "refresh", title: "Try again", text: "If the transcription fails, the recording is kept in memory and one click redoes the dictation." },
      { icon: "edit", title: "Proofread selection", text: "Select text and use the Proofread shortcut (Ctrl + Alt + J) or the Command chip: fixes spelling and punctuation without changing the words." },
      { icon: "globe", title: "10 languages", text: "The interface now also comes in French, German, Japanese, Chinese, Russian, Hindi and Bengali, and follows the Windows language." },
      { icon: "keyboard", title: "Shortcuts without conflicts", text: "Paste again has no shortcut by default (Ctrl + Alt + V belongs to Office), and the app respects your keyboard's AltGr." },
      { icon: "lock", title: "Privacy", text: "A new section in About shows what stays on the PC and what goes to each provider." },
      { icon: "check", title: "Fixes", text: "Starting with Windows is more reliable, and the PC sound comes back if the app closes in the middle of a recording." },
    ],
    es: [
      { icon: "grip", title: "La cápsula va adonde quieras", text: "Arrastra la cápsula mientras grabas, incluso a otro monitor, o elige una posición fija en una miniatura de la pantalla." },
      { icon: "refresh", title: "Intentar de nuevo", text: "Si la transcripción falla, la grabación queda guardada en memoria y un clic rehace el dictado." },
      { icon: "edit", title: "Revisar la selección", text: "Selecciona un texto y usa el atajo Revisar (Ctrl + Alt + J) o el chip del Comando: corrige ortografía y puntuación sin cambiar las palabras." },
      { icon: "globe", title: "10 idiomas", text: "La interfaz ahora también está en francés, alemán, japonés, chino, ruso, hindi y bengalí, y sigue el idioma de Windows." },
      { icon: "keyboard", title: "Atajos sin conflictos", text: "Pegar de nuevo viene sin atajo (Ctrl + Alt + V es de Office) y la app respeta el AltGr de tu teclado." },
      { icon: "lock", title: "Privacidad", text: "Una sección nueva en Acerca de muestra qué queda en el PC y qué va a cada proveedor." },
      { icon: "check", title: "Correcciones", text: "Iniciar con Windows es más fiable, y el sonido del PC vuelve si la app se cierra en medio de una grabación." },
    ],
    fr: [
      { icon: "grip", title: "La capsule va où vous voulez", text: "Faites glisser la capsule pendant l'enregistrement, même vers un autre écran, ou choisissez une position fixe sur une miniature de l'écran." },
      { icon: "refresh", title: "Réessayer", text: "Si la transcription échoue, l'enregistrement reste en mémoire et un clic refait la dictée." },
      { icon: "edit", title: "Corriger la sélection", text: "Sélectionnez un texte et utilisez le raccourci Corriger (Ctrl + Alt + J) ou la puce de Commande : corrige l'orthographe et la ponctuation sans changer les mots." },
      { icon: "globe", title: "10 langues", text: "L'interface existe désormais aussi en français, allemand, japonais, chinois, russe, hindi et bengali, et suit la langue de Windows." },
      { icon: "keyboard", title: "Raccourcis sans conflit", text: "Coller à nouveau n'a plus de raccourci par défaut (Ctrl + Alt + V appartient à Office) et l'app respecte la touche AltGr de votre clavier." },
      { icon: "lock", title: "Confidentialité", text: "Une nouvelle section dans À propos montre ce qui reste sur le PC et ce qui part vers chaque fournisseur." },
      { icon: "check", title: "Corrections", text: "Le démarrage avec Windows est plus fiable, et le son du PC revient si l'app se ferme en plein enregistrement." },
    ],
    de: [
      { icon: "grip", title: "Die Kapsel geht, wohin du willst", text: "Zieh die Kapsel während der Aufnahme, sogar auf einen anderen Monitor, oder wähle eine feste Position auf einer Miniatur des Bildschirms." },
      { icon: "refresh", title: "Erneut versuchen", text: "Schlägt die Transkription fehl, bleibt die Aufnahme im Speicher und ein Klick wiederholt das Diktat." },
      { icon: "edit", title: "Auswahl korrigieren", text: "Wähle Text aus und nutze das Kürzel Korrigieren (Strg + Alt + J) oder den Chip im Befehl: Rechtschreibung und Zeichensetzung werden korrigiert, die Wörter bleiben." },
      { icon: "globe", title: "10 Sprachen", text: "Die Oberfläche gibt es jetzt auch auf Französisch, Deutsch, Japanisch, Chinesisch, Russisch, Hindi und Bengalisch, und sie folgt der Windows-Sprache." },
      { icon: "keyboard", title: "Kürzel ohne Konflikte", text: "Erneut einfügen hat standardmäßig kein Kürzel (Strg + Alt + V gehört zu Office), und die App respektiert das AltGr deiner Tastatur." },
      { icon: "lock", title: "Datenschutz", text: "Ein neuer Abschnitt in Über zeigt, was auf dem PC bleibt und was an welchen Anbieter geht." },
      { icon: "check", title: "Korrekturen", text: "Der Start mit Windows ist zuverlässiger, und der PC-Ton kommt zurück, wenn die App mitten in einer Aufnahme schließt." },
    ],
    ja: [
      { icon: "grip", title: "カプセルを好きな場所へ", text: "録音中にカプセルをドラッグでき、別のモニターにも移せます。画面のミニチュアで固定位置も選べます。" },
      { icon: "refresh", title: "もう一度試す", text: "文字起こしに失敗しても録音はメモリに残り、ワンクリックでやり直せます。" },
      { icon: "edit", title: "選択範囲を校正", text: "テキストを選択して校正ショートカット(Ctrl + Alt + J)かコマンドのチップを使うと、言葉はそのままで、つづりと句読点を直します。" },
      { icon: "globe", title: "10 言語", text: "インターフェースがフランス語、ドイツ語、日本語、中国語、ロシア語、ヒンディー語、ベンガル語にも対応し、Windows の言語に合わせます。" },
      { icon: "keyboard", title: "ぶつからないショートカット", text: "「もう一度貼り付け」は既定でショートカットなし(Ctrl + Alt + V は Office 用)になり、キーボードの AltGr も尊重します。" },
      { icon: "lock", title: "プライバシー", text: "「情報」の新しいセクションで、PC に残るものと各プロバイダーに送られるものを確認できます。" },
      { icon: "check", title: "修正", text: "Windows と同時に起動がより確実になり、録音中にアプリが閉じても PC の音が戻ります。" },
    ],
    zh: [
      { icon: "grip", title: "胶囊想放哪儿就放哪儿", text: "录音时可以拖动胶囊，甚至拖到另一台显示器，也可以在屏幕缩略图上选择固定位置。" },
      { icon: "refresh", title: "重试", text: "转写失败时，录音会保留在内存中，点一下即可重新听写。" },
      { icon: "edit", title: "校对选中内容", text: "选中文字后使用校对快捷键(Ctrl + Alt + J)或命令中的标签：只修正拼写和标点，不改动用词。" },
      { icon: "globe", title: "10 种语言", text: "界面新增法语、德语、日语、中文、俄语、印地语和孟加拉语，并跟随 Windows 的语言。" },
      { icon: "keyboard", title: "快捷键不再冲突", text: "「再次粘贴」默认不设快捷键(Ctrl + Alt + V 属于 Office)，应用也会尊重键盘上的 AltGr。" },
      { icon: "lock", title: "隐私", text: "「关于」中的新部分说明哪些内容留在电脑上，哪些会发送给各个服务商。" },
      { icon: "check", title: "修复", text: "随 Windows 启动更可靠；录音途中应用关闭时，电脑声音也会恢复。" },
    ],
    ru: [
      { icon: "grip", title: "Капсула там, где вам удобно", text: "Перетаскивайте капсулу во время записи, даже на другой монитор, или выберите постоянное место на миниатюре экрана." },
      { icon: "refresh", title: "Повторить", text: "Если расшифровка не удалась, запись остаётся в памяти, и один клик повторяет диктовку." },
      { icon: "edit", title: "Проверить выделенное", text: "Выделите текст и нажмите сочетание «Проверить» (Ctrl + Alt + J) или чип в команде: исправит орфографию и пунктуацию, не меняя слов." },
      { icon: "globe", title: "10 языков", text: "Интерфейс теперь есть и на французском, немецком, японском, китайском, русском, хинди и бенгальском и следует языку Windows." },
      { icon: "keyboard", title: "Сочетания без конфликтов", text: "У «Вставить снова» по умолчанию нет сочетания (Ctrl + Alt + V занято Office), а приложение учитывает AltGr вашей клавиатуры." },
      { icon: "lock", title: "Конфиденциальность", text: "Новый раздел в «О программе» показывает, что остаётся на ПК и что уходит каждому провайдеру." },
      { icon: "check", title: "Исправления", text: "Запуск вместе с Windows стал надёжнее, а звук ПК возвращается, если приложение закрылось посреди записи." },
    ],
    hi: [
      { icon: "grip", title: "कैप्सूल जहां चाहें वहां", text: "रिकॉर्डिंग के दौरान कैप्सूल को खींचें, दूसरे मॉनिटर तक भी, या स्क्रीन के छोटे चित्र पर एक तय जगह चुनें।" },
      { icon: "refresh", title: "फिर से कोशिश करें", text: "ट्रांसक्रिप्शन विफल हो तो रिकॉर्डिंग मेमोरी में रहती है और एक क्लिक से डिक्टेशन फिर से हो जाता है।" },
      { icon: "edit", title: "चुना हुआ टेक्स्ट प्रूफ़रीड करें", text: "टेक्स्ट चुनें और प्रूफ़रीड शॉर्टकट (Ctrl + Alt + J) या कमांड की चिप इस्तेमाल करें: शब्द बदले बिना वर्तनी और विराम चिह्न ठीक होते हैं।" },
      { icon: "globe", title: "10 भाषाएं", text: "इंटरफ़ेस अब फ़्रेंच, जर्मन, जापानी, चीनी, रूसी, हिंदी और बांग्ला में भी है, और Windows की भाषा के साथ चलता है।" },
      { icon: "keyboard", title: "बिना टकराव वाले शॉर्टकट", text: "फिर से पेस्ट का अब कोई डिफ़ॉल्ट शॉर्टकट नहीं है (Ctrl + Alt + V Office का है), और ऐप आपके कीबोर्ड के AltGr का ध्यान रखता है।" },
      { icon: "lock", title: "निजता", text: "जानकारी में नया हिस्सा बताता है कि PC पर क्या रहता है और हर प्रदाता के पास क्या जाता है।" },
      { icon: "check", title: "सुधार", text: "Windows के साथ शुरू होना ज़्यादा भरोसेमंद है, और रिकॉर्डिंग के बीच ऐप बंद होने पर भी PC की आवाज़ लौट आती है।" },
    ],
    bn: [
      { icon: "grip", title: "ক্যাপসুল যেখানে খুশি", text: "রেকর্ড করার সময় ক্যাপসুল টেনে সরান, অন্য মনিটরেও, অথবা স্ক্রিনের ছোট ছবিতে একটি স্থির জায়গা বেছে নিন।" },
      { icon: "refresh", title: "আবার চেষ্টা করুন", text: "ট্রান্সক্রিপশন ব্যর্থ হলে রেকর্ডিং মেমোরিতে থাকে, আর এক ক্লিকে ডিকটেশন আবার হয়।" },
      { icon: "edit", title: "নির্বাচিত লেখা প্রুফরিড করুন", text: "লেখা নির্বাচন করে প্রুফরিড শর্টকাট (Ctrl + Alt + J) বা কমান্ডের চিপ ব্যবহার করুন: শব্দ না বদলে বানান আর যতিচিহ্ন ঠিক হয়।" },
      { icon: "globe", title: "১০টি ভাষা", text: "ইন্টারফেস এখন ফরাসি, জার্মান, জাপানি, চীনা, রুশ, হিন্দি আর বাংলাতেও আছে, আর Windows-এর ভাষা মেনে চলে।" },
      { icon: "keyboard", title: "সংঘাতহীন শর্টকাট", text: "আবার পেস্টের এখন কোনো ডিফল্ট শর্টকাট নেই (Ctrl + Alt + V Office-এর), আর অ্যাপ আপনার কিবোর্ডের AltGr মেনে চলে।" },
      { icon: "lock", title: "গোপনীয়তা", text: "সম্পর্কে নতুন অংশটি দেখায় কী PC-তে থাকে আর কী প্রতিটি প্রদানকারীর কাছে যায়।" },
      { icon: "check", title: "সংশোধন", text: "Windows-এর সাথে চালু হওয়া আরও নির্ভরযোগ্য, আর রেকর্ডিংয়ের মাঝে অ্যাপ বন্ধ হলেও PC-র শব্দ ফিরে আসে।" },
    ],
  },
};

/** The highlights of `version` in `lang`, or null when that version has none. */
export function highlights(version: string, lang: UiLanguage): Highlight[] | null {
  return HIGHLIGHTS[version]?.[lang] ?? null;
}

/** What's new for someone who last saw `since` and now runs `version`: the highlights of every
 *  version in between that has some, newest first (2.1 -> 2.2.1 shows 2.2.1's, then 2.2.0's), or null. */
export function highlightsSince(version: string, since: string, lang: UiLanguage): Highlight[] | null {
  const all = Object.keys(HIGHLIGHTS)
    .filter((v) => compareVersions(v, since || "0") > 0 && compareVersions(v, version) <= 0)
    .sort(compareVersions)
    .reverse()
    .flatMap((v) => highlights(v, lang) ?? []);
  return all.length ? all : null;
}

/** The whole history, from the first version heading on (the file's title and intro left out). */
export function changelogHistory(): string {
  const i = changelog.search(/^## /m);
  return i < 0 ? changelog : changelog.slice(i);
}

/** One version's section of CHANGELOG.md, without its heading ("" if it isn't there). */
export function changelogSection(version: string): string {
  const all = changelogHistory();
  // "## 2.2.1", also with a date after it ("## 2.2.1 - 2026-11-10", "## 2.2.1 (2026-11-10)").
  const re = new RegExp(`^## ${version.replace(/\./g, "\\.")}(?:[ \\t]+[-–(].*)?\\s*$`, "m");
  const m = re.exec(all);
  if (!m) return "";
  const rest = all.slice(m.index + m[0].length);
  const next = rest.search(/^## /m);
  return (next < 0 ? rest : rest.slice(0, next)).trim();
}

/** The CHANGELOG sections of every version after `since` up to `version`, newest first, each
 *  under its own heading ("" when there's none). */
export function changelogSince(version: string, since: string): string {
  const versions = [...changelogHistory().matchAll(/^## (\d+(?:\.\d+)+)/gm)].map((m) => m[1]);
  return versions
    .filter((v) => compareVersions(v, since || "0") > 0 && compareVersions(v, version) <= 0)
    .map((v) => [v, changelogSection(v)] as const)
    .filter(([, s]) => s)
    .map(([v, s]) => `## ${v}\n\n${s}`)
    .join("\n\n");
}

/** -1, 0 or 1, comparing "2.10.0" and "2.9.1" by numbers; a prerelease ("2.3.0-beta.1") comes
 *  before its release. */
export function compareVersions(a: string, b: string): number {
  const [ca, pre_a = ""] = a.split("-", 2);
  const [cb, pre_b = ""] = b.split("-", 2);
  const pa = ca.split(".").map((x) => parseInt(x, 10) || 0);
  const pb = cb.split(".").map((x) => parseInt(x, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d) return d < 0 ? -1 : 1;
  }
  if (pre_a === pre_b) return 0;
  if (!pre_a || !pre_b) return pre_a ? -1 : 1;
  return pre_a.localeCompare(pre_b, "en", { numeric: true }) < 0 ? -1 : 1;
}
