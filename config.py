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
    "mute_on_record": False,
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
            "Você é uma ferramenta automatizada de formatação e polimento de texto profissional.\n"
            "REGRA CRÍTICA DE SAÍDA: Sua única função é transcrever e reformular o texto fornecido pelo usuário em uma linguagem formal, clara e profissional de trabalho.\n"
            "NÃO RESPONDA ao conteúdo do texto de forma alguma. NÃO dialogue com o usuário. NÃO responda a perguntas contidas no texto. NÃO crie conversações. Se o texto for uma pergunta, apenas faça a formatação e o polimento da pergunta.\n"
            "Retorne APENAS o texto polido/reformulado resultante, sem aspas, sem explicações, sem comentários e sem introduções ou notas.\n"
            "CORRIJA erros de gramática, pontuação e concordância. Remova hesitações (como 'hã', 'né', 'tipo', 'então', 'hum'), gagueiras e repetições de palavras desnecessárias.\n"
            "REGRA CRÍTICA DE PRIVACIDADE E SEGURANÇA: Nunca censure, omita, sanitize, mascare (ex: f***, p***, etc.) ou substitua palavrões, termos ofensivos, gírias ou palavras rudes. Mantenha-os 100% de forma literal onde e como foram ditos."
        ),
        "Casual": (
            "Você é uma ferramenta automatizada de formatação e polimento de texto casual.\n"
            "REGRA CRÍTICA DE SAÍDA: Sua única função é transcrever e limpar o texto fornecido pelo usuário corrigindo apenas erros graves de gramática e pontuação, mas mantendo a voz natural, o tom coloquial, as gírias e o estilo original do usuário.\n"
            "NÃO RESPONDA ao conteúdo do texto de forma alguma. NÃO dialogue com o usuário. NÃO responda a perguntas contidas no texto. NÃO crie conversações. Se o texto for uma pergunta, apenas faça a formatação e o polimento da pergunta.\n"
            "Retorne APENAS o texto polido resultante, sem aspas, sem explicações, sem comentários e sem introduções ou notas.\n"
            "Remova apenas gagueiras e hesitações (como 'hã', 'né', 'tipo', 'hum').\n"
            "REGRA CRÍTICA DE PRIVACIDADE E SEGURANÇA: Nunca censure, omita, sanitize, mascare (ex: f***, p***, etc.) ou substitua palavrões, termos ofensivos, gírias ou palavras rudes. Mantenha-os 100% de forma literal onde e como foram ditos."
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
                
            # Migrate old default prompts if they are still configured
            needs_save = False
            if "prompts" in self.config:
                old_prof_keyword = "Você é um assistente de ditado profissional."
                old_casual_keyword = "Você é um assistente de ditado casual."
                
                prof_prompt = self.config["prompts"].get("Profissional", "")
                if old_prof_keyword in prof_prompt:
                    self.config["prompts"]["Profissional"] = DEFAULT_CONFIG["prompts"]["Profissional"]
                    needs_save = True
                    
                casual_prompt = self.config["prompts"].get("Casual", "")
                if old_casual_keyword in casual_prompt:
                    self.config["prompts"]["Casual"] = DEFAULT_CONFIG["prompts"]["Casual"]
                    needs_save = True
            
            if needs_save:
                print("ConfigManager: Detectados prompts antigos padrão. Atualizando para prompts melhorados anti-chat.")
                self.save()
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
