# Release notes do FlowVoice

Cada versão publicada deve ter um arquivo `X.Y.Z.md` nesta pasta.

## Como preparar uma nova versão

1. Atualize a versão em [`version.py`](../version.py):
   ```python
   VERSION = "1.7.1"
   ```

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
| `1.7.1.md` | Release notes da v1.7.1 |

## Versão atual

Consulte [`version.py`](../version.py) — atualmente **1.7.1**.
