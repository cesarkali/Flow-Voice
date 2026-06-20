# FlowVoice - Ditado Inteligente por IA no seu Cursor

O **FlowVoice** é um utilitário de produtividade leve e elegante para **Windows** e **Ubuntu** que roda em segundo plano na bandeja do sistema. Ele permite que você dite textos por voz em qualquer campo de digitação do sistema (navegador, editores de código, chat do Teams, Word, etc.). O áudio é capturado, transcrito por IA e, opcionalmente, polido e corrigido gramaticalmente de forma automática antes de ser colado diretamente onde está o seu cursor.
Desenvolvido por: **Júlio Caliberda** ([caliberda.com.br](https://caliberda.com.br)) | Repositório: [GitHub](https://github.com/cesarkali/Flow-Voice)

**Versão atual:** 1.9.1

### ⚡ Consumo de Recursos (Leveza)
O **FlowVoice** foi projetado para ser executado sem impactar o desempenho do seu computador:
- **Memória RAM**: Consome apenas **~30 MB** enquanto permanece ativo em segundo plano na bandeja do sistema.
- **Processador CPU**: Consome **&lt; 1%** de CPU quando ocioso. Durante a gravação e o processamento de IA, o consumo permanece mínimo devido ao uso de threads de segundo plano.

---

## 🚀 Recursos Principais

- **Atalho Global Automático**: Pressione `Ctrl + Shift + Space` para começar a ditar e aperte novamente para transcrever e colar instantaneamente.
- **Modos de Polimento por IA**:
  - **Profissional**: Remove hesitações ("hã", "tipo", "né"), corrige gramática e pontuação, e reescreve a fala em uma linguagem corporativa de negócios.
  - **Casual**: Corrige a gramática essencial, mantendo a voz natural, coloquial e as gírias do usuário.
  - **Direto/Cru**: Apenas transcreve literalmente cada palavra falada, sem reformulações.
- **Tradução por Voz**: Pressione `Ctrl + Shift + Y` para ditar e traduzir sua fala automaticamente para Inglês, Espanhol, Francês, Alemão ou Italiano.
- **Pesquisa Web por Voz**: Pressione `Ctrl + Shift + U` para fazer perguntas faladas. O app busca via **Tavily** (quando configurado) ou na IA e abre um **Assistente Chat** interativo para você continuar a conversa.
- **Múltiplos Provedores (Failover Pool)**: Configure chaves para **Gemini**, **OpenAI**, **Groq** ou **GitHub Models**. Se um provedor falhar, o app automaticamente tenta o próximo na fila.
- **Transcrição Local (100% Offline)**: Opção de rodar sem chaves de nuvem usando o modelo **Whisper** localmente via GPU/CPU.
- **Internacionalização (i18n)**: Interface completa disponível em **Português**, **Inglês** e **Espanhol**, com troca de idioma ao vivo sem reiniciar o app.
- **Assistente de Configuração (Wizard)**: Fluxo guiado de 4 etapas para configurar o app pela primeira vez: idioma → provedor de IA → estilo de transcrição → atalho global.
- **Proteção por Senha**: Chaves de API podem ser protegidas por senha, com opção de redefinição via e-mail.
- **Painel de Configurações Interativo**: Painel de controle em abas modernas para gerenciar o app de forma simples:
  - **🏠 Início**: Painel com status atual (provedor, estilo, modo, atalho).
  - **⚙️ Geral**: Escolha o tom da transcrição, o idioma padrão da tradução por voz, inicialização automática com o sistema e silenciamento de sons do PC ao gravar.
  - **🔑 Conexões**: Escolha a IA principal, insira chaves de API (com suporte a múltiplas chaves separadas por vírgula), configure a chave Tavily para pesquisa web e gerencie senha de proteção das chaves.
  - **🖥️ Whisper Local**: Selecione o tamanho do modelo offline (desde `tiny` super rápido a `large-v3` de alta precisão) e ative aceleração por placa Nvidia (`CUDA`).
  - **⌨️ Atalhos**: Personalize todos os atalhos globais capturando as combinações de teclas diretamente do seu teclado físico.
- **Verificação Automática de Atualizações**: O sistema busca por novas versões em segundo plano a cada 1 hora sem interromper o uso, mostrando de forma visual e dinâmica no rodapé do painel de configurações o status atual (atualizado ou nova versão disponível).
- **Integração com o sistema**: Roda silenciosamente na área de notificação (bandeja), com opção de **iniciar junto com o sistema** integrada no menu.
- **Sistema de Logs**: Todos os eventos e depurações do Whisper local e da IA são salvos de forma organizada em `%APPDATA%\FlowVoice\flowvoice.log` (Windows) ou `~/.config/FlowVoice/flowvoice.log` (Ubuntu).

---

## 💻 Instalação

### Windows (Recomendado para Usuários)
Basta baixar e executar o arquivo de instalação rápida:
👉 **`dist/FlowVoiceSetup.exe`**

O instalador irá:
1. Instalar o aplicativo na pasta Arquivos de Programas (`C:\Program Files\FlowVoice`).
2. Criar atalhos no Menu Iniciar e na Área de Trabalho.
3. Salvar suas chaves de API e arquivos de preferências de forma persistente e segura na pasta local de usuário (`%APPDATA%\FlowVoice`), de modo que suas configurações não sejam apagadas em atualizações do programa.

### Ubuntu (Recomendado para Usuários)
Baixe ou gere o pacote `.deb` e instale com:

```bash
sudo apt install ./ubuntu/dist/flowvoice_1.9.1_amd64.deb
```

Depois da instalação:
- Aplicativo em `/opt/flowvoice/flowvoice`
- Atalho **FlowVoice** no menu de aplicativos
- Configurações e logs em `~/.config/FlowVoice/`

---

## 🛠️ Desenvolvimento e Execução (via Python)

### 1. Pré-requisitos
Certifique-se de ter o **Python 3.10 ou superior** instalado no seu sistema.

### 2. Instalar Dependências

**Windows:**
```bash
py -m pip install -r requirements.txt
```

**Ubuntu / Linux:**
```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements-linux.txt
```

### 3. Executar o Aplicativo

**Windows:**
```bash
py main.py
```

**Ubuntu / Linux:**
```bash
python3 main.py
```

*Nota: Na primeira execução, o aplicativo gerará automaticamente o arquivo `config.json` na raiz do projeto (desenvolvimento) ou na pasta de dados do usuário (se compilado). Caso prefira configurar as chaves de API e atalhos antes de abrir o aplicativo, basta copiar o arquivo `config.example.json` como `config.json` e inserir suas chaves.*

---

## ⚙️ Empacotamento e Compilação

### Windows — gerar `FlowVoiceSetup.exe`

**Pré-requisito:** [Inno Setup 6](https://jrsoftware.org/isinfo.php) instalado.

Na raiz do projeto, execute um único comando:

```bash
py build-windows.py
```

O script faz automaticamente:
1. Instala `pyinstaller` e `pillow` (se necessário)
2. Compila o executável com PyInstaller em `dist/main/`
3. Gera o instalador com Inno Setup em `dist/FlowVoiceSetup.exe`

**Saída:**
- Instalador: `dist/FlowVoiceSetup.exe`
- Executável: `dist/main/main.exe`

### Ubuntu — gerar `flowvoice_1.9.1_amd64.deb`

**Pré-requisitos no Ubuntu:**
```bash
sudo apt update
sudo apt install -y python3 python3-venv python3-pip dpkg-dev \
    libportaudio2 libasound2t64 libxcb-xinerama0 libxcb-cursor0 \
    libegl1 libgl1 libxkbcommon0 libpulse0
```

Na raiz do projeto, execute:

```bash
chmod +x ubuntu/build-deb.sh
./ubuntu/build-deb.sh
```

O script faz automaticamente:
1. Cria um ambiente virtual e instala as dependências de `requirements-linux.txt`
2. Compila o executável com PyInstaller
3. Empacota o instalador `.deb`

**Saída:**
- Pacote: `ubuntu/dist/flowvoice_1.9.1_amd64.deb`

**Instalar o pacote gerado:**
```bash
sudo apt install ./ubuntu/dist/flowvoice_1.9.1_amd64.deb
```

Documentação adicional do build Ubuntu: [`ubuntu/README.md`](ubuntu/README.md).

---

## 🗂️ Estrutura do Projeto

```
FlowVoice/
├── main.py                  # Ponto de entrada; toda a UI (PySide6), lógica de hotkeys, gravação e colagem
├── ai_processor.py          # Motor de IA: transcrição Whisper, polimento de texto, tradução e pesquisa web
├── config.py                # ConfigManager — leitura/escrita persistente de config.json com suporte a AppData
├── recorder.py              # Captura de áudio via sounddevice + escrita de WAV
├── hotkey.py                # Registro/remoção de hotkeys globais (pynput)
├── paster.py                # Cola texto via clipboard + simulação de Ctrl+V
├── i18n.py                  # Internacionalização: carrega locales/*.json, expõe tr() e set_language()
├── updater.py               # Download e instalação silenciosa de atualizações (Windows)
├── version.py               # Fonte única da versão: VERSION = "X.Y.Z"
├── new_release.py           # Gera esqueleto de release notes em releases/
├── build-windows.py         # Script de build: PyInstaller + Inno Setup → FlowVoiceSetup.exe
├── installer.iss            # Script Inno Setup para o instalador Windows
├── main.spec                # Spec do PyInstaller (Windows)
├── config.example.json      # Modelo de configuração para novos usuários
│
├── locales/                 # Strings de UI traduzidas
│   ├── pt.json              # Português (padrão)
│   ├── en.json              # Inglês
│   └── es.json              # Espanhol
│
├── icons/                   # Ícones SVG usados na UI
│   ├── gemini.svg
│   ├── github.svg
│   ├── globe.svg
│   ├── groq.svg
│   ├── instagram.svg
│   ├── key.svg
│   ├── openai.svg
│   ├── flag_pt.svg
│   ├── flag_en.svg
│   └── flag_es.svg
│
├── releases/                # Release notes por versão (Markdown)
│   └── TEMPLATE.md
│
├── ubuntu/                  # Build e empacotamento para Ubuntu/Linux
│   ├── build-deb.sh         # Script de build do pacote .deb
│   └── README.md            # Documentação do build Ubuntu
│
├── website/                 # Site estático do projeto (Vercel)
│   ├── index.html
│   ├── icon.png
│   └── vercel.json
│
├── dist/                    # Artefatos de build (gerados)
│   └── main.exe
│
├── requirements.txt         # Dependências Python — Windows
├── requirements-linux.txt   # Dependências Python — Ubuntu/Linux
├── icon.ico                 # Ícone do app (Windows)
├── icon.png                 # Ícone do app (PNG)
└── checkmark.svg            # Asset SVG auxiliar
```

### Módulos principais — resumo de responsabilidades

| Arquivo | Responsabilidade |
|---|---|
| `main.py` | UI completa (PySide6): overlay flutuante, bandeja, painel de configurações com abas, wizard de setup, diálogo de chat, diálogo de atualização, hotkeys e cola de texto |
| `ai_processor.py` | Transcrição (Whisper local / API), polimento de texto, tradução, pesquisa web com Tavily + fallback para IA |
| `config.py` | Leitura/escrita de `config.json`; resolve caminho correto entre desenvolvimento e executável compilado |
| `recorder.py` | Gravação de áudio com `sounddevice`; salva arquivo WAV temporário |
| `hotkey.py` | Registra e remove combinações de teclas globais via `pynput` |
| `paster.py` | Copia texto para clipboard e simula `Ctrl+V` para colar onde o cursor está |
| `i18n.py` | Carrega `locales/<lang>.json`; `tr(key)` retorna string traduzida; `set_language()` troca idioma ao vivo |
| `updater.py` | Verifica GitHub Releases, baixa instalador em background e executa atualização silenciosa |

---

## 📋 Versões e Release Notes

A versão oficial fica em [`version.py`](version.py). Ao publicar uma nova versão:

1. Atualize `VERSION` em `version.py`
2. Gere o esqueleto das notas de release:
   ```bash
   py new_release.py
   ```
3. Edite `releases/X.Y.Z.md` com as mudanças em relação à versão anterior
4. Gere os instaladores (`py build-windows.py` e/ou `./ubuntu/build-deb.sh`)
5. Publique no GitHub Release usando o conteúdo de `releases/X.Y.Z.md`

### 🔍 Onde atualizar o número da versão ao lançar um release:
Para atualizar a versão do aplicativo, você deve alterar o número nos seguintes arquivos:
- [version.py](version.py): `VERSION = "X.Y.Z"`
- [installer.iss](installer.iss): `#define MyAppVersion "X.Y.Z"`
- [README.md](README.md): Atualizar a tag `Versão atual`, comandos de instalação `.deb` do Ubuntu e links de release notes.
- [requirements.txt](requirements.txt): Linha 2 `# Versão atual: X.Y.Z`
- [requirements-linux.txt](requirements-linux.txt): Linha 2 `# Versão atual: X.Y.Z`
- [website/index.html](website/index.html): Span com a tag `vX.Y.Z` no nav + seção "O que há de novo" (cards de novidades da versão) + `softwareVersion` no JSON-LD

Release notes da versão atual: [releases/1.9.1.md](releases/1.9.1.md)
