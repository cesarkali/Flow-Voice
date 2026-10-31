# FlowVoice: product context (website)

## Product

FlowVoice 2.0 is free, open-source (GPL-3.0) voice dictation for Windows 10/11 (and Linux, .deb for Ubuntu/Debian), made by one person in Brazil: Júlio Caliberda (GitHub `cesarkali`). Built with Tauri 2 (Rust) and Svelte 5.

Mechanism, in one sentence: press a hotkey in any app, speak, and the text appears where the cursor is, already cleaned up by AI in the style that app needs.

## Facts the site may state (verified against README and app code)

- Default hotkey `Ctrl + Shift + Espaço`; the same hotkey stops. Push-to-talk optional.
- Live preview: text appears in the overlay while speaking; uncertain words lighter.
- Styles: Natural, Profissional, Prompt, Cru, Só transcrição, plus user-created styles. After the result, one click redoes it in another style or translates.
- Translation while dictating (`Ctrl + Shift + Y`) to 34 languages.
- Command mode (`Ctrl + Alt + K`): select text anywhere, say an instruction ("deixa mais formal"), the result replaces the selection.
- Per-app styles: the app in focus picks the style (e.g. Outlook → Profissional, Slack → Natural, VS Code → Prompt). Recognizes desktop apps and web apps in browser tabs (Gmail, WhatsApp Web, ChatGPT, Claude, Notion, Teams…).
- Voice assistant (`Ctrl + Shift + U`) with web search via Tavily.
- Providers: Groq (recommended, free, no card), Google Gemini (free tier; Google may use data), Mistral (free), OpenRouter (`:free` models), OpenAI (paid), Ollama (local). Several keys rotate; failover to next provider and to the local model.
- 100% offline: local Whisper models (Tiny 31 MB, Base 57 MB, Small 181 MB, Distil-Whisper pt-BR 513 MB, Large v3 Turbo 547 MB), downloaded only if wanted.
- Keys stored in the Windows Credential Manager (Secret Service on Linux). No account, no FlowVoice server.
- Installer ~3 MB, no admin needed, auto-updates. Unsigned installer: SmartScreen warning is expected.
- Interface in PT, EN, ES. Imports 1.x keys automatically.

Never invent: users counts, ratings, benchmarks, testimonials, Mac support (there is none).

## Audience and scene

Brazilian knowledge workers and developers who write all day on Windows: e-mail, Slack/WhatsApp/Teams, AI chats, code editors. They arrive from GitHub, a WhatsApp share or a search, on a laptop at a desk in daylight, sometimes on a phone. They want to know in seconds: what it does, that it is really free, that it works in their apps, and where to download.

## Site

Static site on Vercel (`flowvoice.caliberda.com.br`), pt-BR, no framework, no analytics. Pages: index (Persuade), guia (Read: free keys guide), privacidade (Read), novidades (Read: changelog).

Primary action: "Baixar para Windows" → `https://github.com/cesarkali/Flow-Voice/releases/latest/download/FlowVoiceSetup.exe`. Secondary: GitHub repo.

## Brand commitments

- Logo: dark rounded tile, light voice bars turning into a red I-beam text cursor. Accent `#FF2E00` (the app's own record/caret red).
- Owner's verdict on the previous site: "ugly, AI copycat, wrong app icons, a Wikipedia". Wanted: basic, elegant, beautiful, professional; little text; real app icons.
