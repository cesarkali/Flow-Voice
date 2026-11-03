# Changelog

Todas as mudanças relevantes do FlowVoice. As notas da 1.x estão nos [releases antigos](https://github.com/cesarkali/Flow-Voice/releases?q=v1&expanded=true) e no histórico do Git.

## 2.2.0

### Mova a cápsula

- **Arrastar durante a gravação**: clique na cápsula e arraste enquanto grava, para qualquer canto da tela e até para outro monitor. Ela fica onde você largou até o fim daquele ditado; no próximo, volta para a posição padrão. Dá para desligar em **Geral → Janela flutuante**.
- **Posição personalizada**: além de Embaixo e Em cima, escolha o lugar arrastando a cápsula numa miniatura da tela (com encaixe nos cantos, nas bordas e no centro). **Testar** mostra onde ela aparece, e **Usar onde arrastei por último** aproveita o ponto em que você a largou.

### Tentar de novo e ditados seguidos

- **Tentar de novo** quando a transcrição falha (rede, limite, chave): a gravação fica guardada só na memória (as 3 últimas falhas, nunca em disco). O cartão de erro ganha o botão e fica 30 s na tela; o histórico mostra a falha com o motivo e o mesmo botão (lá o texto só é copiado). Refaz como foi ditado: mesmo app, estilo e tradução, com os provedores e chaves de agora. Não vale para o Comando.
- Dois ditados seguidos no mesmo campo ganham um espaço entre eles ("…na quinta. A reunião…"), quando o cursor ficou onde o anterior terminou e faz menos de um minuto. Não em terminais, nem em escritas sem espaço (japonês, chinês, tailandês).

### Correções

- **Iniciar com o Windows** mais confiável: em alguns PCs o Windows pulava a entrada de inicialização do FlowVoice mesmo marcada como habilitada no Gerenciador de Tarefas. Agora o app também cria uma tarefa de logon no Agendador de Tarefas (por usuário, sem pedir administrador).
- Desligar a inicialização no Gerenciador de Tarefas agora vale de verdade: o app não se liga de novo sozinho ao abrir.
- Comando ou "Outra versão" com tradução não devolve mais duas versões (o original e a tradução, a tradução repetida, um "---" com comentário ou uma "versão melhorada" com rótulo); uma nota no fim, como "(Note: …)", também sai.
- Texto ditado em árabe, hebraico, persa e urdu aparece na direção certa no overlay, no histórico e no chat.
- As novidades de uma atualização aparecem formatadas no app (seções, lista, negrito e código), sem o aviso de download do GitHub.
- A tela Sobre mostra a versão instalada no título (era fixo "2.0").

## 2.1.0

### Prévia ao vivo mais estável

- O texto não volta mais atrás: cada modelo tem um pedido por vez, e uma resposta atrasada (de um trecho mais antigo) é descartada.
- Palavras confirmadas: uma palavra só fica firme quando duas respostas seguidas concordam nela; o resto aparece mais claro e ainda pode mudar.
- A prévia não manda mais o texto anterior como contexto (às vezes ele voltava repetido).
- O ritmo segue os limites que o próprio Groq informa e sempre guarda espaço para a transcrição final, que não leva mais "limite atingido" logo depois da prévia.
- Frases longas são fechadas em até 15 s: respostas mais rápidas e menos segundos gastos do plano grátis.
- A faixa extra do Gemini na prévia saiu (escrevia diferente do Whisper e era mais lenta, o texto pulava).
- Medido com os mesmos 4 áudios: texto já mostrado como firme e depois trocado caiu de 2,5–7,5 vezes por ditado para zero; o total de texto reescrito caiu ~4×; atualizações a cada ~1,2–1,5 s (antes ~1,7–2,0 s); nenhum "limite atingido".

### Vidro na cápsula

- Nova opção em **Geral → Janela flutuante**: **Vidro** (transparente e desfocada, com borda de luz) ou **Translúcido** (tinta mais leve), usando o desfoque do próprio Windows, feito pela placa de vídeo.
- **Luz e sombra como no macOS 27**: brilho, borda chanfrada, luz na parte de baixo e sombra suave (ideia de Thiago Silva).
- **Usar também nesta janela**: a janela de configurações ganha o mesmo vidro (o acrílico do Windows 11 atrás dela), com os cartões continuando sólidos para ler bem.
- A aparência da cápsula também pode ser escolhida no primeiro passo do tour, com uma prévia ao vivo.
- Precisa dos efeitos de transparência do Windows; com eles desligados ou com a economia de energia, a cápsula fica sólida.

### Correções

- Num PC ocupado, ao iniciar com o Windows, a janela às vezes mostrava `state not managed for field state…` no lugar das configurações.
- Depois de atualizar, o FlowVoice reabre sozinho com a janela aberta (se tinha iniciado com o Windows, voltava só na bandeja e parecia não ter reaberto).

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
