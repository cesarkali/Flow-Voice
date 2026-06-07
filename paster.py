import time
from PySide6.QtCore import QTimer
from PySide6.QtGui import QGuiApplication
from pynput.keyboard import Controller, Key

class TextPaster:
    def __init__(self):
        self.keyboard = Controller()

    def paste_text(self, text):
        """
        Pastes text by backing up the clipboard, setting the text,
        simulating Ctrl+V, and restoring the clipboard.
        MUST be called from the main GUI thread because it accesses QClipboard.
        """
        if not text:
            return

        # Access system clipboard
        clipboard = QGuiApplication.clipboard()
        
        # 1. Backup old clipboard text
        old_text = clipboard.text()
        
        # 2. Put new text in clipboard
        clipboard.setText(text)
        
        # Allow clipboard to register the update (a tiny delay is safe)
        time.sleep(0.05)
        
        # 3. Simulate Ctrl+V key press
        try:
            # Press Ctrl
            self.keyboard.press(Key.ctrl)
            # Press V (we use 'v' as a string or the key code)
            self.keyboard.press('v')
            
            # Release V
            self.keyboard.release('v')
            # Release Ctrl
            self.keyboard.release(Key.ctrl)
            
            print("Texto colado no campo ativo via Ctrl+V.")
        except Exception as e:
            print(f"Erro ao simular teclas de colagem: {e}")

        # 4. Restore original clipboard content on the main GUI thread after a delay
        # This resolves the COM CoInitialize error because it runs in the main thread context
        QTimer.singleShot(500, lambda: self._restore_clipboard(old_text))

    def _restore_clipboard(self, old_text):
        """Restores original clipboard text."""
        try:
            clipboard = QGuiApplication.clipboard()
            clipboard.setText(old_text)
            print("Área de transferência original restaurada.")
        except Exception as e:
            print(f"Erro ao restaurar área de transferência: {e}")
