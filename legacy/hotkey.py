import threading
from pynput import keyboard

def sanitize_hotkey(hotkey_str):
    """Ensure all special keys and modifiers are wrapped in brackets for pynput."""
    parts = hotkey_str.lower().strip().split("+")
    sanitized_parts = []
    special_keys = {"ctrl", "shift", "alt", "win", "space", "enter", "tab", "esc", "escape"}
    
    for part in parts:
        part_clean = part.replace("<", "").replace(">", "").strip()
        if part_clean in special_keys:
            sanitized_parts.append(f"<{part_clean}>")
        else:
            sanitized_parts.append(part_clean)
            
    return "+".join(sanitized_parts)

class HotkeyListener:
    def __init__(self, hotkeys_map):
        """
        hotkeys_map: dict of {hotkey_str: callback_fn}
        """
        self.hotkeys_map = {sanitize_hotkey(k): v for k, v in hotkeys_map.items() if k}
        self.listener = None
        self.lock = threading.RLock()

    def start(self):
        """Starts the global hotkey listener in a background thread."""
        with self.lock:
            if self.listener is not None:
                self.stop()
            
            if not self.hotkeys_map:
                print("Nenhum atalho registrado para escuta.")
                return

            try:
                # Map hotkey string to the callback function
                self.listener = keyboard.GlobalHotKeys(self.hotkeys_map)
                self.listener.start()
                print(f"Ouvinte de atalhos globais iniciado para: {list(self.hotkeys_map.keys())}")
            except Exception as e:
                print(f"Erro ao iniciar atalhos globais: {e}")

    def stop(self):
        """Stops the current listener if running."""
        with self.lock:
            if self.listener is not None:
                try:
                    self.listener.stop()
                    print("Ouvinte de atalho global parado.")
                except Exception as e:
                    print(f"Erro ao parar atalho global: {e}")
                self.listener = None

    def update_hotkeys(self, new_hotkeys_map):
        """Updates the hotkeys mapping and restarts the listener."""
        sanitized_new = {sanitize_hotkey(k): v for k, v in new_hotkeys_map.items() if k}
        if self.hotkeys_map != sanitized_new:
            self.hotkeys_map = sanitized_new
            self.start()

    def update_hotkey(self, new_hotkey_str):
        """Legacy compatibility wrapper for single hotkey update."""
        new_hotkey_str = sanitize_hotkey(new_hotkey_str)
        if len(self.hotkeys_map) == 1:
            old_key = list(self.hotkeys_map.keys())[0]
            cb = self.hotkeys_map[old_key]
            self.hotkeys_map = {new_hotkey_str: cb}
            self.start()
