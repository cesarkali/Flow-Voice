# Changelog

Todas as mudanças relevantes do FlowVoice. As notas da 1.x estão nos [releases antigos](https://github.com/cesarkali/Flow-Voice/releases?q=v1&expanded=true) e no histórico do Git.

## 2.0.0

Reescrita completa: de Python/PySide6 para **Tauri 2 (Rust) + Svelte 5**.

### Por que a 1.x parou de funcionar

- O GitHub Models foi descontinuado em 30/07/2026.
- Os modelos `llama` do Groq saíram do plano grátis (agosto/2026).
- Os modelos Gemini 1.5/2.0 foram desligados, e a biblioteca `google-generativeai` foi descontinuada.
- A 1.x escondia o erro real, então parecia que simplesmente "não funcionava".

### Novidades

- Overlay novo: prévia ao vivo do texto enquanto você fala e, depois do resultado, chips para trocar o estilo, traduzir, **substituir** o que já foi colado, colar ou copiar.
- Novo estilo **Prompt**: enxuga a fala para mandar a uma IA, com menos tokens e o mesmo contexto.
- Estilos **Natural**, **Profissional**, **Cru** e **Só transcrição**, com prompts editáveis e estilos personalizados.
- **Estilos por aplicativo**: escolha o estilo usado em cada app (Outlook, Slack, VS Code, Gmail no navegador…).
- **Modo Comando** (`Ctrl + Alt + K`): selecione um texto e diga o que fazer com ele.
- Tradução para 34 idiomas, por atalho (`Ctrl + Shift + Y`) ou no próprio overlay.
- **Colar de novo** a última transcrição (`Ctrl + Alt + V`).
- Provedores atualizados: Groq (`whisper-large-v3`, `gpt-oss-20b`), Gemini 3.x, OpenRouter (modelos grátis), Mistral, OpenAI, Ollama e qualquer endpoint compatível com OpenAI.
- Failover entre provedores e chaves, com o erro real do provedor exibido.
- Modelos que se corrigem sozinhos: se um provedor aposenta um modelo, o FlowVoice escolhe e testa o substituto.
- Modo 100% offline com whisper.cpp (modelos baixados sob demanda, incluindo um ajustado para português do Brasil).
- Remoção de vícios de linguagem antes da IA, vocabulário próprio, histórico pesquisável e push-to-talk.
- Chaves no cofre de credenciais do sistema, importadas automaticamente da 1.x.
- Interface nova (PT/EN/ES, tema claro e escuro) e assistente de configuração com tutorial e teste de cada chave.
- Linha de comando (`flowvoice --toggle`, `--translate`, `--ask`, `--command`, `--paste-last`, `--cancel`) para Stream Deck, AutoHotkey e scripts.
- Instalador de ~3 MB e atualização automática assinada pelo GitHub Releases.
