#!/usr/bin/env python3
"""
Gera o instalador Windows do FlowVoice (FlowVoiceSetup.exe).

Uso (na raiz do projeto):
    py build-windows.py
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent
DIST_DIR = ROOT / "dist"
PYINSTALLER_DIST = DIST_DIR / "main"
INSTALLER_OUTPUT = DIST_DIR / "FlowVoiceSetup.exe"


def get_version() -> str:
    version_file = ROOT / "version.py"
    namespace: dict = {}
    exec(version_file.read_text(encoding="utf-8"), namespace)
    return namespace["VERSION"]

ISCC_CANDIDATES = (
    Path(os.environ.get("LOCALAPPDATA", "")) / "Programs" / "Inno Setup 6" / "ISCC.exe",
    Path(r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe"),
    Path(r"C:\Program Files\Inno Setup 6\ISCC.exe"),
)


def log(message: str) -> None:
    print(f"==> {message}")


def run_step(label: str, command: list[str], *, cwd: Path | None = None) -> None:
    log(label)
    print(f"    {' '.join(command)}")
    result = subprocess.run(command, cwd=cwd or ROOT)
    if result.returncode != 0:
        raise SystemExit(f"Falha em: {label} (código {result.returncode})")


def find_iscc() -> Path:
    for candidate in ISCC_CANDIDATES:
        if candidate.is_file():
            return candidate

    raise SystemExit(
        "Inno Setup não encontrado.\n"
        "Instale em https://jrsoftware.org/isinfo.php e tente novamente.\n"
        "Caminhos verificados:\n"
        + "\n".join(f"  - {path}" for path in ISCC_CANDIDATES)
    )


def ensure_build_tools() -> None:
    run_step(
        "Instalando ferramentas de compilação (PyInstaller, Pillow)...",
        [sys.executable, "-m", "pip", "install", "pyinstaller", "pillow"],
    )


def build_executable() -> None:
    if not (ROOT / "main.py").is_file():
        raise SystemExit("Arquivo main.py não encontrado. Execute o script na raiz do projeto.")

    pyinstaller_args = [
        sys.executable,
        "-m",
        "PyInstaller",
        "--noconsole",
        "--onedir",
        "--noconfirm",
        "--clean",
        "--name",
        "main",
        "--hidden-import=pynput.keyboard._win32",
    ]

    icon_png = ROOT / "icon.png"
    if icon_png.is_file():
        pyinstaller_args.extend(["--icon", str(icon_png), "--add-data", f"{icon_png};."])
    else:
        log("Aviso: icon.png não encontrado; compilando sem ícone personalizado.")

    pyinstaller_args.append(str(ROOT / "main.py"))

    run_step("Compilando executável com PyInstaller...", pyinstaller_args)

    exe_path = PYINSTALLER_DIST / "main.exe"
    if not exe_path.is_file():
        raise SystemExit(f"PyInstaller não gerou o executável esperado: {exe_path}")


def build_updater() -> None:
    if not (ROOT / "updater.py").is_file():
        raise SystemExit("Arquivo updater.py não encontrado.")

    pyinstaller_args = [
        sys.executable,
        "-m",
        "PyInstaller",
        "--noconsole",
        "--onefile",
        "--noconfirm",
        "--clean",
        "--name",
        "FlowVoiceUpdater",
        "--distpath",
        str(DIST_DIR),
        "--workpath",
        str(ROOT / "build" / "updater-work"),
        "--specpath",
        str(ROOT / "build" / "updater-spec"),
        str(ROOT / "updater.py"),
    ]

    icon_png = ROOT / "icon.png"
    if icon_png.is_file():
        pyinstaller_args[-1:-1] = ["--icon", str(icon_png)]

    run_step("Compilando FlowVoiceUpdater.exe...", pyinstaller_args)

    updater_exe = DIST_DIR / "FlowVoiceUpdater.exe"
    if not updater_exe.is_file():
        raise SystemExit(f"PyInstaller não gerou o atualizador esperado: {updater_exe}")


def build_installer() -> None:
    iss_path = ROOT / "installer.iss"
    if not iss_path.is_file():
        raise SystemExit(f"Arquivo não encontrado: {iss_path}")

    icon_ico = ROOT / "icon.ico"
    if not icon_ico.is_file():
        log("Aviso: icon.ico não encontrado; o instalador pode falhar se o Inno Setup exigir esse arquivo.")

    iscc = find_iscc()
    version = get_version()
    run_step(
        "Gerando instalador com Inno Setup...",
        [str(iscc), f"/DMyAppVersion={version}", str(iss_path)],
    )

    if not INSTALLER_OUTPUT.is_file():
        raise SystemExit(f"Instalador não encontrado após compilação: {INSTALLER_OUTPUT}")


def main() -> None:
    if sys.platform != "win32":
        raise SystemExit("Este script é apenas para Windows. Use ubuntu/build-deb.sh no Ubuntu.")

    log("FlowVoice: build Windows")
    print(f"    Projeto: {ROOT}")
    print(f"    Versão:  {get_version()}")
    print()

    ensure_build_tools()
    build_executable()
    build_updater()
    build_installer()

    size_mb = INSTALLER_OUTPUT.stat().st_size / (1024 * 1024)
    print()
    log("Build concluído com sucesso!")
    print(f"    Instalador: {INSTALLER_OUTPUT}")
    print(f"    Tamanho:    {size_mb:.1f} MB")
    print(f"    Executável: {PYINSTALLER_DIST / 'main.exe'}")
    print(f"    Atualizador: {DIST_DIR / 'FlowVoiceUpdater.exe'}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        raise SystemExit("\nBuild cancelado pelo usuário.") from None
