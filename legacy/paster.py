import time
import os
from PySide6.QtWidgets import QApplication
from PySide6.QtCore import QTimer
from PySide6.QtGui import QGuiApplication, QClipboard
from pynput.keyboard import Controller, Key

class TextPaster:
    CLIPBOARD_SYNC_ATTEMPTS = 30
    CLIPBOARD_SYNC_DELAY = 0.02
    PRE_PASTE_DELAY = 0.15
    KEY_INTERVAL = 0.04

    def __init__(self):
        self.keyboard = Controller()

    def _wait_for_clipboard(self, clipboard, expected_text):
        """Ensures the OS clipboard contains the text before simulating Ctrl+V."""
        for _ in range(self.CLIPBOARD_SYNC_ATTEMPTS):
            QApplication.processEvents()
            if clipboard.text() == expected_text:
                return True
            time.sleep(self.CLIPBOARD_SYNC_DELAY)
        return clipboard.text() == expected_text

    def _set_clipboard_text(self, clipboard, text):
        clipboard.setText(text, QClipboard.Mode.Clipboard)
        if hasattr(QClipboard.Mode, "Selection"):
            clipboard.setText(text, QClipboard.Mode.Selection)

    def paste_text(self, text):
        """
        Copies text to the clipboard, simulates Ctrl+V, and keeps the dictated
        text available for manual Ctrl+C afterward.
        MUST be called from the main GUI thread because it accesses QClipboard.
        """
        if not text:
            return

        clipboard = QGuiApplication.clipboard()
        self._set_clipboard_text(clipboard, text)

        if not self._wait_for_clipboard(clipboard, text):
            print("Aviso: área de transferência ainda não sincronizou; tentando colar mesmo assim.")

        time.sleep(self.PRE_PASTE_DELAY)

        # No Wayland (Ubuntu 22.04+ padrão), simular Ctrl+V causa um alerta de segurança do GNOME.
        # Se for Wayland, pulamos a simulação e deixamos o usuário colar manualmente sem alertas.
        is_wayland = os.environ.get("XDG_SESSION_TYPE", "").lower() == "wayland"
        if not is_wayland:
            try:
                self.keyboard.press(Key.ctrl)
                time.sleep(self.KEY_INTERVAL)
                self.keyboard.press('v')
                time.sleep(self.KEY_INTERVAL)
                self.keyboard.release('v')
                time.sleep(self.KEY_INTERVAL)
                self.keyboard.release(Key.ctrl)
                print("Texto colado no campo ativo via Ctrl+V.")
            except Exception as e:
                print(f"Erro ao simular teclas de colagem: {e}")
        else:
            print("Sessão Wayland detectada. O texto foi copiado, aguardando colagem manual.")

        # Keep dictated text in clipboard for Ctrl+C and re-apply after paste handlers run.
        QTimer.singleShot(120, lambda: self._set_clipboard_text(clipboard, text))
        QTimer.singleShot(400, lambda: self._set_clipboard_text(clipboard, text))
