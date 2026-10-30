#!/usr/bin/env python3
"""
Cria o arquivo de release notes para a versão em version.py.

Uso:
    py new_release.py
    py new_release.py --open
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parent
RELEASES_DIR = ROOT / "releases"
TEMPLATE = RELEASES_DIR / "TEMPLATE.md"


def get_version() -> str:
    namespace: dict = {}
    exec((ROOT / "version.py").read_text(encoding="utf-8"), namespace)
    return namespace["VERSION"]


def get_previous_version(current: str) -> str | None:
    if not RELEASES_DIR.is_dir():
        return None

    versions = []
    for path in RELEASES_DIR.glob("*.md"):
        if path.name.upper() == "TEMPLATE.MD" or path.name.upper() == "README.MD":
            continue
        match = re.fullmatch(r"(\d+\.\d+\.\d+)\.md", path.name)
        if match:
            versions.append(match.group(1))

    def parse(value: str) -> tuple[int, ...]:
        return tuple(int(part) for part in value.split("."))

    older = [value for value in versions if parse(value) < parse(current)]
    if not older:
        return None
    return sorted(older, key=parse)[-1]


def render_template(version: str) -> str:
    content = TEMPLATE.read_text(encoding="utf-8")
    content = content.replace("X.Y.Z", version)
    return content


def main() -> None:
    parser = argparse.ArgumentParser(description="Gera release notes para a versão atual.")
    parser.add_argument(
        "--open",
        action="store_true",
        help="Abre o arquivo gerado no editor padrão do Windows.",
    )
    args = parser.parse_args()

    version = get_version()
    RELEASES_DIR.mkdir(exist_ok=True)
    output = RELEASES_DIR / f"{version}.md"

    if output.exists():
        raise SystemExit(f"Release notes já existem: {output}")

    previous = get_previous_version(version)
    body = render_template(version)
    if previous:
        lines = body.split("\n", 1)
        note = f"> Comparar com a versão anterior: [`releases/{previous}.md`]({previous}.md)\n\n"
        body = lines[0] + "\n\n" + note + (lines[1] if len(lines) > 1 else "")

    output.write_text(body, encoding="utf-8")
    print(f"Release notes criadas: {output}")
    if previous:
        print(f"Versão anterior detectada: v{previous}")
    print("Edite o arquivo com as mudanças desta versão antes de publicar a release.")

    if args.open and output.exists():
        import os

        os.startfile(output)


if __name__ == "__main__":
    main()
