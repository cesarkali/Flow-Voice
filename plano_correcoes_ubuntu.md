# Plano de Correções e Validação - Ubuntu / Linux

Aqui está o resumo dos problemas relatados e o status das alterações para o funcionamento perfeito no Ubuntu (sem afetar a build do Windows).

## 1. Erros/Avisos na instalação do pacote `.deb`
**O Problema:** O arquivo `.desktop` gerado durante a build possui categorias que quebram a norma de Desktop Entries do Linux. O aviso do `_apt` no final do log (`Permission denied`) é apenas um alerta de segurança nativo padrão do Ubuntu quando o APT lê um arquivo isolado da sua pasta de usuário e não impede a instalação.
**A Solução:**
- No arquivo `ubuntu/debian/flowvoice.desktop` (que gerencia o ícone do sistema), encontre a linha `Categories=...` e certifique-se de emparelhar a categoria `Audio` corretamente.
- Exemplo de alteração: Mudar `Categories=Utility;Audio;Office;` para `Categories=Utility;AudioVideo;Audio;Office;`.
**Status:** ✅ Solucionado (Aplicado diff no arquivo `flowvoice.desktop`).

## 2. Atalhos de Teclado e Colar Automático (Captura de Voz) não funcionam
**O Problema:** O Ubuntu 22.04+ utiliza o servidor gráfico **Wayland** por padrão. A biblioteca `pynput` suporta apenas o servidor **X11 (Xorg)**. O Wayland tem regras rígidas de segurança que bloqueiam nativamente que aplicativos em segundo plano leiam atalhos de teclado (keyloggers) ou injetem teclas (como o Ctrl+V para colar o texto gerado).
**A Solução:**
- **Sem alterar código (Solução rápida):** Fazer logout, e na tela de login do Ubuntu, clicar na "engrenagem" (canto inferior direito) e selecionar **"Ubuntu on Xorg"**. O programa e os atalhos voltarão a funcionar instantaneamente.
- **Alterando o código para suportar Wayland:**
  - **Para atalhos:** Você precisará trocar/complementar o `pynput` por atalhos lidos diretamente via `evdev` (que lê eventos diretos do kernel, já instalado na sua base, mas requer que o usuário esteja no grupo `input`) ou usar a API do D-Bus.
  - **Para colar (TextPaster):** O `pynput` falhará em emular o Ctrl+V. A solução no Wayland é contar com o `QClipboard` nativamente para a área de transferência e o usuário usa Ctrl+V manual, ou depender de pacotes extras do linux como o `wtype` (via subprocess) para simular o teclado.
**Status:** ⚠️ Depende da escolha da arquitetura. Como alterar a biblioteca de inputs tem alto risco de quebrar a versão Windows, a recomendação atual é instruir o uso do **Xorg**.

## 3. Não puxa atualizações e falha no download
**O Problema:** A função `get_latest_release` em `main.py` está tentando baixar o pacote `.deb` usando a versão instalada **atual** (ex: v1.7.3) como nome de busca do arquivo, e não a versão do **novo** pacote encontrado no GitHub (ex: v1.7.4). Isso resulta em uma URL inexistente e o download nunca acontece.
**Solução:** Alterado o `main.py` repassando o parâmetro `target_version` (`latest_ver`) ao montar a URL.
**Status:** ✅ Solucionado (Código alterado no `main.py`).

## 4. Não instala a atualização sozinho no Ubuntu
**O Problema:** Ao finalizar o download, para o Windows, você chama o `FlowVoiceUpdater.exe`. Para o Linux, você executa o `xdg-open` no arquivo `.deb`. O `xdg-open` no Ubuntu costuma apenas abrir o pacote visualmente na Loja de Aplicativos (App Center) em vez de instalá-lo de forma automática e silenciosa.
**Solução:** Substituído por chamada ao comando Linux `pkexec` que faz a elevação gráfica de privilégios e auto-instala via `apt install`.
**Status:** ✅ Solucionado (Código alterado no `main.py`).
