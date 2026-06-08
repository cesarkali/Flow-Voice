# FlowVoice - Ditado Inteligente por IA no seu Cursor

O **FlowVoice** é um utilitário de produtividade leve e elegante para **Windows** e **Ubuntu** que roda em segundo plano na bandeja do sistema. Ele permite que você dite textos por voz em qualquer campo de digitação do sistema (navegador, editores de código, chat do Teams, Word, etc.). O áudio é capturado, transcrito por IA e, opcionalmente, polido e corrigido gramaticalmente de forma automática antes de ser colado diretamente onde está o seu cursor.
Desenvolvido por: **Júlio Caliberda** ([caliberda.com.br](https://caliberda.com.br)) | Repositório: [GitHub](https://github.com/cesarkali/Flow-Voice)

**Versão atual:** 1.4.0

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
- **Pesquisa Google por Voz**: Pressione `Ctrl + Shift + U` para fazer perguntas faladas. O app busca na IA e abre um **Assistente Chat** interativo para você continuar a conversa.
- **Múltiplos Provedores (Failover Pool)**: Configure chaves para **Gemini**, **OpenAI**, **Groq** ou **GitHub Models**. Se um provedor falhar, o app automaticamente tenta o próximo na fila.
- **Transcrição Local (100% Offline)**: Opção de rodar sem chaves de nuvem usando o modelo **Whisper** localmente via GPU/CPU.
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
sudo apt install ./ubuntu/dist/flowvoice_1.4.0_amd64.deb
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

### Ubuntu — gerar `flowvoice_1.4.0_amd64.deb`

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
- Pacote: `ubuntu/dist/flowvoice_1.4.0_amd64.deb`

**Instalar o pacote gerado:**
```bash
sudo apt install ./ubuntu/dist/flowvoice_1.4.0_amd64.deb
```

Documentação adicional do build Ubuntu: [`ubuntu/README.md`](ubuntu/README.md).

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

Release notes da versão atual: [`releases/1.4.0.md`](releases/1.4.0.md)
