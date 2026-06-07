import os
import json

DEFAULT_CONFIG = {
    "hotkey": "<ctrl>+<shift>+<space>",
    "hotkey_translation": "<ctrl>+<shift>+<y>",
    "hotkey_pesquisa": "<ctrl>+<shift>+<u>",
    "provider": "groq",  # gemini, openai, local
    "local_refinement": "ollama",  # lm_studio, ollama, nenhum
    "active_style": "Direto",  # Profissional, Casual, Direto
    "operation_mode": "ditado",  # ditado, traducao, pesquisa
    "translation_target": "Inglês",  # Inglês, Espanhol, Francês, Alemão, Italiano
    "start_with_windows": True,
    "api_keys": {
        "gemini": "",
        "openai": "",
        "groq": "",
        "github_models": ""
    },
    "whisper": {
        "model_size": "base",  # tiny, base, small
        "device": "cpu"  # cpu, cuda
    },
    "local_llm": {
        "ollama_model": "llama3.2",
        "model_path": "",  # Path to local .gguf file
        "repo_id": "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
        "filename": "qwen2.5-1.5b-instruct-q4_k_m.gguf",
        "n_ctx": 2048
    },
    "language": "pt",
    "prompts": {
        "Profissional": (
            "REGRA CRÍTICA ABSOLUTA: Nunca censure, omita, sanitize, mascare (ex: f***, p***, etc.) ou "
            "substitua palavrões, gírias, termos ofensivos ou palavras rudes. Transcreva-as e mantenha-as "
            "100% de forma literal e integral, sem qualquer tipo de moderação ou suavização. "
            "Você é um assistente de ditado profissional. Transcreva e reformule o texto falado "
            "em uma linguagem formal e profissional de trabalho, corrigindo erros de gramática, "
            "pontuação e concordância. Remova hesitações (como 'hã', 'né', 'tipo', 'então', 'hum'), gagueiras "
            "e repetições de palavras. Entretanto, lembre-se: MANTENHA todos os palavrões exatamente "
            "onde e como foram ditos pelo usuário."
        ),
        "Casual": (
            "REGRA CRÍTICA ABSOLUTA: Nunca censure, omita, sanitize, mascare (ex: f***, p***, etc.) ou "
            "substitua palavrões, gírias, termos ofensivos ou palavras rudes. Transcreva-as e mantenha-as "
            "100% de forma literal e integral. "
            "Você é um assistente de ditado casual. Transcreva o texto falado corrigindo "
            "apenas erros graves de gramática e pontuação, mas mantendo a voz natural, o tom coloquial "
            "e o estilo do usuário. Remova gagueiras e hesitações."
        ),
        "Direto": (
            "REGRA CRÍTICA ABSOLUTA: Nunca censure, omita, sanitize, mascare (ex: f***, p***, etc.) ou "
            "substitua palavrões, gírias, termos ofensivos ou palavras rudes. Transcreva o áudio de entrada "
            "de forma 100% literal, mantendo exatamente as palavras faladas pelo usuário, sem aplicar nenhuma "
            "reesrita, formatação ou correção de estilo. Apenas adicione a pontuação básica necessária para leitura. "
            "Retorne APENAS a transcrição literal."
        )
    }
}

import sys

class ConfigManager:
    def __init__(self, filename="config.json"):
        # Store config in user's Roaming AppData if packaged (so we have write permissions), or locally in development
        if getattr(sys, 'frozen', False):
            appdata = os.getenv('APPDATA')
            if appdata:
                base_dir = os.path.join(appdata, "FlowVoice")
                os.makedirs(base_dir, exist_ok=True)
            else:
                base_dir = os.path.dirname(sys.executable)
        else:
            base_dir = os.path.dirname(os.path.abspath(__file__))
            
        self.filepath = os.path.join(base_dir, filename)
        self.config = {}
        self.load()

    def load(self):
        """Loads configuration from JSON file or creates a default one if not found."""
        if not os.path.exists(self.filepath):
            self.config = DEFAULT_CONFIG.copy()
            self.save()
            return

        try:
            with open(self.filepath, "r", encoding="utf-8") as f:
                loaded = json.load(f)
                # Merge loaded config with default config to ensure all keys exist
                self.config = self._deep_merge(DEFAULT_CONFIG, loaded)
        except Exception as e:
            print(f"Erro ao carregar configurações: {e}. Usando padrões.")
            self.config = DEFAULT_CONFIG.copy()

    def save(self):
        """Saves current configuration to JSON file."""
        try:
            with open(self.filepath, "w", encoding="utf-8") as f:
                json.dump(self.config, f, indent=4, ensure_ascii=False)
        except Exception as e:
            print(f"Erro ao salvar configurações: {e}")

    def _deep_merge(self, default, target):
        """Recursively merges target dictionary into default to guarantee keys exist."""
        result = default.copy()
        for key, value in target.items():
            if key in result:
                if isinstance(result[key], dict) and isinstance(value, dict):
                    result[key] = self._deep_merge(result[key], value)
                else:
                    result[key] = value
            else:
                result[key] = value
        return result

    def get(self, key, default=None):
        return self.config.get(key, default)

    def set(self, key, value):
        self.config[key] = value
        self.save()

    def get_api_key(self, provider):
        return self.config.get("api_keys", {}).get(provider, "")

    def get_api_keys_list(self, provider):
        """Retorna uma lista de chaves limpas para o provedor, separadas por vírgula."""
        raw_keys = self.get_api_key(provider)
        if not raw_keys:
            return []
        # Divide por vírgula e limpa espaços extras
        return [k.strip() for k in raw_keys.split(",") if k.strip()]

    def set_api_key(self, provider, key_value):
        if "api_keys" not in self.config:
            self.config["api_keys"] = {}
        self.config["api_keys"][provider] = key_value
        self.save()

    def get_prompt_for_active_style(self):
        style = self.config.get("active_style", "Profissional")
        return self.config.get("prompts", {}).get(style, DEFAULT_CONFIG["prompts"]["Profissional"])
