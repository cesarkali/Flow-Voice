# Release notes do FlowVoice

Cada versão publicada deve ter um arquivo `X.Y.Z.md` nesta pasta.

## Como preparar uma nova versão

1. Atualize a versão nos seguintes arquivos:
   - `version.py`: `VERSION = "1.7.3"`
   - `installer.iss`: `#define MyAppVersion "1.7.3"`
   - `requirements.txt`: `# Versão atual: 1.7.3`
   - `requirements-linux.txt`: `# Versão atual: 1.7.3`
   - Todos os arquivos `README.md` (principal e do Ubuntu)

2. Gere o esqueleto das release notes:
   ```bash
   py new_release.py
   ```

3. Edite `releases/X.Y.Z.md` descrevendo:
   - **Novidades**
   - **Melhorias**
   - **Correções**
   - **Comparativo** com a versão anterior (quando fizer sentido)

4. Gere os instaladores:
   ```bash
   py build-windows.py
   ```
   ```bash
   ./ubuntu/build-deb.sh
   ```

5. Publique no GitHub Release usando o conteúdo de `releases/X.Y.Z.md`.

## Arquivos

| Arquivo | Descrição |
|---------|-----------|
| `TEMPLATE.md` | Modelo base para novas versões |
| `1.6.0.md` | Release notes da v1.6.0 |
| `1.7.0.md` | Release notes da v1.7.0 |
| `1.7.2.md` | Release notes da v1.7.2 |
| `1.7.3.md` | Release notes da v1.7.3 |

## Versão atual

Consulte `version.py` — atualmente **1.7.3**.
