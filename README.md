# FlowVoice - Ditado Inteligente por IA no seu Cursor

O **FlowVoice** é um utilitário de produtividade leve e elegante para Windows que roda em segundo plano na bandeja do sistema. Ele permite que você dite textos por voz em qualquer campo de digitação do sistema (navegador, editores de código, chat do Teams, Word, etc.). O áudio é capturado, transcrito por IA e, opcionalmente, polido e corrigido gramaticalmente de forma automática antes de ser colado diretamente onde está o seu cursor.

Desenvolvido por: **Júlio Caliberda** ([caliberda.com.br](https://caliberda.com.br)) | Repositório: [GitHub](https://github.com/cesarkali/Flow-Voice)

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
- **Integração com Windows**: Roda silenciosamente na área de notificação (bandeja), com opção de **iniciar junto com o Windows** integrada no menu.
- **Sistema de Logs**: Todos os eventos e depurações do Whisper local e da IA são salvos de forma organizada em `%APPDATA%\FlowVoice\flowvoice.log`.

---

## 💻 Instalação

### Instalação via Instalador (Recomendado para Usuários)
Basta baixar e executar o arquivo de instalação rápida:
👉 **`dist/FlowVoiceSetup.exe`**

O instalador irá:
1. Instalar o aplicativo na pasta Arquivos de Programas (`C:\Program Files\FlowVoice`).
2. Criar atalhos no Menu Iniciar e na Área de Trabalho.
3. Salvar suas chaves de API e arquivos de preferências de forma persistente e segura na pasta local de usuário (`%APPDATA%\FlowVoice`), de modo que suas configurações não sejam apagadas em atualizações do programa.

---

## 🛠️ Desenvolvimento e Execução (via Python)

### 1. Pré-requisitos
Certifique-se de ter o Python 3.10 ou superior instalado no seu sistema.

### 2. Instalar Dependências
Instale as dependências executando:
```bash
py -m pip install -r requirements.txt
```

### 3. Executar o Aplicativo
Inicie o aplicativo pelo interpretador do Python:
```bash
py main.py
```
*Nota: Na primeira execução, o aplicativo gerará automaticamente o arquivo `config.json` na raiz ou no `%APPDATA%` (se compilado). Caso prefira configurar as chaves de API e atalhos antes de abrir o aplicativo, basta copiar o arquivo `config.example.json` como `config.json` e inserir suas chaves.*

---

## ⚙️ Empacotamento e Compilação

Para recompilar o executável e o instalador de alta compressão:

### 1. Instalar as Ferramentas de Compilação
```bash
py -m pip install pyinstaller pillow
```

### 2. Compilar o Diretório de Distribuição
```bash
py -m PyInstaller --noconsole --onedir --icon=icon.png --add-data "icon.png;." --hidden-import="pynput.keyboard._win32" main.py
```
Isso gerará a pasta do programa descompactada e otimizada para inicialização rápida em `dist/main/`.

### 3. Gerar o Instalador Setup
Abra o programa **Inno Setup** e compile o arquivo [`installer.iss`](file:///c:/Dev/ST/installer.iss), ou execute via linha de comando:
```powershell
& "C:\Users\<SeuUsuario>\AppData\Local\Programs\Inno Setup 6\ISCC.exe" installer.iss
```
O arquivo final do instalador otimizado de ~108MB será gerado em `dist/FlowVoiceSetup.exe`.
