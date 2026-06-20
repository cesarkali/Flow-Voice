"""
i18n.py — Internationalization module for FlowVoice.
Loads locale JSON files from the locales/ directory and exposes tr() and set_language().
"""
import json
import os
import sys

# Runtime language code — default Portuguese
_LANG: str = "pt"
_cache: dict = {}

SUPPORTED = ("pt", "en", "es")


def _get_locales_dir() -> str:
    """Returns the path to the locales/ directory, compatible with PyInstaller."""
    if getattr(sys, "frozen", False):
        # PyInstaller extracts bundled files to sys._MEIPASS, not next to the .exe
        base = getattr(sys, "_MEIPASS", os.path.dirname(sys.executable))
    else:
        base = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(base, "locales")


def _load(lang: str) -> dict:
    """Load and return a locale dict for the given language code."""
    locales_dir = _get_locales_dir()
    path = os.path.join(locales_dir, f"{lang}.json")
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except Exception as e:
        print(f"[i18n] Failed to load locale '{lang}': {e}")
        return {}


def set_language(lang_code: str, config_manager=None) -> None:
    """Set the active UI language and optionally persist it to config."""
    global _LANG, _cache
    if lang_code in SUPPORTED:
        _LANG = lang_code
        _cache = _load(lang_code)
        if not _cache:
            # Fallback: load Portuguese if target locale failed
            _LANG = "pt"
            _cache = _load("pt")
    if config_manager is not None:
        config_manager.set("ui_language", lang_code)


def get_language() -> str:
    """Return the currently active language code."""
    return _LANG


def tr(key: str, *args) -> str:
    """
    Return the translated string for the current language.
    Falls back to the key itself if not found.
    Supports positional .format() args: tr("hello_{}", name)
    """
    global _cache
    text = _cache.get(key, key)
    if args:
        try:
            return text.format(*args)
        except (IndexError, KeyError):
            return text
    return text


# Initialise with default language on import
_cache = _load(_LANG)
