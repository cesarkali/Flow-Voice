# FlowVoice para Ubuntu

Esta pasta contém os arquivos necessários para gerar o instalador `.deb` do FlowVoice no Ubuntu.

O Windows continua usando o instalador em `installer.iss` (`dist/FlowVoiceSetup.exe`). Esta pasta é exclusiva para a distribuição Ubuntu/Debian.

## Pré-requisitos no Ubuntu

```bash
sudo apt update
sudo apt install -y python3 python3-venv python3-pip dpkg-dev \
    libportaudio2 libasound2t64 libxcb-xinerama0 libxcb-cursor0 \
    libegl1 libgl1 libxkbcommon0 libpulse0
```

> **Ubuntu 22.04 / Debian:** se `libasound2t64` não existir, use `libasound2` no lugar.

> **VirtualBox (pasta compartilhada `/media/sf_*`):** o script detecta automaticamente e compila em `/tmp`. Se precisar forçar: `FLOWVOICE_BUILD_LOCAL=1 ./ubuntu/build-deb.sh`

## Gerar o pacote .deb

Na raiz do projeto, execute:

```bash
chmod +x ubuntu/build-deb.sh
./ubuntu/build-deb.sh
```

O arquivo final será criado em:

```text
ubuntu/dist/flowvoice_1.7.3_amd64.deb
```

## Instalar no Ubuntu

```bash
sudo apt install ./ubuntu/dist/flowvoice_1.7.3_amd64.deb
```

Depois da instalação:

- O aplicativo ficará em `/opt/flowvoice/flowvoice`
- Atalho no menu: **FlowVoice** (busque por "ditado" ou "voz")
- Configurações e logs: `~/.config/FlowVoice/`

O instalador executa automaticamente o refresh do menu (`update-desktop-database` e cache de ícones).

Se não aparecer imediatamente, rode manualmente:

```bash
sudo update-desktop-database /usr/share/applications
sudo gtk-update-icon-cache -f /usr/share/icons/hicolor
```

No GNOME, se ainda não listar, pressione `Alt+F2`, digite `r` e Enter (reinicia o shell) ou faça logout/login.

## Executar em desenvolvimento (sem .deb)

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements-linux.txt
python3 main.py
```

## Observações

- A opção de mutar áudio do PC durante a gravação funciona apenas no Windows (`pycaw`).
- Atalhos globais no Linux podem exigir permissões de sessão gráfica (Wayland/X11).
- Para publicar atualizações automáticas no Ubuntu, anexe o `.deb` gerado na release do GitHub com o nome `flowvoice_1.7.3_amd64.deb` (versão em `version.py`).