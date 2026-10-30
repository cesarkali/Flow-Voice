# FlowVoice

**Ditado por voz com IA em qualquer app.** Aperte um atalho, fale, e o texto aparece onde está o cursor: limpo, pontuado e no estilo que você escolher. Funciona com chaves **gratuitas** (Groq, Gemini, OpenRouter, Mistral) ou **100% offline**.

Feito por **Júlio Caliberda** · [github.com/cesarkali](https://github.com/cesarkali) · [caliberda.com.br](https://caliberda.com.br)

**Versão atual:** 2.0.0 · Windows e Linux (Ubuntu/Debian)

---

## O que ele faz

- **Ditado em qualquer lugar**: `Ctrl + Shift + Espaço` começa, o mesmo atalho para. O texto é colado na janela onde você estava.
- **Prévia ao vivo**: enquanto você fala, o texto aparece em tempo real no overlay (Whisper local leve).
- **Estilos**
  - **Natural**: corrige e limpa a fala sem mudar seu jeito de falar.
  - **Profissional**: texto formal e organizado, pronto para e-mail.
  - **Prompt**: enxuga a fala para mandar a uma IA. Corta "né", "tipo", rodeios e repetições e mantém todo o contexto, então gasta menos tokens.
  - **Cru**: só as palavras, sem pontuação e sem quebra de linha.
  - **Só transcrição**: exatamente o que foi transcrito, sem passar por IA.
  - Você pode editar os prompts e criar estilos próprios.
- **Mudou de ideia? Troque no próprio overlay**: depois do resultado, um clique refaz em outro estilo ou **traduz** (inglês, espanhol e outros). **Substituir** desfaz a colagem anterior e cola a nova.
- **Tradução por voz**: `Ctrl + Shift + Y` dita já traduzindo.
- **Assistente por voz**: `Ctrl + Shift + U` faz uma pergunta falada e abre um chat. Com a chave Tavily, busca na web e cita as fontes.
- **Failover**: cadastre vários provedores e várias chaves. Se um falhar ou estourar o limite, o FlowVoice tenta o próximo. Se a nuvem cair, usa o modelo local. Os erros aparecem com a mensagem real do provedor.
- **Remoção de vícios de linguagem** antes da IA ("né", "tipo", "hã", gagueira), que economiza tokens.
- **Vocabulário próprio**: nomes e termos técnicos que você quer sempre escritos certo.
- **Histórico** pesquisável, **push-to-talk** (segurar para falar), silenciar o PC enquanto grava, iniciar com o sistema, temas claro e escuro, interface em PT/EN/ES e atualização automática.
- **Leve**: roda na bandeja, sem janela aberta. O instalador tem poucos MB e os modelos locais só são baixados se você quiser.

## Provedores e chaves (todas grátis, menos a OpenAI)

| Provedor | Para quê | Onde pegar a chave | Plano grátis |
|---|---|---|---|
| **Groq** (recomendado) | voz + texto | [console.groq.com/keys](https://console.groq.com/keys) → *Create API Key* | ~8 h de áudio/dia, `gpt-oss-20b` |
| **Google Gemini** | voz + texto | [aistudio.google.com/apikey](https://aistudio.google.com/apikey) → *Create API key* | sim (o Google pode usar os dados para melhorar produtos) |
| **OpenRouter** | texto | [openrouter.ai/settings/keys](https://openrouter.ai/settings/keys) → *Create Key* | modelos `:free`, 50 pedidos/dia |
| **Mistral** | voz + texto | [console.mistral.ai](https://console.mistral.ai) → *API Keys* | sim, sem cartão |
| **OpenAI** | voz + texto | [platform.openai.com/api-keys](https://platform.openai.com/api-keys) | pago |
| **Ollama** | texto, local | [ollama.com/download](https://ollama.com/download) → `ollama pull gemma3:4b` | grátis e offline |
| **Tavily** | busca na web | [app.tavily.com](https://app.tavily.com) | 1.000 buscas/mês |

Cada provedor tem um tutorial passo a passo e um botão **Testar** dentro do app. Os modelos podem ser trocados nas configurações se algum for descontinuado.

> O **GitHub Models** (recomendado na 1.x) foi descontinuado pelo GitHub em 30/07/2026. Os modelos `llama` do Groq saíram do plano grátis, e os do Gemini 1.5/2.0 foram desligados. Por isso a 1.x parou de funcionar.

As chaves ficam no **cofre de credenciais do sistema** (Gerenciador de Credenciais do Windows / Secret Service no Linux), nunca em texto puro. Na primeira execução, a 2.0 importa as chaves da 1.x automaticamente.

## Modo 100% offline

Em **Configurações → Transcrição**, escolha *100% local* e baixe um modelo do whisper.cpp:

| Modelo | Tamanho | Uso |
|---|---|---|
| Tiny | 31 MB | PCs bem fracos |
| **Base** | 57 MB | rápido, usado também na prévia ao vivo |
| Small | 181 MB | equilíbrio |
| Large v3 Turbo | 547 MB | máxima precisão |

Para refinar o texto sem internet, use o **Ollama**.

## Instalação

- **Windows**: baixe `FlowVoice_x.y.z_x64-setup.exe` em [Releases](https://github.com/cesarkali/Flow-Voice/releases). Não precisa de administrador.
- **Ubuntu/Debian**: baixe o `.deb` em Releases e rode `sudo apt install ./FlowVoice_*.deb`. Para colar automaticamente, o pacote instala o `xdotool` (X11). No Wayland, instale o `wtype`, ou o texto fica só copiado.

## Desenvolvimento

Stack: **Tauri 2** (Rust) + **Svelte 5** + Vite. O código da versão 1.x (Python) está em `legacy/`.

Pré-requisitos: Node 22+, Rust estável e, no Windows, o Visual Studio Build Tools (C++ e CMake) mais a `libclang` (LLVM). Se o LLVM não estiver instalado, aponte `LIBCLANG_PATH` para uma pasta com `libclang.dll`.

```bash
npm install
npm run tauri dev      # app completo
npm run dev            # só a interface no navegador, com dados simulados
npm run tauri build    # instalador
```

Estrutura:

```
src/            interface (settings, overlay, chat) em Svelte
src-tauri/src/  núcleo em Rust
  audio.rs      captura do microfone → 16 kHz mono
  local_stt.rs  whisper.cpp + download de modelos
  cloud.rs      transcrição/IA na nuvem, erros legíveis
  pipeline.rs   gravar → transcrever → refinar → colar, assistente
  text.rs       vícios de linguagem, estilo cru, filtro de alucinações
  settings.rs   configurações e prompts dos estilos
```

### Publicar uma versão

1. Atualize a versão em `package.json`, `src-tauri/Cargo.toml` e `src-tauri/tauri.conf.json`.
2. Cadastre uma vez os secrets `TAURI_SIGNING_PRIVATE_KEY` (conteúdo da chave privada do atualizador) e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` no repositório.
3. Crie e envie a tag: `git tag v2.0.0 && git push origin v2.0.0`. O GitHub Actions gera os instaladores, o `latest.json` do atualizador e um release em rascunho.

## Licença

GPL-3.0 © Júlio Caliberda
