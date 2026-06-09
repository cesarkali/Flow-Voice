#!/usr/bin/env python3
"""
FlowVoice Updater — executável separado para instalar atualizações no Windows.

Uso:
    FlowVoiceUpdater.exe <caminho_instalador> <versão> <caminho_flowvoice.exe>
"""

from __future__ import annotations

import os
import subprocess
import sys
import threading
import time
import tkinter as tk
from tkinter import ttk

LOG_PATH = os.path.join(os.environ.get("TEMP", "."), "flowvoice_update.log")

INSTALLER_ARGS = (
    "/VERYSILENT",
    "/SUPPRESSMSGBOXES",
    "/CLOSEAPPLICATIONS",
    "/FORCECLOSEAPPLICATIONS",
)


def log(message: str) -> None:
    line = f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] {message}\n"
    try:
        with open(LOG_PATH, "a", encoding="utf-8") as handle:
            handle.write(line)
    except OSError:
        pass


def _no_window_flags() -> int:
    if sys.platform == "win32":
        return subprocess.CREATE_NO_WINDOW
    return 0


def kill_flowvoice() -> None:
    subprocess.run(
        ["taskkill", "/F", "/IM", "FlowVoice.exe", "/T"],
        capture_output=True,
        creationflags=_no_window_flags(),
    )


def run_installer_elevated(installer_path: str) -> int:
    """Runs the Inno Setup installer elevated and waits until it finishes."""
    arg_list = ",".join(f'"{arg}"' for arg in INSTALLER_ARGS)
    ps_command = (
        f'$p = Start-Process -FilePath "{installer_path}" '
        f"-ArgumentList {arg_list} "
        f"-Verb RunAs -Wait -PassThru; "
        f"if ($null -eq $p) {{ exit 1 }} else {{ exit $p.ExitCode }}"
    )
    result = subprocess.run(
        [
            "powershell.exe",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            ps_command,
        ],
        capture_output=True,
        text=True,
        creationflags=_no_window_flags(),
    )
    if result.stdout.strip():
        log(f"PowerShell stdout: {result.stdout.strip()}")
    if result.stderr.strip():
        log(f"PowerShell stderr: {result.stderr.strip()}")
    return int(result.returncode)


def relaunch_app(app_exe: str) -> None:
    subprocess.Popen(
        [app_exe],
        close_fds=True,
        creationflags=subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP,
    )


class UpdaterWindow:
    def __init__(self, installer_path: str, version: str, app_exe: str):
        self.installer_path = installer_path
        self.version = version
        self.app_exe = app_exe

        self.root = tk.Tk()
        self.root.title("FlowVoice - Atualizando")
        self.root.geometry("480x170")
        self.root.resizable(False, False)
        self.root.attributes("-topmost", True)

        frame = ttk.Frame(self.root, padding=20)
        frame.pack(fill=tk.BOTH, expand=True)

        self.title_label = ttk.Label(
            frame,
            text=f"Instalando FlowVoice v{version}",
            font=("Segoe UI", 11, "bold"),
        )
        self.title_label.pack(fill=tk.X)

        self.message_label = ttk.Label(
            frame,
            text="Por favor, aguarde. O aplicativo abrirá novamente ao concluir.",
            justify=tk.CENTER,
            wraplength=420,
        )
        self.message_label.pack(fill=tk.X, pady=(8, 14))

        self.progress = ttk.Progressbar(frame, mode="indeterminate", length=420)
        self.progress.pack(fill=tk.X)
        self.progress.start(12)

        self.status_label = ttk.Label(frame, text="Preparando atualização...", foreground="#666666")
        self.status_label.pack(fill=tk.X, pady=(12, 0))

        self.root.protocol("WM_DELETE_WINDOW", self._ignore_close)
        self.root.after(400, self._start_update_thread)

    def _ignore_close(self) -> None:
        pass

    def _set_status(self, text: str) -> None:
        self.status_label.config(text=text)
        self.root.update_idletasks()

    def _start_update_thread(self) -> None:
        threading.Thread(target=self._run_update, daemon=True).start()

    def _run_update(self) -> None:
        try:
            log(f"Iniciando atualização para v{self.version}")
            log(f"Instalador: {self.installer_path}")
            log(f"Executável: {self.app_exe}")

            if not os.path.isfile(self.installer_path):
                raise FileNotFoundError(f"Instalador não encontrado: {self.installer_path}")

            self.root.after(0, lambda: self._set_status("Encerrando o FlowVoice..."))
            kill_flowvoice()
            time.sleep(1.5)

            self.root.after(
                0,
                lambda: self._set_status(
                    "Instalando... Se solicitado, confirme a permissão de administrador."
                ),
            )
            exit_code = run_installer_elevated(self.installer_path)
            log(f"Código de saída do instalador: {exit_code}")

            if exit_code != 0:
                self.root.after(
                    0,
                    lambda: self._show_error(
                        f"A instalação falhou (código {exit_code}).\nConsulte: {LOG_PATH}"
                    ),
                )
                return

            self.root.after(0, lambda: self._set_status("Reiniciando o FlowVoice..."))
            time.sleep(0.8)

            if os.path.isfile(self.app_exe):
                relaunch_app(self.app_exe)
                log("FlowVoice reiniciado com sucesso.")
            else:
                log(f"Executável não encontrado após instalação: {self.app_exe}")
                self.root.after(
                    0,
                    lambda: self._show_error(
                        f"Instalação concluída, mas o executável não foi encontrado:\n{self.app_exe}"
                    ),
                )
                return

            self.root.after(0, self.root.destroy)
        except Exception as exc:
            log(f"Erro fatal: {exc}")
            self.root.after(0, lambda: self._show_error(str(exc)))

    def _show_error(self, message: str) -> None:
        self.progress.stop()
        self.title_label.config(text="Erro na atualização")
        self.message_label.config(text=message)
        self.status_label.config(text=f"Log: {LOG_PATH}", foreground="#cc0000")
        self.root.after(15000, self.root.destroy)

    def run(self) -> None:
        self.root.mainloop()


def main() -> None:
    if len(sys.argv) < 4:
        print("Uso: FlowVoiceUpdater.exe <instalador> <versão> <app_exe>")
        sys.exit(1)

    UpdaterWindow(sys.argv[1], sys.argv[2], sys.argv[3]).run()


if __name__ == "__main__":
    main()
