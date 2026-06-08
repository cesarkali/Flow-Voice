import time
from PySide6.QtWidgets import QApplication
from PySide6.QtCore import QTimer
from PySide6.QtGui import QGuiApplication, QClipboard
from pynput.keyboard import Controller, Key

class TextPaster:
    CLIPBOARD_SYNC_ATTEMPTS = 25
    CLIPBOARD_SYNC_DELAY = 0.02
    PRE_PASTE_DELAY = 0.12
    KEY_INTERVAL = 0.03
    CLIPBOARD_RESTORE_DELAY = 1500

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

    def paste_text(self, text):
        """
        Pastes text by backing up the clipboard, setting the text,
        simulating Ctrl+V, and restoring the clipboard.
        MUST be called from the main GUI thread because it accesses QClipboard.
        """
        if not text:
            return

        clipboard = QGuiApplication.clipboard()
        old_text = clipboard.text()

        clipboard.setText(text, QClipboard.Mode.Clipboard)
        if not self._wait_for_clipboard(clipboard, text):
            print("Aviso: área de transferência ainda não sincronizou; tentando colar mesmo assim.")

        time.sleep(self.PRE_PASTE_DELAY)

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

        QTimer.singleShot(self.CLIPBOARD_RESTORE_DELAY, lambda: self._restore_clipboard(old_text))

    def _restore_clipboard(self, old_text):
        """Restores original clipboard text."""
        try:
            clipboard = QGuiApplication.clipboard()
            clipboard.setText(old_text, QClipboard.Mode.Clipboard)
            print("Área de transferência original restaurada.")
        except Exception as e:
            print(f"Erro ao restaurar área de transferência: {e}")
