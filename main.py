import sys
import os
import logging

# Suppress Hugging Face Hub warnings and logs
os.environ["HF_HUB_DISABLE_SYMLINKS_WARNING"] = "1"
logging.getLogger("huggingface_hub").setLevel(logging.ERROR)

# Add NVIDIA CUDA/cuDNN DLL paths to search path on Windows
if sys.platform == 'win32':
    try:
        import site
        site_paths = []
        try:
            site_paths.extend(site.getsitepackages())
        except Exception:
            pass
        try:
            site_paths.append(site.getusersitepackages())
        except Exception:
            pass
            
        for site_dir in site_paths:
            if site_dir and os.path.isdir(site_dir):
                nvidia_base = os.path.join(site_dir, "nvidia")
                if os.path.isdir(nvidia_base):
                    for root, dirs, files in os.walk(nvidia_base):
                        if "bin" in dirs:
                            bin_path = os.path.join(root, "bin")
                            try:
                                if os.path.isdir(bin_path) and any(f.lower().endswith(".dll") for f in os.listdir(bin_path)):
                                    os.add_dll_directory(bin_path)
                                    os.environ["PATH"] = bin_path + os.pathsep + os.environ["PATH"]
                                    print(f"Adicionado diretório de DLL ao path: {bin_path}")
                            except Exception as e:
                                print(f"Erro ao adicionar DLL path {bin_path}: {e}")
    except Exception as e:
        print(f"Erro ao carregar caminhos de DLL NVIDIA: {e}")

import time
import subprocess
import urllib.request
import json
import re
import tempfile
from PySide6.QtWidgets import (
    QApplication, QWidget, QHBoxLayout, QVBoxLayout, QLabel, 
    QSystemTrayIcon, QMenu, QDialog, QFormLayout, QLineEdit, 
    QComboBox, QPushButton, QMessageBox, QFrame, QGraphicsDropShadowEffect,
    QTextEdit, QCheckBox, QProgressBar, QScrollArea, QTabWidget,
    QGraphicsOpacityEffect, QStackedWidget
)
from PySide6.QtCore import Qt, QTimer, QThread, Signal, Slot, QPropertyAnimation, QParallelAnimationGroup, QEasingCurve, QRect, QAbstractAnimation, QUrl
from PySide6.QtGui import QIcon, QColor, QFont, QAction, QPainter, QBrush, QPen, QPixmap, QDesktopServices
from PySide6.QtSvg import QSvgRenderer
from PySide6 import QtSvg

from version import VERSION

CURRENT_VERSION = VERSION

# ── Internationalization — loaded from locales/ JSON files ────────────────────
from i18n import tr, set_language, get_language


# Safe pycaw import for Windows volume control
try:
    from pycaw.pycaw import AudioUtilities
    PYCAW_AVAILABLE = True
except ImportError:
    PYCAW_AVAILABLE = False

_previous_mute_state = None

def mute_system_audio():
    global _previous_mute_state
    if not PYCAW_AVAILABLE:
        return
    try:
        devices = AudioUtilities.GetSpeakers()
        volume = devices.EndpointVolume
        _previous_mute_state = volume.GetMute()
        volume.SetMute(1, None)
        print(f"Áudio do sistema mutado (estado anterior: {_previous_mute_state})")
    except Exception as e:
        print(f"Erro ao mutar áudio do sistema: {e}")

def unmute_system_audio():
    global _previous_mute_state
    if not PYCAW_AVAILABLE:
        return
    try:
        if _previous_mute_state is not None:
            devices = AudioUtilities.GetSpeakers()
            volume = devices.EndpointVolume
            volume.SetMute(_previous_mute_state, None)
            print(f"Áudio do sistema restaurado para: {_previous_mute_state}")
            _previous_mute_state = None
    except Exception as e:
        print(f"Erro ao restaurar áudio do sistema: {e}")

# Import application modules
from config import ConfigManager, get_app_data_dir, get_resource_path
from hotkey import HotkeyListener
from recorder import AudioRecorder
from ai_processor import AIProcessor
from paster import TextPaster

# Custom thread worker for AI processing
class AIWorker(QThread):
    finished = Signal(str, str)
    error = Signal(str)
    status_changed = Signal(str)

    def __init__(self, processor, audio_path, mode, target_lang="Inglês"):
        super().__init__()
        self.processor = processor
        self.audio_path = audio_path
        self.mode = mode
        self.target_lang = target_lang

    def run(self):
        try:
            # Transcribe and format
            text = self.processor.transcribe_and_process(
                self.audio_path, 
                mode=self.mode,
                target_lang=self.target_lang,
                status_callback=self.status_changed.emit
            )
            self.finished.emit(text, self.mode)
        except Exception as e:
            self.error.emit(str(e))

# Thread for interactive chat assistant
class ChatWorker(QThread):
    finished = Signal(str, str)  # (transcription, response)
    error = Signal(str)
    status_changed = Signal(str)

    def __init__(self, processor, messages, audio_path=None):
        super().__init__()
        self.processor = processor
        self.messages = messages
        self.audio_path = audio_path

    def run(self):
        try:
            transcribed_text = None
            if self.audio_path:
                self.status_changed.emit("Transcrevendo voz...")
                transcribed_text = self.processor._transcribe_audio(self.audio_path)
                if not transcribed_text:
                    raise ValueError("Nenhuma fala detectada.")
                self.status_changed.emit(f"Você: {transcribed_text}")
                time.sleep(0.5)

            api_messages = []
            for msg in self.messages:
                if msg["role"] in ["user", "assistant", "system"]:
                    api_messages.append({
                        "role": msg["role"],
                        "content": msg["content"]
                    })
            
            if transcribed_text:
                api_messages.append({
                    "role": "user",
                    "content": transcribed_text
                })

            self.status_changed.emit("IA pensando...")
            response = self.processor.chat_via_pool(api_messages)
            
            if not response:
                raise RuntimeError("Falha ao obter resposta de todos os provedores da IA.")

            self.finished.emit(transcribed_text or "", response)
        except Exception as e:
            self.error.emit(str(e))
        finally:
            if self.audio_path and os.path.exists(self.audio_path):
                try:
                    os.remove(self.audio_path)
                except Exception:
                    pass

def get_installer_asset_name(target_version=None):
    if sys.platform == 'win32':
        return "FlowVoiceSetup.exe"
    version_to_use = target_version if target_version else CURRENT_VERSION
    return f"flowvoice_{version_to_use}_amd64.deb"

# Helper functions for version updates
def get_latest_release():
    url = "https://api.github.com/repos/cesarkali/Flow-Voice/releases/latest"
    req = urllib.request.Request(
        url,
        headers={'User-Agent': 'FlowVoice-Updater'}
    )
    with urllib.request.urlopen(req, timeout=7) as response:
        data = json.loads(response.read().decode('utf-8'))
        tag_name = data.get("tag_name", "")
        # Clean version tag (e.g. "v1.0.1" -> "1.0.1")
        version_match = re.search(r"(\d+\.\d+\.\d+)", tag_name)
        if not version_match:
            return None
        latest_ver = version_match.group(1)
        
        asset_name = get_installer_asset_name(latest_ver)
        
        download_url = None
        for asset in data.get("assets", []):
            if asset.get("name") == asset_name:
                download_url = asset.get("browser_download_url")
                break
        if not download_url:
            download_url = f"https://github.com/cesarkali/Flow-Voice/releases/download/{tag_name}/{asset_name}"
            
        return latest_ver, download_url

def is_version_newer(current, latest):
    try:
        def parse_version(v_str):
            return [int(x) for x in re.findall(r"\d+", v_str)]
        return parse_version(latest) > parse_version(current)
    except Exception:
        return False

# Background thread to check for updates
class UpdateCheckerWorker(QThread):
    update_available = Signal(str, str) # version, download_url
    no_update_found = Signal()
    error = Signal(str)
    
    def run(self):
        try:
            res = get_latest_release()
            if res:
                latest_ver, download_url = res
                if is_version_newer(CURRENT_VERSION, latest_ver):
                    self.update_available.emit(latest_ver, download_url)
                else:
                    self.no_update_found.emit()
            else:
                self.error.emit("Nenhuma versão válida foi encontrada no GitHub.")
        except Exception as e:
            self.error.emit(f"Falha de conexão com o GitHub: {str(e)}")

def get_app_executable_path():
    """Returns the installed FlowVoice executable path for restart after updates."""
    if getattr(sys, 'frozen', False):
        return sys.executable
    program_files = os.environ.get('ProgramFiles', r'C:\Program Files')
    return os.path.join(program_files, 'FlowVoice', 'FlowVoice.exe')


def get_updater_path():
    """Returns the path to the standalone updater executable or script."""
    if getattr(sys, 'frozen', False):
        return os.path.join(os.path.dirname(sys.executable), "FlowVoiceUpdater.exe")
    return os.path.join(os.path.dirname(os.path.abspath(__file__)), "updater.py")


def launch_windows_update(installer_path, target_version):
    """Starts the dedicated updater process and returns immediately."""
    app_exe = get_app_executable_path()
    updater_path = get_updater_path()

    if updater_path.endswith(".py"):
        command = [sys.executable, updater_path, installer_path, target_version, app_exe]
    else:
        if not os.path.isfile(updater_path):
            raise FileNotFoundError(
                f"Atualizador não encontrado: {updater_path}. "
                "Reinstale o FlowVoice manualmente uma vez para incluir o FlowVoiceUpdater.exe."
            )
        command = [updater_path, installer_path, target_version, app_exe]

    subprocess.Popen(
        command,
        close_fds=True,
        creationflags=subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP,
    )


# Background thread to download the installer
class DownloadWorker(QThread):
    progress = Signal(int) # percentage
    finished = Signal(str) # temp file path
    error = Signal(str)
    
    def __init__(self, url, target_version=None):
        super().__init__()
        self.url = url
        self.target_version = target_version or VERSION
        
    def run(self):
        try:
            temp_dir = tempfile.gettempdir()
            if sys.platform == 'win32':
                dest_name = "FlowVoiceSetup_update.exe"
            else:
                dest_name = f"flowvoice_{self.target_version}_amd64.deb"
            dest_path = os.path.join(temp_dir, dest_name)
            
            # Download file chunk by chunk to calculate progress
            req = urllib.request.Request(self.url, headers={'User-Agent': 'FlowVoice-Updater'})
            with urllib.request.urlopen(req) as response:
                total_size = int(response.headers.get('content-length', 0))
                downloaded = 0
                chunk_size = 8192 * 4
                
                with open(dest_path, 'wb') as f:
                    while True:
                        chunk = response.read(chunk_size)
                        if not chunk:
                            break
                        f.write(chunk)
                        downloaded += len(chunk)
                        if total_size > 0:
                            percent = int((downloaded / total_size) * 100)
                            self.progress.emit(percent)
                               
            self.finished.emit(dest_path)
        except Exception as e:
            self.error.emit(str(e))

# Modern premium Dialog to inform about update and manage downloading
class UpdateDialog(QDialog):
    def __init__(self, latest_version, download_url, parent=None):
        super().__init__(parent)
        self.latest_version = latest_version
        self.download_url = download_url
        self.download_worker = None
        self.drag_position = None
        self.init_ui()

    def init_ui(self):
        self.setWindowFlags(Qt.FramelessWindowHint | Qt.Dialog | Qt.WindowStaysOnTopHint)
        self.setAttribute(Qt.WA_TranslucentBackground, True)
        self.setFixedSize(450, 260)

        # Main glassmorphism container
        self.container_frame = QFrame(self)
        self.container_frame.setObjectName("container_frame")
        self.container_frame.setGeometry(10, 10, 430, 240)
        self.container_frame.setStyleSheet("""
            QFrame#container_frame {
                background-color: rgba(12, 12, 12, 248);
                border: 1px solid rgba(255, 255, 255, 30);
                border-radius: 12px;
            }
        """)

        # Shadow effect
        shadow = QGraphicsDropShadowEffect(self)
        shadow.setBlurRadius(20)
        shadow.setColor(QColor(0, 0, 0, 200))
        shadow.setOffset(0, 4)
        self.container_frame.setGraphicsEffect(shadow)

        # Layout inside container
        layout = QVBoxLayout(self.container_frame)
        layout.setContentsMargins(20, 16, 20, 20)
        layout.setSpacing(12)

        # Title
        self.lbl_title = QLabel("ATUALIZAÇÃO DISPONÍVEL")
        title_font = QFont("Segoe UI", 9)
        title_font.setBold(True)
        self.lbl_title.setFont(title_font)
        self.lbl_title.setStyleSheet("color: #8b5cf6; letter-spacing: 1.5px;")
        layout.addWidget(self.lbl_title)

        # Separator
        sep = QFrame()
        sep.setFixedHeight(1)
        sep.setStyleSheet("background-color: rgba(255, 255, 255, 15);")
        layout.addWidget(sep)

        # Description
        self.lbl_desc = QLabel(
            f"Uma nova versão (v{self.latest_version}) do FlowVoice está disponível no GitHub.<br/>"
            "Deseja baixar e atualizar agora automaticamente?"
        )
        self.lbl_desc.setWordWrap(True)
        self.lbl_desc.setStyleSheet("color: rgba(255, 255, 255, 200); font-size: 13px; font-family: 'Segoe UI', sans-serif; line-height: 1.4;")
        layout.addWidget(self.lbl_desc)

        layout.addStretch()

        # Progress bar (hidden initially)
        self.progress_bar = QProgressBar()
        self.progress_bar.setRange(0, 100)
        self.progress_bar.setValue(0)
        self.progress_bar.setFixedHeight(8)
        self.progress_bar.setTextVisible(False)
        self.progress_bar.setStyleSheet("""
            QProgressBar {
                background-color: rgba(255, 255, 255, 10);
                border: none;
                border-radius: 4px;
            }
            QProgressBar::chunk {
                background-color: #ffffff;
                border-radius: 4px;
            }
        """)
        self.progress_bar.hide()
        layout.addWidget(self.progress_bar)

        self.lbl_progress = QLabel("Baixando... 0%")
        self.lbl_progress.setStyleSheet("color: rgba(255, 255, 255, 140); font-size: 11px; font-family: 'Segoe UI', sans-serif;")
        self.lbl_progress.hide()
        layout.addWidget(self.lbl_progress)

        # Buttons
        self.btn_layout = QHBoxLayout()
        self.btn_layout.setSpacing(10)
        
        self.btn_cancel = QPushButton("Ignorar")
        self.btn_cancel.setObjectName("btn_cancel")
        self.btn_cancel.setCursor(Qt.PointingHandCursor)
        self.btn_cancel.setStyleSheet("""
            QPushButton#btn_cancel {
                background-color: transparent;
                border: 1px solid rgba(255, 255, 255, 25);
                border-radius: 6px;
                color: rgba(255, 255, 255, 160);
                padding: 6px 14px;
                font-size: 12px;
                font-family: 'Segoe UI', sans-serif;
                font-weight: bold;
                min-height: 28px;
            }
            QPushButton#btn_cancel:hover {
                background-color: rgba(255, 255, 255, 12);
                color: #ffffff;
            }
        """)
        self.btn_cancel.clicked.connect(self.reject)

        self.btn_update = QPushButton("Baixar e Instalar")
        self.btn_update.setObjectName("btn_update")
        self.btn_update.setCursor(Qt.PointingHandCursor)
        self.btn_update.setStyleSheet("""
            QPushButton#btn_update {
                background-color: #ffffff;
                border: none;
                border-radius: 6px;
                color: #000000;
                padding: 6px 14px;
                font-size: 12px;
                font-family: 'Segoe UI', sans-serif;
                font-weight: bold;
                min-height: 28px;
            }
            QPushButton#btn_update:hover {
                background-color: rgba(255, 255, 255, 220);
            }
        """)
        self.btn_update.clicked.connect(self.start_download)

        self.btn_layout.addWidget(self.btn_cancel)
        self.btn_layout.addWidget(self.btn_update)
        layout.addLayout(self.btn_layout)

    # Window Dragging Logic
    def mousePressEvent(self, event):
        if event.button() == Qt.LeftButton and event.position().y() < 40:
            self.drag_position = event.globalPosition().toPoint() - self.frameGeometry().topLeft()
            event.accept()
        else:
            self.drag_position = None
            super().mousePressEvent(event)

    def mouseMoveEvent(self, event):
        if event.buttons() == Qt.LeftButton and self.drag_position is not None:
            self.move(event.globalPosition().toPoint() - self.drag_position)
            event.accept()
        else:
            super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event):
        self.drag_position = None
        super().mouseReleaseEvent(event)

    def start_download(self):
        # Hide standard action buttons and show progress bar
        self.btn_cancel.hide()
        self.btn_update.hide()
        self.progress_bar.show()
        self.lbl_progress.show()
        self.lbl_desc.setText(f"Baixando a versão v{self.latest_version} do GitHub...")
        self.lbl_title.setText("BAIXANDO ATUALIZAÇÃO")
        self.lbl_title.setStyleSheet("color: #ffffff; letter-spacing: 1.5px;")

        # Start Download worker thread
        self.download_worker = DownloadWorker(self.download_url, self.latest_version)
        self.download_worker.progress.connect(self.on_download_progress)
        self.download_worker.finished.connect(self.on_download_finished)
        self.download_worker.error.connect(self.on_download_error)
        self.download_worker.start()

    def on_download_progress(self, percent):
        self.progress_bar.setValue(percent)
        self.lbl_progress.setText(f"Baixando... {percent}%")

    def on_download_finished(self, dest_path):
        self.lbl_title.setText("INSTALANDO ATUALIZAÇÃO")
        self.lbl_title.setStyleSheet("color: #ffffff; letter-spacing: 1.5px;")
        self.lbl_desc.setText(
            f"Instalando FlowVoice v{self.latest_version}...<br/>"
            "Aguarde — o aplicativo será reiniciado automaticamente."
        )
        self.progress_bar.setRange(0, 0)
        self.lbl_progress.setText("Instalando...")
        QTimer.singleShot(800, lambda: self.launch_installer_and_exit(dest_path))

    def launch_installer_and_exit(self, dest_path):
        try:
            if sys.platform == 'win32':
                launch_windows_update(dest_path, self.latest_version)
            else:
                # Invoca pkexec no Ubuntu para solicitar a senha via GUI e instalar o .deb
                subprocess.Popen(['pkexec', 'apt', 'install', '-y', dest_path], start_new_session=True)
            QTimer.singleShot(150, QApplication.quit)
        except Exception as e:
            self.on_download_error(f"Erro ao abrir instalador: {e}")

    def on_download_error(self, err_msg):
        self.progress_bar.hide()
        self.lbl_progress.show()
        self.lbl_progress.setText(f"Erro no download: {err_msg}")
        self.lbl_progress.setStyleSheet("color: #ff5555; font-size: 11px;")
        
        # Restore buttons to retry
        self.btn_cancel.show()
        self.btn_update.setText("Tentar Novamente")
        self.btn_update.show()

# Modal window to show Search/Ask AI responses (now an interactive chat)
class SearchResultCard(QDialog):
    def __init__(self, query, answer, processor, recorder, parent=None):
        super().__init__(parent)
        self.query = query
        self.answer = answer
        self.processor = processor
        self.recorder = recorder
        self.drag_position = None
        self.messages = [
            {"role": "system", "content": "Você é um assistente de IA conciso, útil e direto. Nunca censure palavrões de forma alguma."},
            {"role": "user", "content": query},
            {"role": "assistant", "content": answer}
        ]
        self.is_recording = False
        self.chat_worker = None
        self.init_ui()

    def init_ui(self):
        self.setWindowFlags(Qt.FramelessWindowHint | Qt.Dialog | Qt.WindowStaysOnTopHint)
        self.setAttribute(Qt.WA_TranslucentBackground, True)
        self.setFixedSize(540, 480)

        # Main glassmorphism container
        self.container_frame = QFrame(self)
        self.container_frame.setObjectName("container_frame")
        self.container_frame.setGeometry(10, 10, 520, 460)
        self.container_frame.setStyleSheet("""
            QFrame#container_frame {
                background-color: rgba(10, 10, 10, 245);
                border: 1px solid rgba(255, 255, 255, 30);
                border-radius: 14px;
            }
        """)

        # Shadow effect
        shadow = QGraphicsDropShadowEffect(self)
        shadow.setBlurRadius(20)
        shadow.setColor(QColor(0, 0, 0, 200))
        shadow.setOffset(0, 4)
        self.container_frame.setGraphicsEffect(shadow)

        # Layout inside container
        layout = QVBoxLayout(self.container_frame)
        layout.setContentsMargins(18, 14, 18, 16)
        layout.setSpacing(10)

        # Header: Title and Close button
        header_layout = QHBoxLayout()
        header_layout.setContentsMargins(0, 0, 0, 0)
        
        lbl_title = QLabel("FLOWVOICE ASSISTENTE")
        title_font = QFont("Segoe UI", 9)
        title_font.setBold(True)
        lbl_title.setFont(title_font)
        lbl_title.setStyleSheet("color: #ffffff; letter-spacing: 1.5px;")
        header_layout.addWidget(lbl_title)
        
        header_layout.addStretch()
        btn_close = QPushButton("✕")
        btn_close.setObjectName("btn_close")
        btn_close.setFixedSize(20, 20)
        btn_close.setStyleSheet("""
            QPushButton#btn_close {
                background-color: transparent;
                border: none;
                color: rgba(255, 255, 255, 12);
                font-size: 13px;
                font-weight: bold;
            }
            QPushButton#btn_close:hover {
                color: #ff5555;
            }
        """)
        btn_close.clicked.connect(self.fade_out_and_close)
        btn_close.setAutoDefault(False)
        header_layout.addWidget(btn_close)
        layout.addLayout(header_layout)

        # Separator line
        sep = QFrame()
        sep.setFixedHeight(1)
        sep.setStyleSheet("background-color: rgba(255, 255, 255, 15);")
        layout.addWidget(sep)

        # AI Answer & Chat History (Scrollable, Selectable)
        self.text_display = QTextEdit()
        self.text_display.setReadOnly(True)
        self.text_display.setStyleSheet("""
            QTextEdit {
                background-color: transparent;
                border: none;
                color: #ffffff;
                font-size: 13px;
                font-family: 'Segoe UI', sans-serif;
                line-height: 140%;
            }
            QScrollBar:vertical {
                border: none;
                background: transparent;
                width: 4px;
                margin: 0px;
            }
            QScrollBar::handle:vertical {
                background: rgba(255, 255, 255, 40);
                min-height: 20px;
                border-radius: 2px;
            }
            QScrollBar::handle:vertical:hover {
                background: rgba(255, 255, 255, 80);
            }
            QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
                border: none;
                background: none;
            }
        """)
        layout.addWidget(self.text_display, 1)

        # Bottom Input Bar (follow-up text field and voice response button)
        input_layout = QHBoxLayout()
        input_layout.setSpacing(8)
        
        self.input_field = QLineEdit()
        self.input_field.setPlaceholderText("Escreva uma resposta e aperte Enter...")
        self.input_field.setStyleSheet("""
            QLineEdit {
                background-color: rgba(255, 255, 255, 8);
                border: 1px solid rgba(255, 255, 255, 20);
                border-radius: 8px;
                padding: 8px 12px;
                color: #ffffff;
                font-size: 12px;
                font-family: 'Segoe UI', sans-serif;
            }
            QLineEdit:focus {
                border: 1px solid rgba(255, 255, 255, 80);
                background-color: rgba(255, 255, 255, 12);
            }
        """)
        self.input_field.returnPressed.connect(self.send_text_message)
        input_layout.addWidget(self.input_field, 1)
        
        self.btn_mic = QPushButton("🎤")
        self.btn_mic.setFixedSize(34, 34)
        self.btn_mic.setStyleSheet("""
            QPushButton {
                background-color: rgba(255, 255, 255, 12);
                border: 1px solid rgba(255, 255, 255, 20);
                border-radius: 17px;
                font-size: 14px;
                color: #ffffff;
            }
            QPushButton:hover {
                background-color: rgba(255, 255, 255, 25);
                border: 1px solid rgba(255, 255, 255, 60);
            }
        """)
        self.btn_mic.clicked.connect(self.toggle_voice_response)
        self.btn_mic.setAutoDefault(False)
        input_layout.addWidget(self.btn_mic)
        
        layout.addLayout(input_layout)

        # Footer Buttons
        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(10)
        
        btn_copy = QPushButton("Copiar Resposta")
        btn_copy.setStyleSheet("""
            QPushButton {
                background-color: #ffffff;
                color: #000000;
                border: none;
                border-radius: 6px;
                font-weight: 700;
                padding: 6px 16px;
                font-size: 11px;
                font-family: 'Segoe UI', sans-serif;
            }
            QPushButton:hover {
                background-color: rgba(255, 255, 255, 220);
            }
        """)
        btn_copy.clicked.connect(self.copy_answer)
        btn_copy.setAutoDefault(False)
        
        btn_dismiss = QPushButton("Fechar")
        btn_dismiss.setStyleSheet("""
            QPushButton {
                background-color: transparent;
                border: 1px solid rgba(255, 255, 255, 25);
                border-radius: 6px;
                color: rgba(255, 255, 255, 160);
                padding: 6px 16px;
                font-size: 11px;
                font-family: 'Segoe UI', sans-serif;
            }
            QPushButton:hover {
                background-color: rgba(255, 255, 255, 12);
                color: #ffffff;
                border: 1px solid rgba(255, 255, 255, 60);
            }
        """)
        btn_dismiss.clicked.connect(self.fade_out_and_close)
        btn_dismiss.setAutoDefault(False)
        
        btn_layout.addWidget(btn_dismiss)
        btn_layout.addWidget(btn_copy)
        layout.addLayout(btn_layout)

        # Initial render of the chat history
        self.render_chat()

    def render_chat(self):
        html = """
        <style>
            .message-box {
                margin-bottom: 12px;
                font-family: 'Segoe UI', sans-serif;
                font-size: 13px;
                line-height: 140%;
            }
            .user-msg {
                color: #55aaff;
                font-weight: bold;
            }
            .assistant-msg {
                color: #ffffff;
            }
            .system-status {
                color: #ffaa00;
                font-style: italic;
            }
        </style>
        """
        for msg in self.messages:
            role = msg["role"]
            content = msg["content"].replace("\n", "<br>")
            if role == "system":
                continue
            elif role == "user":
                html += f'<div class="message-box"><span class="user-msg">Você:</span> <span class="assistant-msg">{content}</span></div>'
            elif role == "assistant":
                html += f'<div class="message-box"><span class="user-msg" style="color: #55ffaa;">Assistente:</span> <span class="assistant-msg">{content}</span></div>'
            elif role == "status":
                html += f'<div class="message-box"><span class="system-status">{content}</span></div>'
                
        self.text_display.setHtml(html)
        # Scroll to bottom
        scrollbar = self.text_display.verticalScrollBar()
        scrollbar.setValue(scrollbar.maximum())

    def send_text_message(self):
        text = self.input_field.text().strip()
        if not text:
            return
        self.input_field.setText("")
        self.start_chat_worker(text_query=text)

    def toggle_voice_response(self):
        if not self.is_recording:
            if self.chat_worker and self.chat_worker.isRunning():
                return
            
            self.is_recording = True
            self.btn_mic.setText("🔴")
            self.btn_mic.setStyleSheet("""
                QPushButton {
                    background-color: rgba(255, 85, 85, 30);
                    border: 1px solid #ff5555;
                    border-radius: 17px;
                    font-size: 14px;
                    color: #ff5555;
                }
            """)
            self.input_field.setEnabled(False)
            self.input_field.setPlaceholderText("Gravando voz... Clique em 🔴 para parar.")
            if self.processor.config_manager.get("mute_on_record", False):
                mute_system_audio()
            self.recorder.start()
        else:
            self.is_recording = False
            self.btn_mic.setText("🎤")
            self.btn_mic.setStyleSheet("""
                QPushButton {
                    background-color: rgba(255, 255, 255, 12);
                    border: 1px solid rgba(255, 255, 255, 20);
                    border-radius: 17px;
                    font-size: 14px;
                    color: #ffffff;
                }
                QPushButton:hover {
                    background-color: rgba(255, 255, 255, 25);
                    border: 1px solid rgba(255, 255, 255, 60);
                }
            """)
            self.input_field.setEnabled(True)
            self.input_field.setPlaceholderText("Escreva uma resposta e aperte Enter...")
            
            audio_path = self.recorder.stop()
            if self.processor.config_manager.get("mute_on_record", False):
                unmute_system_audio()
            if audio_path:
                self.start_chat_worker(audio_path=audio_path)

    def start_chat_worker(self, text_query=None, audio_path=None):
        self.input_field.setEnabled(False)
        self.btn_mic.setEnabled(False)
        
        if text_query:
            self.messages.append({"role": "user", "content": text_query})
            self.messages.append({"role": "status", "content": "Assistente pensando..."})
        else:
            self.messages.append({"role": "status", "content": "Transcrevendo voz e consultando IA..."})
            
        self.render_chat()
        
        self.chat_worker = ChatWorker(self.processor, self.messages, audio_path)
        self.chat_worker.status_changed.connect(self.on_chat_status)
        self.chat_worker.finished.connect(self.on_chat_success)
        self.chat_worker.error.connect(self.on_chat_error)
        self.chat_worker.start()

    @Slot(str)
    def on_chat_status(self, status_msg):
        if self.messages and self.messages[-1]["role"] == "status":
            self.messages[-1]["content"] = status_msg
        else:
            self.messages.append({"role": "status", "content": status_msg})
        self.render_chat()

    @Slot(str, str)
    def on_chat_success(self, transcription, response):
        if self.messages and self.messages[-1]["role"] == "status":
            self.messages.pop()
            
        if transcription:
            self.messages.append({"role": "user", "content": transcription})
            
        self.messages.append({"role": "assistant", "content": response})
        self.render_chat()
        
        self.input_field.setEnabled(True)
        self.input_field.setText("")
        self.btn_mic.setEnabled(True)
        self.input_field.setFocus()
        
    @Slot(str)
    def on_chat_error(self, err_msg):
        if self.messages and self.messages[-1]["role"] == "status":
            self.messages.pop()
            
        self.messages.append({"role": "status", "content": f"Erro: {err_msg}"})
        self.render_chat()
        
        self.input_field.setEnabled(True)
        self.btn_mic.setEnabled(True)
        self.input_field.setFocus()

    # Mouse Dragging logic to move card around the screen
    def mousePressEvent(self, event):
        if event.button() == Qt.LeftButton and event.position().y() < 40:
            self.drag_position = event.globalPosition().toPoint() - self.frameGeometry().topLeft()
            event.accept()
        else:
            self.drag_position = None
            super().mousePressEvent(event)

    def mouseMoveEvent(self, event):
        if event.buttons() == Qt.LeftButton and self.drag_position is not None:
            self.move(event.globalPosition().toPoint() - self.drag_position)
            event.accept()
        else:
            super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event):
        self.drag_position = None
        super().mouseReleaseEvent(event)

    def keyPressEvent(self, event):
        if event.key() in (Qt.Key_Return, Qt.Key_Enter):
            event.accept()
            return
        super().keyPressEvent(event)

    def showEvent(self, event):
        # Center card on screen and play slide/fade in animation
        screen = QApplication.primaryScreen().geometry()
        x = (screen.width() - self.width()) // 2
        y = (screen.height() - self.height()) // 2
        self.setGeometry(x, y + 15, self.width(), self.height())
        self.setWindowOpacity(0.0)
        
        self.pos_anim = QPropertyAnimation(self, b"geometry")
        self.pos_anim.setDuration(220)
        self.pos_anim.setStartValue(QRect(x, y + 15, self.width(), self.height()))
        self.pos_anim.setEndValue(QRect(x, y, self.width(), self.height()))
        self.pos_anim.setEasingCurve(QEasingCurve.OutCubic)
        
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(220)
        self.opacity_anim.setStartValue(0.0)
        self.opacity_anim.setEndValue(1.0)
        
        self.anim_group = QParallelAnimationGroup()
        self.anim_group.addAnimation(self.pos_anim)
        self.anim_group.addAnimation(self.opacity_anim)
        self.anim_group.start()
        super().showEvent(event)

    def fade_out_and_close(self):
        if self.is_recording:
            self.is_recording = False
            try:
                self.recorder.stop()
            except Exception:
                pass
            if self.processor.config_manager.get("mute_on_record", False):
                unmute_system_audio()

        geom = self.geometry()
        target_geom = QRect(geom.x(), geom.y() + 10, geom.width(), geom.height())
        
        self.pos_anim = QPropertyAnimation(self, b"geometry")
        self.pos_anim.setDuration(180)
        self.pos_anim.setStartValue(geom)
        self.pos_anim.setEndValue(target_geom)
        self.pos_anim.setEasingCurve(QEasingCurve.InCubic)
        
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(180)
        self.opacity_anim.setStartValue(self.windowOpacity())
        self.opacity_anim.setEndValue(0.0)
        
        self.anim_group = QParallelAnimationGroup()
        self.anim_group.addAnimation(self.pos_anim)
        self.anim_group.addAnimation(self.opacity_anim)
        self.anim_group.finished.connect(self.accept)
        self.anim_group.start()

    def copy_answer(self):
        lines = []
        for msg in self.messages:
            role = msg["role"]
            if role == "user":
                lines.append(f"Você: {msg['content']}")
            elif role == "assistant":
                lines.append(f"Assistente: {msg['content']}")
        
        chat_text = "\n\n".join(lines)
        clipboard = QApplication.clipboard()
        clipboard.setText(chat_text)
        
        sender = self.sender()
        if sender:
            sender.setText("Copiado!")
            QTimer.singleShot(1500, lambda: sender.setText("Copiar Resposta"))

# Settings/Wizard Dialog
# Custom QLineEdit to capture hotkey physically
class HotkeyLineEdit(QLineEdit):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.is_recording = False
        self.original_val = ""

    def start_recording(self, original_val, on_stop_cb=None):
        self.original_val = original_val
        self.is_recording = True
        self.on_stop_cb = on_stop_cb
        self.setFocus()
        self.setText("")
        self.setPlaceholderText(tr("hotkey_prompt"))
        self.setStyleSheet("""
            QLineEdit {
                border: 1px solid #ff5555;
                background-color: rgba(255, 85, 85, 15);
                color: #ffffff;
            }
        """)

    def stop_recording(self):
        self.is_recording = False
        self.setPlaceholderText("")
        self.setStyleSheet("")
        if hasattr(self, 'on_stop_cb') and self.on_stop_cb:
            self.on_stop_cb()

    def keyPressEvent(self, event):
        if not self.is_recording:
            super().keyPressEvent(event)
            return
            
        event.accept()
        key = event.key()
        
        if key == Qt.Key_Escape:
            self.setText(self.original_val)
            self.stop_recording()
            return
            
        modifiers = event.modifiers()
        parts = []
        if modifiers & Qt.ControlModifier:
            parts.append("<ctrl>")
        if modifiers & Qt.ShiftModifier:
            parts.append("<shift>")
        if modifiers & Qt.AltModifier:
            parts.append("<alt>")
        if modifiers & Qt.MetaModifier:
            parts.append("<win>")
            
        if key == Qt.Key_Control and "<ctrl>" not in parts:
            parts.append("<ctrl>")
        elif key == Qt.Key_Shift and "<shift>" not in parts:
            parts.append("<shift>")
        elif key == Qt.Key_Alt and "<alt>" not in parts:
            parts.append("<alt>")
        elif key == Qt.Key_Meta and "<win>" not in parts:
            parts.append("<win>")
            
        is_mod = key in (Qt.Key_Control, Qt.Key_Shift, Qt.Key_Alt, Qt.Key_Meta)
        
        key_name = ""
        if not is_mod:
            if key == Qt.Key_Space:
                key_name = "<space>"
            elif key in (Qt.Key_Enter, Qt.Key_Return):
                key_name = "<enter>"
            elif key == Qt.Key_Tab:
                key_name = "<tab>"
            elif Qt.Key_F1 <= key <= Qt.Key_F12:
                key_name = f"f{key - Qt.Key_F1 + 1}"
            else:
                try:
                    key_name = chr(key).lower()
                except ValueError:
                    key_name = ""
                    
            if key_name:
                parts.append(key_name)
                
        hotkey_str = "+".join(parts)
        self.setText(hotkey_str)
        
        if not is_mod and key_name:
            self.stop_recording()

def _autostart_desktop_path():
    xdg_config = os.getenv('XDG_CONFIG_HOME') or os.path.join(os.path.expanduser('~'), '.config')
    autostart_dir = os.path.join(xdg_config, 'autostart')
    os.makedirs(autostart_dir, exist_ok=True)
    return os.path.join(autostart_dir, 'flowvoice.desktop')

def set_run_at_startup(enabled=True):
    if not getattr(sys, 'frozen', False):
        return

    if sys.platform == 'win32':
        import winreg
        key_path = r"Software\Microsoft\Windows\CurrentVersion\Run"
        app_name = "FlowVoice"
        exe_path = sys.executable
        try:
            key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path, 0, winreg.KEY_SET_VALUE)
            if enabled:
                winreg.SetValueEx(key, app_name, 0, winreg.REG_SZ, f'"{exe_path}"')
            else:
                try:
                    winreg.DeleteValue(key, app_name)
                except FileNotFoundError:
                    pass
            winreg.CloseKey(key)
        except Exception as e:
            print(f"Erro ao configurar inicialização com Windows: {e}")
        return

    if sys.platform == 'linux':
        desktop_path = _autostart_desktop_path()
        try:
            if enabled:
                exe_path = sys.executable
                desktop_content = (
                    "[Desktop Entry]\n"
                    "Type=Application\n"
                    "Name=FlowVoice\n"
                    f'Exec="{exe_path}"\n'
                    "Hidden=false\n"
                    "NoDisplay=false\n"
                    "X-GNOME-Autostart-enabled=true\n"
                    "Comment=FlowVoice - Ditado Inteligente por IA\n"
                )
                with open(desktop_path, "w", encoding="utf-8") as f:
                    f.write(desktop_content)
            elif os.path.exists(desktop_path):
                os.remove(desktop_path)
        except Exception as e:
            print(f"Erro ao configurar inicialização com o sistema: {e}")

def is_run_at_startup_enabled():
    if not getattr(sys, 'frozen', False):
        return False

    if sys.platform == 'win32':
        import winreg
        key_path = r"Software\Microsoft\Windows\CurrentVersion\Run"
        app_name = "FlowVoice"
        try:
            key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path, 0, winreg.KEY_READ)
            try:
                value, _ = winreg.QueryValueEx(key, app_name)
                winreg.CloseKey(key)
                return True
            except FileNotFoundError:
                winreg.CloseKey(key)
                return False
        except Exception:
            return False

    if sys.platform == 'linux':
        return os.path.exists(_autostart_desktop_path())

    return False

class WizardPrimaryBtn(QPushButton):
    """Animated primary button for the wizard nav."""
    def __init__(self, text, parent=None):
        super().__init__(text, parent)
        self.setFixedHeight(36)
        self.setCursor(Qt.PointingHandCursor)
        self._base_style = """
            QPushButton {{
                background: {bg};
                border: none; border-radius: 8px;
                color: #ffffff; font-size: 12px; font-weight: 700;
                font-family: 'Segoe UI', sans-serif; padding: 0 22px;
            }}
        """
        self._set_bg("#8b5cf6")

    def _set_bg(self, color):
        self.setStyleSheet(self._base_style.format(bg=color))

    def enterEvent(self, e):
        super().enterEvent(e)
        self._set_bg("#7c3aed")
        eff = QGraphicsDropShadowEffect(self)
        eff.setBlurRadius(18); eff.setColor(QColor(139,92,246,120)); eff.setOffset(0,3)
        self.setGraphicsEffect(eff)

    def leaveEvent(self, e):
        super().leaveEvent(e)
        self._set_bg("#8b5cf6")
        self.setGraphicsEffect(None)

    def mousePressEvent(self, e):
        super().mousePressEvent(e)
        self._set_bg("#6d28d9")

    def mouseReleaseEvent(self, e):
        super().mouseReleaseEvent(e)
        self._set_bg("#7c3aed")


class WizardSecondaryBtn(QPushButton):
    """Animated secondary (back) button for the wizard nav."""
    def __init__(self, text, parent=None):
        super().__init__(text, parent)
        self.setFixedHeight(36)
        self.setCursor(Qt.PointingHandCursor)
        self._idle = "QPushButton { background: transparent; border: 1px solid rgba(255,255,255,22); border-radius: 8px; color: rgba(255,255,255,140); font-size: 12px; font-weight: 600; font-family: 'Segoe UI', sans-serif; padding: 0 18px; }"
        self._hover = "QPushButton { background: rgba(255,255,255,10); border: 1px solid rgba(255,255,255,55); border-radius: 8px; color: #ffffff; font-size: 12px; font-weight: 600; font-family: 'Segoe UI', sans-serif; padding: 0 18px; }"
        self.setStyleSheet(self._idle)

    def enterEvent(self, e):
        super().enterEvent(e); self.setStyleSheet(self._hover)

    def leaveEvent(self, e):
        super().leaveEvent(e); self.setStyleSheet(self._idle)

    def mousePressEvent(self, e):
        super().mousePressEvent(e)
        self.setStyleSheet("QPushButton { background: rgba(255,255,255,18); border: 1px solid rgba(255,255,255,70); border-radius: 8px; color: #ffffff; font-size: 12px; font-weight: 600; font-family: 'Segoe UI', sans-serif; padding: 0 18px; }")

    def mouseReleaseEvent(self, e):
        super().mouseReleaseEvent(e); self.setStyleSheet(self._hover)


# Settings/Wizard Dialog
class SetupWizard(QDialog):
    """4-step first-run wizard: Welcome → Provider → Style → Hotkey."""

    def __init__(self, config_manager, app=None, parent=None, allow_close=False):
        super().__init__(parent)
        self.config_manager = config_manager
        self.app = app
        self.drag_position = None
        self._step = 0
        self._allow_close = allow_close
        self.setWindowFlags(Qt.FramelessWindowHint | Qt.Dialog)
        self.setAttribute(Qt.WA_TranslucentBackground, True)
        self.setFixedSize(740, 590)
        self._init_ui()

    def _init_ui(self):
        self.container = QFrame(self)
        self.container.setGeometry(10, 10, 720, 570)
        self.container.setStyleSheet("""
            QFrame {
                background-color: #0d0d0d;
                border: 1px solid rgba(255,255,255,25);
                border-radius: 16px;
            }
        """)
        shadow = QGraphicsDropShadowEffect(self)
        shadow.setBlurRadius(30)
        shadow.setColor(QColor(0, 0, 0, 200))
        shadow.setOffset(0, 6)
        self.container.setGraphicsEffect(shadow)

        root = QVBoxLayout(self.container)
        root.setContentsMargins(0, 0, 0, 0)
        root.setSpacing(0)

        # Progress bar (steps) — inside the rounded container, with own margins
        prog_wrap = QFrame()
        prog_wrap.setFixedHeight(14)
        prog_wrap.setStyleSheet("QFrame { background: transparent; border: none; }")
        prog_wrap_l = QHBoxLayout(prog_wrap)
        prog_wrap_l.setContentsMargins(16, 5, 16, 0)
        prog_wrap_l.setSpacing(0)

        self._progress_bar = QFrame()
        self._progress_bar.setFixedHeight(4)
        self._progress_bar.setStyleSheet("""
            QFrame { background: rgba(139,92,246,30); border-radius: 2px; border: none; }
        """)
        prog_wrap_l.addWidget(self._progress_bar)
        root.addWidget(prog_wrap)

        self._progress_fill = QFrame(self._progress_bar)
        self._progress_fill.setFixedHeight(4)
        self._progress_fill.setStyleSheet("""
            QFrame { background: qlineargradient(x1:0,y1:0,x2:1,y2:0, stop:0 #8b5cf6, stop:1 #ec4899); border-radius: 2px; border: none; }
        """)
        self._progress_fill.setGeometry(0, 0, 0, 4)

        # Title bar row with close button
        title_bar = QFrame()
        title_bar.setFixedHeight(44)
        title_bar.setStyleSheet("QFrame { background: transparent; border: none; }")
        tb_l = QHBoxLayout(title_bar)
        tb_l.setContentsMargins(20, 0, 12, 0)
        tb_l.setSpacing(0)
        tb_l.addStretch()
        btn_close = QPushButton("✕")
        btn_close.setFixedSize(28, 28)
        btn_close.setCursor(Qt.PointingHandCursor if self._allow_close else Qt.ForbiddenCursor)
        btn_close.setToolTip("Fechar" if self._allow_close else "Configure ao menos um provedor para continuar")
        if self._allow_close:
            btn_close.setStyleSheet("""
                QPushButton { background: transparent; border: none; color: rgba(255,255,255,80); font-size: 13px; font-weight: bold; border-radius: 6px; }
                QPushButton:hover { background: rgba(255,85,85,30); color: #ff5555; }
            """)
            btn_close.clicked.connect(lambda: self.fade_out_and_close(False))
        else:
            btn_close.setStyleSheet("""
                QPushButton { background: transparent; border: none; color: rgba(255,255,255,20); font-size: 13px; font-weight: bold; border-radius: 6px; }
            """)
        tb_l.addWidget(btn_close)
        root.addWidget(title_bar)

        # Steps stack
        self.stack = QStackedWidget()
        self.stack.setStyleSheet("background: transparent; border: none;")
        root.addWidget(self.stack, 1)

        self._build_step_welcome()
        self._build_step_provider()
        self._build_step_style()
        self._build_step_hotkey()

        # Bottom nav
        nav = QFrame()
        nav.setFixedHeight(64)
        nav.setStyleSheet("""
            QFrame {
                background: rgba(255,255,255,3);
                border-top: 1px solid rgba(255,255,255,10);
                border-bottom-left-radius: 16px;
                border-bottom-right-radius: 16px;
            }
        """)
        nav_l = QHBoxLayout(nav)
        nav_l.setContentsMargins(24, 0, 24, 0)
        nav_l.setSpacing(12)

        self.step_lbl = QLabel(tr("wiz_step", 1, 4))
        self.step_lbl.setStyleSheet("color: rgba(255,255,255,60); font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        nav_l.addWidget(self.step_lbl)
        nav_l.addStretch()

        self.btn_back = WizardSecondaryBtn(tr("wiz_btn_back"))
        self.btn_back.setVisible(False)
        self.btn_back.clicked.connect(self._go_back)

        self.btn_next = WizardPrimaryBtn(tr("wiz_btn_next"))
        self.btn_next.clicked.connect(self._go_next)

        nav_l.addWidget(self.btn_back)
        nav_l.addWidget(self.btn_next)
        root.addWidget(nav)

        self._update_progress()

    def _step_frame(self, title, subtitle, emoji=""):
        w = QWidget()
        w.setStyleSheet("background: transparent;")
        l = QVBoxLayout(w)
        l.setContentsMargins(32, 10, 32, 8)
        l.setSpacing(0)
        em = QLabel(emoji)
        em.setAlignment(Qt.AlignCenter)
        em.setStyleSheet("font-size: 32px; background: transparent; border: none; margin-bottom: 6px;")
        t = QLabel(title)
        t.setAlignment(Qt.AlignCenter)
        t.setStyleSheet("color: #ffffff; font-size: 19px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        s = QLabel(subtitle)
        s.setAlignment(Qt.AlignCenter)
        s.setWordWrap(True)
        s.setStyleSheet("color: rgba(255,255,255,140); font-size: 12px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none; margin-top: 6px; margin-bottom: 14px;")
        l.addWidget(em)
        l.addWidget(t)
        l.addWidget(s)
        return w, l

    def _build_step_welcome(self):
        w, l = self._step_frame(
            tr("wiz_welcome_title"),
            tr("wiz_welcome_desc"),
            ""
        )
        
        # Language Selector
        lang_wrap = QFrame()
        lang_wrap.setStyleSheet("QFrame { background: rgba(255,255,255,4); border: 1px solid rgba(255,255,255,12); border-radius: 10px; }")
        lang_l = QHBoxLayout(lang_wrap)
        lang_l.setContentsMargins(16, 14, 16, 14)
        lang_l.setSpacing(12)
        
        icon_globe = QLabel("🌍")
        icon_globe.setStyleSheet("font-size: 24px; background: transparent; border: none;")
        
        txt_l = QVBoxLayout()
        txt_l.setSpacing(2)
        lang_lbl = QLabel(tr("wiz_welcome_lang"))
        lang_lbl.setStyleSheet("color: rgba(255,255,255,200); font-size: 13px; font-weight: 600; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        lang_desc = QLabel("Selecione seu idioma / Choose your language")
        lang_desc.setStyleSheet("color: rgba(255,255,255,120); font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        txt_l.addWidget(lang_lbl)
        txt_l.addWidget(lang_desc)
        
        self.combo_lang = QComboBox()
        self.combo_lang.setCursor(Qt.PointingHandCursor)
        self.combo_lang.setFixedWidth(160)
        self.combo_lang.addItems(["Português (Brasil)", "English (US)", "Español"])
        cur = get_language()
        if cur == "en": self.combo_lang.setCurrentIndex(1)
        elif cur == "es": self.combo_lang.setCurrentIndex(2)
        else: self.combo_lang.setCurrentIndex(0)
        
        # Add SVGs to combo box items
        self.combo_lang.setItemIcon(0, QIcon(get_resource_path("icons/flag_pt.svg")))
        self.combo_lang.setItemIcon(1, QIcon(get_resource_path("icons/flag_en.svg")))
        self.combo_lang.setItemIcon(2, QIcon(get_resource_path("icons/flag_es.svg")))
        
        self.combo_lang.setStyleSheet("""
            QComboBox { background: rgba(255,255,255,10); border: 1px solid rgba(255,255,255,25); border-radius: 6px; color: white; padding: 6px 12px; font-size: 12px; font-weight: 600; }
            QComboBox::drop-down { border: none; }
            QComboBox QAbstractItemView { background: #1a1a1a; color: white; selection-background-color: #8b5cf6; outline: none; }
        """)
        self.combo_lang.currentIndexChanged.connect(self._on_lang_changed)
        
        lang_l.addWidget(icon_globe)
        lang_l.addLayout(txt_l, 1)
        lang_l.addWidget(self.combo_lang)
        l.addWidget(lang_wrap)
        l.addSpacing(6)

        info = QFrame()
        info.setStyleSheet("QFrame { background: rgba(139,92,246,12); border: 1px solid rgba(139,92,246,40); border-radius: 10px; }")
        info_l = QVBoxLayout(info)
        info_l.setContentsMargins(20, 14, 20, 14)
        info_l.setSpacing(8)
        for icon, txt in [
            ("⚡", tr("wiz_welcome_feat1")),
            ("✍️", tr("wiz_welcome_feat2")),
            ("🔒", tr("wiz_welcome_feat3")),
        ]:
            row = QHBoxLayout()
            row.setSpacing(10)
            li = QLabel(icon)
            li.setFixedWidth(20)
            li.setStyleSheet("font-size: 14px; background: transparent; border: none;")
            lt = QLabel(txt)
            lt.setStyleSheet("color: rgba(255,255,255,180); font-size: 12px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            row.addWidget(li)
            row.addWidget(lt, 1)
            info_l.addLayout(row)
        l.addWidget(info)
        l.addStretch()
        self.stack.addWidget(w)

    def _build_step_provider(self):
        w, l = self._step_frame(
            tr("wiz_provider_title"),
            tr("wiz_provider_desc"),
            ""
        )

        def _svg_pixmap(svg_path, size=20):
            pm = QPixmap(size, size)
            pm.fill(Qt.transparent)
            try:
                from PySide6.QtSvg import QSvgRenderer
                from PySide6.QtCore import QRectF
                renderer = QSvgRenderer(svg_path)
                if renderer.isValid():
                    painter = QPainter(pm)
                    painter.setRenderHint(QPainter.RenderHint.Antialiasing)
                    renderer.render(painter, QRectF(0, 0, size, size))
                    painter.end()
            except Exception as e:
                print(f"SVG render error {svg_path}: {e}")
            return pm

        _provider_meta = {
            "github_models": {"label": "GitHub Models", "icon": get_resource_path("icons/github.svg"), "color": "#6e40c9", "badge_bg": "#1a1a2e", "desc": tr("wiz_prov_desc_github")},
            "groq":          {"label": "Groq",          "icon": get_resource_path("icons/groq.svg"),   "color": "#f59e0b", "badge_bg": "#1a1a1a", "desc": tr("wiz_prov_desc_groq")},
            "gemini":        {"label": "Google Gemini", "icon": get_resource_path("icons/gemini.svg"), "color": "#34d399", "badge_bg": "#0d1a15", "desc": tr("wiz_prov_desc_gemini")},
            "openai":        {"label": "OpenAI",        "icon": get_resource_path("icons/openai.svg"), "color": "#60a5fa", "badge_bg": "#0d0d0d", "desc": tr("wiz_prov_desc_openai")},
        }
        current_provider = self.config_manager.get("provider", "groq")
        self._wiz_selected_provider = current_provider
        self._wiz_provider_cards = {}

        cards_layout = QVBoxLayout()
        cards_layout.setSpacing(6)

        for pid, meta in _provider_meta.items():
            card = QFrame()
            card.setCursor(Qt.PointingHandCursor)
            is_active = pid == current_provider
            card.setStyleSheet(f"""
                QFrame {{
                    background: {'rgba(139,92,246,18)' if is_active else 'rgba(255,255,255,4)'};
                    border: {'2px solid ' + meta['color'] if is_active else '1px solid rgba(255,255,255,12)'};
                    border-radius: 9px;
                }}
            """)
            cl = QHBoxLayout(card)
            cl.setContentsMargins(14, 10, 14, 10)
            cl.setSpacing(12)

            # Icon badge: dark rounded square with colored border + SVG logo inside
            icon_badge = QFrame()
            icon_badge.setFixedSize(38, 38)
            icon_badge.setStyleSheet(f"QFrame {{ background: {meta['badge_bg']}; border-radius: 10px; border: 1px solid rgba(255,255,255,15); }}")
            badge_l = QVBoxLayout(icon_badge)
            badge_l.setContentsMargins(0, 0, 0, 0)
            badge_l.setAlignment(Qt.AlignCenter)
            icon_lbl = QLabel()
            icon_lbl.setFixedSize(38, 38)
            icon_lbl.setAlignment(Qt.AlignCenter)
            pm = _svg_pixmap(meta["icon"], 22)
            icon_lbl.setPixmap(pm)
            icon_lbl.setStyleSheet("background: transparent; border: none;")
            badge_l.addWidget(icon_lbl)

            txt = QVBoxLayout()
            txt.setSpacing(2)
            lbl_name = QLabel(meta["label"])
            lbl_name.setStyleSheet(f"color: {'#ffffff' if is_active else 'rgba(255,255,255,210)'}; font-size: 13px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            lbl_desc = QLabel(meta["desc"])
            lbl_desc.setWordWrap(False)
            lbl_desc.setStyleSheet("color: rgba(255,255,255,90); font-size: 10px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            txt.addWidget(lbl_name)
            txt.addWidget(lbl_desc)
            dot = QLabel("●" if is_active else "○")
            dot.setFixedWidth(18)
            dot.setAlignment(Qt.AlignCenter)
            dot.setStyleSheet(f"color: {meta['color']}; font-size: 13px; background: transparent; border: none;")
            cl.addWidget(icon_badge)
            cl.addLayout(txt, 1)
            cl.addWidget(dot, 0)
            self._wiz_provider_cards[pid] = (card, dot, lbl_name, meta["color"])
            # Hover effect
            _idle_style = card.styleSheet()
            _active_color = meta["color"]
            def _make_hover(c, idle, col, p):
                def _enter(e):
                    if self._wiz_selected_provider != p:
                        c.setStyleSheet(f"QFrame {{ background: rgba(255,255,255,8); border: 1px solid rgba(255,255,255,28); border-radius: 9px; }}")
                def _leave(e):
                    if self._wiz_selected_provider != p:
                        c.setStyleSheet(idle)
                c.enterEvent = _enter
                c.leaveEvent = _leave
            _make_hover(card, _idle_style, _active_color, pid)
            card.mousePressEvent = lambda e, p=pid: self._wiz_select_provider(p)
            cards_layout.addWidget(card)

        l.addLayout(cards_layout)
        l.addSpacing(12)

        # Key input with show/hide toggle
        key_row = QHBoxLayout()
        key_row.setSpacing(6)
        self.wiz_txt_key = QLineEdit()
        self.wiz_txt_key.setEchoMode(QLineEdit.Password)
        self.wiz_txt_key.setText(self.config_manager.get_api_key(current_provider))
        self.wiz_txt_key.setPlaceholderText(tr("wiz_key_placeholder"))
        self.wiz_txt_key.setFixedHeight(38)
        self.wiz_txt_key.setStyleSheet("""
            QLineEdit { background: rgba(255,255,255,8); border: 1px solid rgba(255,255,255,20); border-radius: 8px; padding: 6px 12px; color: #fff; font-size: 12px; font-family: 'Segoe UI', sans-serif; }
            QLineEdit:hover { border-color: rgba(139,92,246,120); }
            QLineEdit:focus { border: 1px solid #8b5cf6; background: rgba(255,255,255,12); }
        """)
        btn_toggle_key = QPushButton("👁")
        btn_toggle_key.setFixedSize(38, 38)
        btn_toggle_key.setCursor(Qt.PointingHandCursor)
        btn_toggle_key.setCheckable(True)
        btn_toggle_key.setStyleSheet("""
            QPushButton { background: rgba(255,255,255,8); border: 1px solid rgba(255,255,255,20); border-radius: 8px; color: rgba(255,255,255,160); font-size: 14px; }
            QPushButton:hover { background: rgba(255,255,255,15); border-color: rgba(139,92,246,120); }
            QPushButton:checked { background: rgba(139,92,246,25); border-color: #8b5cf6; }
        """)
        def _wiz_toggle_key(checked):
            if checked:
                pwd = self.config_manager.get("keys_password", "")
                if pwd:
                    # Show inline password prompt
                    dlg = QDialog(self)
                    dlg.setWindowFlags(Qt.FramelessWindowHint | Qt.Dialog)
                    dlg.setAttribute(Qt.WA_TranslucentBackground, True)
                    dlg.setFixedSize(340, 160)
                    outer = QFrame(dlg)
                    outer.setGeometry(8, 8, 324, 144)
                    outer.setStyleSheet("QFrame { background: #141414; border: 1px solid rgba(139,92,246,80); border-radius: 12px; }")
                    vl = QVBoxLayout(outer)
                    vl.setContentsMargins(20, 16, 20, 16)
                    vl.setSpacing(10)
                    lbl = QLabel(tr("pwd_prompt_reveal"))
                    lbl.setStyleSheet("color: rgba(255,255,255,200); font-size: 12px; font-weight: 600; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
                    pwd_input = QLineEdit()
                    pwd_input.setEchoMode(QLineEdit.Password)
                    pwd_input.setPlaceholderText(tr("pwd_placeholder"))
                    pwd_input.setFixedHeight(34)
                    pwd_input.setStyleSheet("QLineEdit { background: rgba(255,255,255,8); border: 1px solid rgba(255,255,255,20); border-radius: 7px; padding: 4px 10px; color: #fff; font-size: 12px; font-family: 'Segoe UI', sans-serif; } QLineEdit:focus { border-color: #8b5cf6; }")
                    err_lbl = QLabel("")
                    err_lbl.setStyleSheet("color: #f87171; font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
                    btn_row = QHBoxLayout()
                    btn_row.setSpacing(8)
                    b_cancel = QPushButton(tr("btn_cancel"))
                    b_cancel.setFixedHeight(30)
                    b_cancel.setStyleSheet("QPushButton { background: transparent; border: 1px solid rgba(255,255,255,20); border-radius: 6px; color: rgba(255,255,255,140); font-size: 11px; font-family: 'Segoe UI', sans-serif; padding: 0 12px; } QPushButton:hover { background: rgba(255,255,255,10); }")
                    b_ok = QPushButton(tr("btn_confirm"))
                    b_ok.setFixedHeight(30)
                    b_ok.setStyleSheet("QPushButton { background: #8b5cf6; border: none; border-radius: 6px; color: #fff; font-size: 11px; font-weight: 700; font-family: 'Segoe UI', sans-serif; padding: 0 12px; } QPushButton:hover { background: #7c3aed; }")
                    btn_row.addWidget(b_cancel); btn_row.addWidget(b_ok)
                    vl.addWidget(lbl); vl.addWidget(pwd_input); vl.addWidget(err_lbl); vl.addLayout(btn_row)
                    def _confirm():
                        if pwd_input.text() == pwd:
                            dlg.accept()
                        else:
                            err_lbl.setText(tr("pwd_error"))
                            pwd_input.clear()
                    b_ok.clicked.connect(_confirm)
                    b_cancel.clicked.connect(dlg.reject)
                    pwd_input.returnPressed.connect(_confirm)
                    if dlg.exec() != QDialog.Accepted:
                        btn_toggle_key.setChecked(False)
                        return
                self.wiz_txt_key.setEchoMode(QLineEdit.Normal)
            else:
                self.wiz_txt_key.setEchoMode(QLineEdit.Password)
        btn_toggle_key.toggled.connect(_wiz_toggle_key)
        key_row.addWidget(self.wiz_txt_key, 1)
        key_row.addWidget(btn_toggle_key)
        l.addLayout(key_row)

        lbl_link = QLabel(
            tr("wiz_get_keys") +
            "<a href='https://console.groq.com/keys' style='color:#8b5cf6;'>Groq</a>  •  "
            "<a href='https://github.com/marketplace/models' style='color:#8b5cf6;'>GitHub Models</a>  •  "
            "<a href='https://aistudio.google.com' style='color:#8b5cf6;'>Gemini</a>"
        )
        lbl_link.setOpenExternalLinks(True)
        lbl_link.setStyleSheet("color: rgba(255,255,255,80); font-size: 10px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none; margin-top: 4px;")
        lbl_link.setWordWrap(True)
        l.addWidget(lbl_link)
        l.addStretch()
        self.stack.addWidget(w)

    def _wiz_select_provider(self, pid):
        self._wiz_selected_provider = pid
        _colors = {
            "github_models": "#6e40c9", "groq": "#f59e0b",
            "gemini": "#34d399", "openai": "#60a5fa",
        }
        for p, (card, dot, lbl_name, color) in self._wiz_provider_cards.items():
            active = p == pid
            card.setStyleSheet(f"""
                QFrame {{
                    background: {'rgba(139,92,246,18)' if active else 'rgba(255,255,255,4)'};
                    border: {'2px solid ' + color if active else '1px solid rgba(255,255,255,12)'};
                    border-radius: 9px;
                }}
            """)
            dot.setText("●" if active else "○")
            dot.setStyleSheet(f"color: {color}; font-size: 13px; background: transparent; border: none;")
            lbl_name.setStyleSheet(f"color: {'#ffffff' if active else 'rgba(255,255,255,200)'}; font-size: 12px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        self.wiz_txt_key.setText(self.config_manager.get_api_key(pid))

    def _build_step_style(self):
        w, l = self._step_frame(
            tr("wiz_style_title"),
            tr("wiz_style_desc"),
            ""
        )
        style_data = [
            ("Profissional", tr("wiz_style_prof_title"), tr("wiz_style_prof_desc"), "#8b5cf6"),
            ("Casual", tr("wiz_style_casual_title"), tr("wiz_style_casual_desc"), "#06b6d4"),
            ("Direto", tr("wiz_style_raw_title"), tr("wiz_style_raw_desc"), "#34d399"),
        ]
        self._style_btns = {}
        current_style = self.config_manager.get("active_style", "Profissional")
        for style_key, style_name, desc, color in style_data:
            card = QFrame()
            card.setObjectName(f"style_card_{style_key}")
            is_active = style_key == current_style
            card.setStyleSheet(f"""
                QFrame {{
                    background: {'rgba(139,92,246,18)' if is_active else 'rgba(255,255,255,4)'};
                    border: {'2px solid ' + color if is_active else '1px solid rgba(255,255,255,12)'};
                    border-radius: 10px;
                }}
                QFrame:hover {{ background: rgba(255,255,255,8); border-color: {color}; }}
            """)
            card.setCursor(Qt.PointingHandCursor)
            card_l = QHBoxLayout(card)
            card_l.setContentsMargins(16, 12, 16, 12)
            card_l.setSpacing(12)
            txt_col = QVBoxLayout()
            txt_col.setSpacing(3)
            t = QLabel(style_name)
            t.setStyleSheet(f"color: {'#ffffff' if is_active else 'rgba(255,255,255,200)'}; font-size: 13px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            d = QLabel(desc)
            d.setStyleSheet("color: rgba(255,255,255,120); font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            d.setWordWrap(True)
            txt_col.addWidget(t)
            txt_col.addWidget(d)
            card_l.addLayout(txt_col, 1)
            dot = QLabel("●" if is_active else "○")
            dot.setStyleSheet(f"color: {color}; font-size: 14px; background: transparent; border: none;")
            card_l.addWidget(dot)
            self._style_btns[style_key] = (card, dot, t, color)
            _idle_s = card.styleSheet()
            def _make_style_hover(c, col, sn):
                def _enter(e):
                    if getattr(self, '_selected_style', None) != sn:
                        c.setStyleSheet(f"QFrame {{ background: rgba(255,255,255,8); border: 1px solid {col}; border-radius: 10px; }}")
                def _leave(e):
                    if getattr(self, '_selected_style', None) != sn:
                        c.setStyleSheet(f"QFrame {{ background: rgba(255,255,255,4); border: 1px solid rgba(255,255,255,12); border-radius: 10px; }}")
                c.enterEvent = _enter
                c.leaveEvent = _leave
            _make_style_hover(card, color, style_key)
            card.mousePressEvent = lambda e, s=style_key: self._select_style(s)
            l.addWidget(card)
            l.addSpacing(6)
        l.addStretch()
        self.stack.addWidget(w)

    def _select_style(self, style_name):
        for name, (card, dot, title_lbl, color) in self._style_btns.items():
            active = name == style_name
            card.setStyleSheet(f"""
                QFrame {{
                    background: {'rgba(139,92,246,18)' if active else 'rgba(255,255,255,4)'};
                    border: {'2px solid ' + color if active else '1px solid rgba(255,255,255,12)'};
                    border-radius: 10px;
                }}
                QFrame:hover {{ background: rgba(255,255,255,8); border-color: {color}; }}
            """)
            dot.setText("●" if active else "○")
            dot.setStyleSheet(f"color: {color}; font-size: 14px; background: transparent; border: none;")
        self._selected_style = style_name

    def _build_step_hotkey(self):
        w, l = self._step_frame(
            tr("wiz_hotkey_title"),
            tr("wiz_hotkey_desc"),
            ""
        )
        row = QHBoxLayout()
        row.setSpacing(8)
        self.wiz_hotkey_edit = HotkeyLineEdit()
        self.wiz_hotkey_edit.setText(self.config_manager.get("hotkey", "<ctrl>+<shift>+<space>"))
        self.wiz_hotkey_edit.setFixedHeight(38)
        self.wiz_hotkey_edit.setStyleSheet("""
            QLineEdit { background: rgba(255,255,255,8); border: 1px solid rgba(255,255,255,20); border-radius: 8px; padding: 6px 12px; color: #fff; font-size: 12px; font-family: 'Segoe UI', sans-serif; }
            QLineEdit:focus { border: 1px solid #8b5cf6; background: rgba(255,255,255,12); }
        """)
        btn_cap = QPushButton(tr("btn_capture"))
        btn_cap.setFixedHeight(38)
        btn_cap.setFixedWidth(90)
        btn_cap.setCursor(Qt.PointingHandCursor)
        btn_cap.setStyleSheet("""
            QPushButton { background: rgba(255,255,255,10); border: 1px solid rgba(255,255,255,25); border-radius: 8px; color: rgba(255,255,255,200); font-size: 12px; font-weight: 600; font-family: 'Segoe UI', sans-serif; }
            QPushButton:hover { background: rgba(255,255,255,20); border-color: rgba(255,255,255,60); color: #fff; }
        """)
        def _on_capture():
            btn_cap.setText(tr("btn_capturing"))
            btn_cap.setStyleSheet("QPushButton { background: rgba(255,85,85,20); border: 1px solid #ff5555; border-radius: 8px; color: #ff5555; font-size: 12px; font-weight: 600; font-family: 'Segoe UI', sans-serif; }")
            self.wiz_hotkey_edit.start_recording(self.wiz_hotkey_edit.text(), lambda: (btn_cap.setText(tr("btn_capture")), btn_cap.setStyleSheet("")))
        btn_cap.clicked.connect(_on_capture)
        row.addWidget(self.wiz_hotkey_edit, 1)
        row.addWidget(btn_cap)
        l.addLayout(row)
        l.addSpacing(12)
        lbl_done = QLabel(tr("wiz_done"))
        lbl_done.setWordWrap(True)
        lbl_done.setStyleSheet("color: rgba(255,255,255,160); font-size: 12px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        l.addWidget(lbl_done)
        l.addStretch()
        self._selected_style = self.config_manager.get("active_style", "Profissional")
        self.stack.addWidget(w)

    def _update_progress(self):
        total_steps = self.stack.count()
        pct = (self._step + 1) / total_steps
        bar_w = self._progress_bar.width() if self._progress_bar.width() > 0 else 608
        target_w = int(bar_w * pct)
        self._anim_progress = QPropertyAnimation(self._progress_fill, b"geometry")
        self._anim_progress.setDuration(300)
        self._anim_progress.setStartValue(self._progress_fill.geometry())
        self._anim_progress.setEndValue(QRect(0, 0, target_w, 3))
        self._anim_progress.setEasingCurve(QEasingCurve.OutCubic)
        self._anim_progress.start()
        self.step_lbl.setText(tr("wiz_step", self._step + 1, total_steps))
        self.btn_back.setVisible(self._step > 0)
        is_last = self._step == total_steps - 1
        self.btn_next.setText(tr("wiz_btn_finish") if is_last else tr("wiz_btn_next"))

    def _on_lang_changed(self, idx):
        if idx == 0: lang_code = "pt"
        elif idx == 1: lang_code = "en"
        else: lang_code = "es"
        
        set_language(lang_code, self.config_manager)
        
        # Save state and reload wizard
        app_ref = self.app
        config_ref = self.config_manager
        parent_ref = self.parent()
        allow_close_ref = self._allow_close
        step_ref = self._step
        
        self.reject()
        
        def _reopen():
            wiz = SetupWizard(config_ref, app_ref, parent_ref, allow_close_ref)
            wiz._step = step_ref
            wiz.stack.setCurrentIndex(step_ref)
            wiz._update_progress()
            wiz.exec()
            
        QTimer.singleShot(100, _reopen)

    def _animate_step(self, direction=1):
        widget = self.stack.currentWidget()
        try:
            effect = QGraphicsOpacityEffect(widget)
            widget.setGraphicsEffect(effect)
            anim = QPropertyAnimation(effect, b"opacity")
            anim.setDuration(220)
            anim.setStartValue(0.0)
            anim.setEndValue(1.0)
            anim.setEasingCurve(QEasingCurve.OutCubic)
            def _cleanup():
                try:
                    widget.setGraphicsEffect(None)
                except RuntimeError:
                    pass
            anim.finished.connect(_cleanup)
            anim.start()
            self._step_anim = anim
        except RuntimeError:
            pass

    def _go_next(self):
        if self._step == self.stack.count() - 1:
            self._save_wizard()
            return
        self._step += 1
        self.stack.setCurrentIndex(self._step)
        self._update_progress()
        self._animate_step(1)

    def _go_back(self):
        if self._step > 0:
            self._step -= 1
            self.stack.setCurrentIndex(self._step)
            self._update_progress()
            self._animate_step(-1)

    def _save_wizard(self):
        provider = getattr(self, "_wiz_selected_provider", self.config_manager.get("provider", "groq"))
        key = self.wiz_txt_key.text().strip()
        self.config_manager.set("provider", provider)
        if key:
            self.config_manager.set_api_key(provider, key)
        self.config_manager.set("active_style", getattr(self, "_selected_style", "Profissional"))
        hotkey = self.wiz_hotkey_edit.text().strip()
        if hotkey:
            self.config_manager.set("hotkey", hotkey)
        self.config_manager.set("wizard_completed", True)
        self.fade_out_and_close(True)

    def mousePressEvent(self, event):
        if event.button() == Qt.LeftButton and event.position().y() < 48:
            self.drag_position = event.globalPosition().toPoint() - self.frameGeometry().topLeft()
            event.accept()
        else:
            self.drag_position = None
            super().mousePressEvent(event)

    def mouseMoveEvent(self, event):
        if event.buttons() == Qt.LeftButton and self.drag_position is not None:
            self.move(event.globalPosition().toPoint() - self.drag_position)
            event.accept()
        else:
            super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event):
        self.drag_position = None
        super().mouseReleaseEvent(event)

    def showEvent(self, event):
        geom = self.geometry()
        self.setWindowOpacity(0.0)
        self.pos_anim = QPropertyAnimation(self, b"geometry")
        self.pos_anim.setDuration(280)
        self.pos_anim.setStartValue(QRect(geom.x(), geom.y() + 20, geom.width(), geom.height()))
        self.pos_anim.setEndValue(geom)
        self.pos_anim.setEasingCurve(QEasingCurve.OutCubic)
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(280)
        self.opacity_anim.setStartValue(0.0)
        self.opacity_anim.setEndValue(1.0)
        self.anim_group = QParallelAnimationGroup()
        self.anim_group.addAnimation(self.pos_anim)
        self.anim_group.addAnimation(self.opacity_anim)
        self.anim_group.start()
        super().showEvent(event)

    def fade_out_and_close(self, accept_dialog=False):
        geom = self.geometry()
        self.pos_anim = QPropertyAnimation(self, b"geometry")
        self.pos_anim.setDuration(200)
        self.pos_anim.setStartValue(geom)
        self.pos_anim.setEndValue(QRect(geom.x(), geom.y() + 12, geom.width(), geom.height()))
        self.pos_anim.setEasingCurve(QEasingCurve.InCubic)
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(200)
        self.opacity_anim.setStartValue(self.windowOpacity())
        self.opacity_anim.setEndValue(0.0)
        self.anim_group = QParallelAnimationGroup()
        self.anim_group.addAnimation(self.pos_anim)
        self.anim_group.addAnimation(self.opacity_anim)
        if accept_dialog:
            self.anim_group.finished.connect(self.accept)
        else:
            self.anim_group.finished.connect(self.reject)
        self.anim_group.start()


class SidebarButton(QPushButton):
    """Icon + label sidebar nav button with active indicator."""
    def __init__(self, icon, label, parent=None):
        super().__init__(parent)
        self.setCheckable(True)
        self.setFixedHeight(48)
        self.setCursor(Qt.PointingHandCursor)
        self._icon_char = icon
        self._label = label

        layout = QHBoxLayout(self)
        layout.setContentsMargins(16, 0, 14, 0)
        layout.setSpacing(10)

        # Active indicator bar (left edge, positioned as child)
        self._indicator = QFrame(self)
        self._indicator.setFixedSize(3, 20)
        self._indicator.move(0, 14)
        self._indicator.setStyleSheet("background: #8b5cf6; border-radius: 2px;")
        self._indicator.setVisible(False)
        self._indicator.raise_()

        self.icon_lbl = QLabel(icon)
        self.icon_lbl.setFixedWidth(22)
        self.icon_lbl.setAlignment(Qt.AlignCenter)
        self.text_lbl = QLabel(label)
        layout.addWidget(self.icon_lbl)
        layout.addWidget(self.text_lbl, 1)

        self._update_colors(False)

    def enterEvent(self, event):
        super().enterEvent(event)
        if not self.isChecked():
            self.setStyleSheet("""
                SidebarButton { background-color: rgba(255,255,255,9); border: none; border-radius: 8px; }
            """)

    def leaveEvent(self, event):
        super().leaveEvent(event)
        if not self.isChecked():
            self.setStyleSheet("""
                SidebarButton { background-color: transparent; border: none; border-radius: 8px; }
            """)

    def setChecked(self, val):
        super().setChecked(val)
        self._update_colors(val)

    def _update_colors(self, active):
        color = "#ffffff" if active else "rgba(255,255,255,160)"
        icon_size = "16px"
        self.icon_lbl.setStyleSheet(f"font-size: {icon_size}; background: transparent; border: none; color: {color};")
        self.text_lbl.setStyleSheet(f"font-size: 12px; font-weight: 600; font-family: 'Segoe UI', sans-serif; background: transparent; border: none; color: {color};")
        if active:
            self._indicator.setVisible(True)
            self.setStyleSheet("""
                SidebarButton { background-color: rgba(139,92,246,20); border: none; border-radius: 8px; }
            """)
        else:
            self._indicator.setVisible(False)
            self.setStyleSheet("""
                SidebarButton { background-color: transparent; border: none; border-radius: 8px; }
            """)


class SettingsDialog(QDialog):
    def __init__(self, config_manager, app=None, parent=None):
        super().__init__(parent)
        self.config_manager = config_manager
        self.app = app
        self.drag_position = None
        self._current_page = 0
        self._opened = False
        self.init_ui()

    def showEvent(self, event):
        super().showEvent(event)
        if self._opened:
            return
        self._opened = True
        # Animate in: fade + rise from slightly below
        self.setWindowOpacity(0.0)
        geom = self.geometry()
        start = QRect(geom.x(), geom.y() + 18, geom.width(), geom.height())
        self.setGeometry(start)
        self._open_pos_anim = QPropertyAnimation(self, b"geometry")
        self._open_pos_anim.setDuration(320)
        self._open_pos_anim.setStartValue(start)
        self._open_pos_anim.setEndValue(geom)
        self._open_pos_anim.setEasingCurve(QEasingCurve.OutCubic)
        self._open_fade_anim = QPropertyAnimation(self, b"windowOpacity")
        self._open_fade_anim.setDuration(280)
        self._open_fade_anim.setStartValue(0.0)
        self._open_fade_anim.setEndValue(1.0)
        self._open_fade_anim.setEasingCurve(QEasingCurve.OutCubic)
        self._open_anim_group = QParallelAnimationGroup()
        self._open_anim_group.addAnimation(self._open_pos_anim)
        self._open_anim_group.addAnimation(self._open_fade_anim)
        self._open_anim_group.start()

    def _make_scroll_page(self):
        """Returns a QScrollArea wrapping a plain QWidget for use as a sidebar page."""
        page = QWidget()
        page.setStyleSheet("background: transparent;")
        scroll = QScrollArea()
        scroll.setWidget(page)
        scroll.setWidgetResizable(True)
        scroll.setFrameShape(QFrame.NoFrame)
        scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarAlwaysOff)
        scroll.setStyleSheet("""
            QScrollArea { background: transparent; border: none; }
            QWidget { background: transparent; }
            QScrollBar:vertical {
                background: rgba(255,255,255,6); width: 5px; border-radius: 3px; margin: 0;
            }
            QScrollBar::handle:vertical {
                background: rgba(139,92,246,100); border-radius: 3px; min-height: 24px;
            }
            QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical { height: 0px; }
        """)
        return scroll, page

    def _section_label(self, text):
        lbl = QLabel(text)
        lbl.setStyleSheet(
            "color: rgba(139,92,246,220); font-size: 10px; font-weight: 700; "
            "letter-spacing: 1.5px; font-family: 'Segoe UI', sans-serif; "
            "border-bottom: 1px solid rgba(139,92,246,40); padding-bottom: 4px; margin-bottom: 2px;"
        )
        return lbl

    def _desc(self, text):
        lbl = QLabel(text)
        lbl.setStyleSheet(
            "color: rgba(255,255,255,90); font-size: 11px; font-weight: normal; "
            "font-family: 'Segoe UI', sans-serif; margin-top: 3px; margin-bottom: 10px;"
        )
        lbl.setWordWrap(True)
        return lbl

    def _field_label(self, text):
        lbl = QLabel(text)
        lbl.setStyleSheet(
            "color: rgba(255,255,255,200); font-size: 12px; font-weight: 600; "
            "font-family: 'Segoe UI', sans-serif; margin-top: 10px; margin-bottom: 4px;"
        )
        return lbl

    def _combo(self, items, current):
        cb = QComboBox()
        cb.addItems(items)
        cb.setCurrentText(current)
        cb.setCursor(Qt.PointingHandCursor)
        cb.setFocusPolicy(Qt.StrongFocus)
        cb.wheelEvent = lambda e: e.ignore()
        cb.setStyleSheet("""
            QComboBox {
                background-color: rgba(255,255,255,8);
                border: 1px solid rgba(255,255,255,20);
                border-radius: 7px;
                padding: 5px 12px;
                min-height: 32px;
                color: #ffffff;
                font-size: 12px;
                font-family: 'Segoe UI', sans-serif;
                selection-background-color: transparent;
            }
            QComboBox:hover { border: 1px solid rgba(139,92,246,120); }
            QComboBox:focus { border: 1px solid #8b5cf6; background-color: rgba(255,255,255,12); }
            QComboBox::drop-down {
                border: none; width: 28px;
            }
            QComboBox::down-arrow {
                image: none;
                border-left: 4px solid transparent;
                border-right: 4px solid transparent;
                border-top: 5px solid rgba(255,255,255,120);
                margin-right: 10px;
            }
            QComboBox QAbstractItemView {
                background-color: #1a1a2e;
                border: 1px solid rgba(139,92,246,80);
                border-radius: 7px;
                color: #ffffff;
                font-size: 12px;
                font-family: 'Segoe UI', sans-serif;
                padding: 4px;
                selection-background-color: rgba(139,92,246,60);
                selection-color: #ffffff;
                outline: none;
            }
            QComboBox QAbstractItemView::item {
                min-height: 28px;
                padding: 4px 8px;
                border-radius: 4px;
            }
            QComboBox QAbstractItemView::item:hover {
                background-color: rgba(139,92,246,40);
            }
        """)
        return cb

    def _lineedit(self, text="", placeholder="", password=False):
        le = QLineEdit()
        le.setText(text)
        le.setPlaceholderText(placeholder)
        if password:
            le.setEchoMode(QLineEdit.Password)
        _base_style = """
            QLineEdit {
                background-color: rgba(255,255,255,8);
                border: 1px solid rgba(255,255,255,20);
                border-radius: 8px;
                padding: 6px 14px;
                min-height: 34px;
                color: #ffffff;
                font-size: 12px;
                font-family: 'Segoe UI', sans-serif;
            }
            QLineEdit:hover { border: 1px solid rgba(139,92,246,140); background-color: rgba(255,255,255,10); }
            QLineEdit:focus { border: 2px solid #8b5cf6; background-color: rgba(139,92,246,12); padding: 5px 13px; }
            QLineEdit::placeholder { color: rgba(255,255,255,55); }
        """
        le.setStyleSheet(_base_style)
        # Focus glow: animate drop-shadow via graphics effect on focus/blur
        def _on_focus_in():
            eff = QGraphicsDropShadowEffect(le)
            eff.setBlurRadius(0)
            eff.setColor(QColor(139, 92, 246, 0))
            eff.setOffset(0, 0)
            le.setGraphicsEffect(eff)
            anim = QPropertyAnimation(eff, b"blurRadius")
            anim.setDuration(220)
            anim.setStartValue(0)
            anim.setEndValue(14)
            anim.setEasingCurve(QEasingCurve.OutCubic)
            color_anim = QPropertyAnimation(eff, b"color")
            color_anim.setDuration(220)
            color_anim.setStartValue(QColor(139, 92, 246, 0))
            color_anim.setEndValue(QColor(139, 92, 246, 80))
            color_anim.setEasingCurve(QEasingCurve.OutCubic)
            le._glow_group = QParallelAnimationGroup()
            le._glow_group.addAnimation(anim)
            le._glow_group.addAnimation(color_anim)
            le._glow_group.start()
        def _on_focus_out():
            eff = le.graphicsEffect()
            if not eff:
                return
            anim = QPropertyAnimation(eff, b"blurRadius")
            anim.setDuration(180)
            anim.setStartValue(14)
            anim.setEndValue(0)
            anim.setEasingCurve(QEasingCurve.InCubic)
            anim.finished.connect(lambda: le.setGraphicsEffect(None))
            le._glow_out = anim
            anim.start()
        le.focusInEvent = lambda e, orig=le.focusInEvent: (_on_focus_in(), orig(e))
        le.focusOutEvent = lambda e, orig=le.focusOutEvent: (_on_focus_out(), orig(e))
        return le

    def _separator(self):
        sep = QFrame()
        sep.setFixedHeight(1)
        sep.setStyleSheet("background-color: rgba(255,255,255,12); margin: 6px 0;")
        return sep

    def create_hotkey_row(self, label_text, config_key, default_val):
        h_layout = QHBoxLayout()
        h_layout.setSpacing(6)
        h_layout.setContentsMargins(0, 0, 0, 0)

        line_edit = HotkeyLineEdit()
        line_edit.setText(self.config_manager.get(config_key, default_val))
        line_edit.setStyleSheet("""
            QLineEdit {
                background-color: rgba(255,255,255,8);
                border: 1px solid rgba(255,255,255,20);
                border-radius: 7px;
                padding: 5px 12px;
                min-height: 32px;
                color: #ffffff;
                font-size: 12px;
                font-family: 'Segoe UI', sans-serif;
            }
            QLineEdit:focus { border: 1px solid #8b5cf6; background-color: rgba(255,255,255,12); }
        """)

        btn_capture = QPushButton(tr("btn_capture"))
        btn_capture.setObjectName("btn_capture")
        btn_capture.setFixedWidth(80)
        btn_capture.setStyleSheet("""
            QPushButton#btn_capture {
                background-color: rgba(255, 255, 255, 12);
                border: 1px solid rgba(255, 255, 255, 25);
                border-radius: 6px;
                color: rgba(255, 255, 255, 200);
                font-size: 11px;
                font-family: 'Segoe UI', sans-serif;
                font-weight: bold;
                padding: 4px 10px;
                min-height: 28px;
            }
            QPushButton#btn_capture:hover {
                background-color: rgba(255, 255, 255, 25);
                border: 1px solid rgba(255, 255, 255, 60);
                color: #ffffff;
            }
        """)
        
        def on_capture_clicked(le=line_edit, btn=btn_capture):
            if not le.is_recording:
                btn.setText(tr("btn_capturing"))
                btn.setStyleSheet("""
                    QPushButton#btn_capture {
                        background-color: rgba(255, 85, 85, 30);
                        border: 1px solid #ff5555;
                        border-radius: 6px;
                        color: #ff5555;
                        font-size: 11px;
                        font-family: 'Segoe UI', sans-serif;
                        font-weight: bold;
                        padding: 4px 10px;
                        min-height: 28px;
                    }
                """)
                le.start_recording(le.text(), lambda: restore_button(btn))
            else:
                le.stop_recording()
                
        def restore_button(btn):
            btn.setText(tr("btn_capture"))
            btn.setStyleSheet("")
            
        btn_capture.clicked.connect(lambda: on_capture_clicked(line_edit, btn_capture))
        
        h_layout.addWidget(line_edit, 1)
        h_layout.addWidget(btn_capture)
        
        return line_edit, h_layout

    def _switch_page(self, index):
        if index == self._current_page:
            return
        prev_index = self._current_page
        self._current_page = index

        # Update sidebar buttons immediately
        for i, btn in enumerate(self._sidebar_btns):
            btn.setChecked(i == index)
            btn._update_colors(i == index)

        # Animate title swap using stylesheet color (avoids compositing issues)
        new_title = self._page_titles[index]
        _base_style = "font-size: 12px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none; letter-spacing: 0.3px;"
        steps = 6
        self._title_fade_steps = steps
        self._title_fade_i = 0
        self._title_new = new_title

        def _title_step():
            i = self._title_fade_i
            if i <= steps:
                alpha = int(200 * (1.0 - i / steps))
                self.page_title_lbl.setStyleSheet(f"color: rgba(139,92,246,{alpha}); {_base_style}")
                self._title_fade_i += 1
                QTimer.singleShot(12, _title_step)
            else:
                self.page_title_lbl.setText(self._title_new)
                self._title_fade_i = 0
                def _title_in():
                    i2 = self._title_fade_i
                    if i2 <= steps:
                        alpha = int(200 * (i2 / steps))
                        self.page_title_lbl.setStyleSheet(f"color: rgba(139,92,246,{alpha}); {_base_style}")
                        self._title_fade_i += 1
                        QTimer.singleShot(12, _title_in)
                    else:
                        self.page_title_lbl.setStyleSheet(f"color: rgba(139,92,246,200); {_base_style}")
                _title_in()

        _title_step()

        # Stop any prior animation and clean up
        if hasattr(self, '_page_anim_group') and self._page_anim_group is not None:
            try:
                if self._page_anim_group.state() == QAbstractAnimation.Running:
                    self._page_anim_group.stop()
            except RuntimeError:
                pass
            self._page_anim_group = None
        for attr in ('_page_anim_prev_w', '_page_anim_target_w'):
            w = getattr(self, attr, None)
            if w:
                try: w.setGraphicsEffect(None)
                except RuntimeError: pass

        # Switch the stack immediately — no geometry manipulation on stack children
        self.pages_stack.setCurrentIndex(index)

        # Fade-in only on the incoming widget — no geometry changes on stack children.
        # Pre-apply opacity=0 BEFORE setCurrentIndex so Qt never renders it visible.
        in_widget = self.pages_stack.widget(index)

        # Clean up any leftover effect from a prior interrupted transition
        prev_target = getattr(self, '_page_anim_target_w', None)
        if prev_target and prev_target is not in_widget:
            try: prev_target.setGraphicsEffect(None)
            except RuntimeError: pass
        if hasattr(self, '_page_anim') and self._page_anim is not None:
            try:
                if self._page_anim.state() == QAbstractAnimation.Running:
                    self._page_anim.stop()
            except RuntimeError: pass
            self._page_anim = None

        in_eff = QGraphicsOpacityEffect(in_widget)
        in_eff.setOpacity(0.0)
        in_widget.setGraphicsEffect(in_eff)      # invisible before switch
        self.pages_stack.setCurrentIndex(index)   # switch (in_widget already at opacity 0)
        self._page_anim_target_w = in_widget

        anim = QPropertyAnimation(in_eff, b"opacity")
        anim.setDuration(260)
        anim.setStartValue(0.0)
        anim.setEndValue(1.0)
        anim.setEasingCurve(QEasingCurve.OutCubic)
        self._page_anim = anim

        def _on_done():
            try: in_widget.setGraphicsEffect(None)
            except RuntimeError: pass
            self._page_anim = None

        anim.finished.connect(_on_done)
        anim.start()

    def init_ui(self):
        self.setWindowFlags(Qt.FramelessWindowHint | Qt.Dialog)
        self.setAttribute(Qt.WA_TranslucentBackground, True)
        self.setFixedSize(820, 620)

        # Checkmark SVG for checkboxes
        base_dir = get_app_data_dir()
        checkmark_path = os.path.join(base_dir, "checkmark.svg").replace("\\", "/")
        if not os.path.exists(checkmark_path):
            try:
                with open(checkmark_path, "w", encoding="utf-8") as f:
                    f.write(
                        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="16" height="16">'
                        '<path fill="none" stroke="#ffffff" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" d="M3 8.5l3.5 3.5 6.5-7"/>'
                        '</svg>'
                    )
            except Exception:
                pass

        self._checkbox_style = f"""
            QCheckBox {{
                color: rgba(255,255,255,210);
                font-size: 12px;
                font-family: 'Segoe UI', sans-serif;
                font-weight: 600;
                spacing: 8px;
            }}
            QCheckBox::indicator {{
                width: 17px; height: 17px;
                min-width: 17px; max-width: 17px;
                min-height: 17px; max-height: 17px;
                border: 1px solid rgba(255,255,255,40);
                border-radius: 4px;
                background-color: rgba(255,255,255,8);
            }}
            QCheckBox::indicator:unchecked:hover {{ border-color: #8b5cf6; }}
            QCheckBox::indicator:checked {{
                background-color: #8b5cf6;
                border: 1px solid #8b5cf6;
                image: url("{checkmark_path}");
            }}
        """

        # Outer window (for drop shadow)
        self.container_frame = QFrame(self)
        self.container_frame.setObjectName("container_frame")
        self.container_frame.setGeometry(10, 10, 800, 600)
        self.container_frame.setStyleSheet("""
            QFrame#container_frame {
                background-color: #0d0d0d;
                border: 1px solid rgba(255,255,255,25);
                border-radius: 14px;
            }
        """)
        shadow = QGraphicsDropShadowEffect(self)
        shadow.setBlurRadius(30)
        shadow.setColor(QColor(0, 0, 0, 200))
        shadow.setOffset(0, 6)
        self.container_frame.setGraphicsEffect(shadow)

        # Root layout: title bar + body
        root_layout = QVBoxLayout(self.container_frame)
        root_layout.setContentsMargins(0, 0, 0, 0)
        root_layout.setSpacing(0)

        # ── Title Bar ──────────────────────────────────────────────
        title_bar = QFrame()
        title_bar.setFixedHeight(48)
        title_bar.setStyleSheet("""
            QFrame {
                background-color: rgba(255,255,255,4);
                border-bottom: 1px solid rgba(255,255,255,12);
                border-top-left-radius: 14px;
                border-top-right-radius: 14px;
            }
        """)
        tb_layout = QHBoxLayout(title_bar)
        tb_layout.setContentsMargins(20, 0, 16, 0)
        tb_layout.setSpacing(10)

        logo_lbl = QLabel()
        _icon_path = get_resource_path("icon.png")
        if os.path.exists(_icon_path):
            _pix = QIcon(_icon_path).pixmap(20, 20)
            logo_lbl.setPixmap(_pix)
        else:
            logo_lbl.setText("🎙️")
            logo_lbl.setStyleSheet("font-size: 16px;")
        logo_lbl.setFixedSize(22, 22)
        logo_lbl.setAlignment(Qt.AlignCenter)
        logo_lbl.setStyleSheet("background: transparent; border: none;")
        title_lbl = QLabel("FlowVoice")
        title_lbl.setStyleSheet("color: #ffffff; font-size: 13px; font-weight: 700; font-family: 'Segoe UI', sans-serif; letter-spacing: 0.5px; background: transparent; border: none;")
        subtitle_lbl = QLabel(tr("settings"))
        subtitle_lbl.setStyleSheet("color: rgba(255,255,255,50); font-size: 12px; font-weight: 400; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")

        self.page_title_lbl = QLabel(tr("nav_home"))
        self.page_title_lbl.setStyleSheet("color: rgba(139,92,246,200); font-size: 12px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none; letter-spacing: 0.3px;")

        tb_layout.addWidget(logo_lbl)
        tb_layout.addWidget(title_lbl)
        tb_layout.addWidget(subtitle_lbl)
        tb_layout.addSpacing(10)
        tb_layout.addWidget(self.page_title_lbl)
        tb_layout.addStretch()

        # Language picker button
        self._lang_btn = QPushButton(tr("language_btn").replace("🇧🇷 ", "").replace("🇺🇸 ", "").replace("🇪🇸 ", "").replace("🌐 ", ""))
        flag_path = get_resource_path(f"icons/flag_{get_language()}.svg")
        if os.path.exists(flag_path):
            self._lang_btn.setIcon(QIcon(flag_path))
        self._lang_btn.setFixedHeight(26)
        self._lang_btn.setCursor(Qt.PointingHandCursor)
        self._lang_btn.setStyleSheet("""
            QPushButton {
                background: rgba(139,92,246,18);
                border: 1px solid rgba(139,92,246,60);
                border-radius: 6px;
                color: rgba(255,255,255,200);
                font-size: 11px; font-weight: 600;
                font-family: 'Segoe UI Emoji', 'Segoe UI', sans-serif;
                padding: 0 10px;
            }
            QPushButton:hover { background: rgba(139,92,246,45); border-color: #8b5cf6; color: #fff; }
        """)
        self._lang_btn.clicked.connect(self._show_lang_menu)
        tb_layout.addWidget(self._lang_btn)
        tb_layout.addSpacing(6)

        btn_close = QPushButton("✕")
        btn_close.setFixedSize(28, 28)
        btn_close.setCursor(Qt.PointingHandCursor)
        btn_close.setStyleSheet("""
            QPushButton { background: transparent; border: none; color: rgba(255,255,255,100); font-size: 14px; font-weight: bold; border-radius: 6px; }
            QPushButton:hover { background: rgba(255,85,85,30); color: #ff5555; }
        """)
        btn_close.clicked.connect(lambda: self.fade_out_and_close(False))
        tb_layout.addWidget(btn_close)
        root_layout.addWidget(title_bar)

        # ── Body: Sidebar + Pages ──────────────────────────────────
        body = QFrame()
        body.setStyleSheet("QFrame { background: transparent; border: none; }")
        body_layout = QHBoxLayout(body)
        body_layout.setContentsMargins(0, 0, 0, 0)
        body_layout.setSpacing(0)

        # Sidebar
        sidebar = QFrame()
        sidebar.setFixedWidth(175)
        sidebar.setStyleSheet("""
            QFrame {
                background-color: rgba(255,255,255,3);
                border-right: 1px solid rgba(255,255,255,10);
                border-bottom-left-radius: 14px;
            }
        """)
        sidebar_layout = QVBoxLayout(sidebar)
        sidebar_layout.setContentsMargins(10, 16, 10, 16)
        sidebar_layout.setSpacing(4)

        nav_items = [
            ("🏠", tr("nav_home")),
            ("⚙️", tr("nav_general")),
            ("🔑", tr("nav_connections")),
            ("🖥️", tr("nav_whisper")),
            ("⌨️", tr("nav_hotkeys")),
        ]
        self._page_titles = [item[1] for item in nav_items]
        self._sidebar_btns = []
        for i, (icon, label) in enumerate(nav_items):
            btn = SidebarButton(icon, label)
            btn.setChecked(i == 0)
            btn._update_colors(i == 0)
            btn.clicked.connect(lambda checked, idx=i: self._switch_page(idx))
            sidebar_layout.addWidget(btn)
            self._sidebar_btns.append(btn)

        sidebar_layout.addStretch()

        # Wizard button at bottom of sidebar
        btn_wizard = QPushButton(tr("btn_wizard"))
        btn_wizard.setCursor(Qt.PointingHandCursor)
        btn_wizard.setFixedHeight(34)
        btn_wizard.setStyleSheet("""
            QPushButton {
                background: rgba(139,92,246,18);
                border: 1px solid rgba(139,92,246,60);
                border-radius: 7px;
                color: rgba(139,92,246,220);
                font-size: 11px;
                font-weight: 600;
                font-family: 'Segoe UI', sans-serif;
            }
            QPushButton:hover {
                background: rgba(139,92,246,35);
                border: 1px solid #8b5cf6;
                color: #ffffff;
            }
        """)
        btn_wizard.clicked.connect(self._open_wizard)
        sidebar_layout.addWidget(btn_wizard)

        body_layout.addWidget(sidebar)

        # Pages stack
        from PySide6.QtWidgets import QStackedWidget
        self.pages_stack = QStackedWidget()
        self.pages_stack.setStyleSheet("QStackedWidget { background: transparent; border: none; }")
        body_layout.addWidget(self.pages_stack, 1)
        root_layout.addWidget(body, 1)

        # ── Bottom bar ─────────────────────────────────────────────
        bottom_bar = QFrame()
        bottom_bar.setFixedHeight(56)
        bottom_bar.setStyleSheet("""
            QFrame {
                background-color: rgba(255,255,255,3);
                border-top: 1px solid rgba(255,255,255,10);
                border-bottom-left-radius: 14px;
                border-bottom-right-radius: 14px;
            }
        """)
        bb_layout = QHBoxLayout(bottom_bar)
        bb_layout.setContentsMargins(20, 0, 20, 0)
        bb_layout.setSpacing(12)

        status_text = f"v{CURRENT_VERSION}"
        status_color = "rgba(255,255,255,80)"
        if self.app and getattr(self.app, 'latest_checked_version', None):
            status_text += tr("version_update_badge", self.app.latest_checked_version)
            status_color = "#34d399"
        lbl_ver = QLabel(f"<span style='color:{status_color};'>{status_text}</span>")
        lbl_ver.setStyleSheet("font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        def _icon_link_btn(svg_path, fallback_char, url, tooltip):
            btn = QPushButton()
            btn.setToolTip(tooltip)
            btn.setFixedSize(28, 28)
            btn.setCursor(Qt.PointingHandCursor)
            btn.setStyleSheet("""
                QPushButton { background: transparent; border: none; border-radius: 6px; color: rgba(255,255,255,120); font-size: 14px; }
                QPushButton:hover { background: rgba(255,255,255,10); color: #fff; }
            """)
            _svg = get_resource_path(svg_path)
            if os.path.exists(_svg):
                pm = QPixmap(18, 18)
                pm.fill(Qt.transparent)
                try:
                    from PySide6.QtSvg import QSvgRenderer
                    from PySide6.QtCore import QRectF
                    renderer = QSvgRenderer(_svg)
                    if renderer.isValid():
                        painter = QPainter(pm)
                        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
                        painter.setOpacity(0.7)
                        renderer.render(painter, QRectF(0, 0, 18, 18))
                        painter.end()
                        icon_lbl = QLabel(btn)
                        icon_lbl.setPixmap(pm)
                        icon_lbl.setFixedSize(18, 18)
                        icon_lbl.move(5, 5)
                        icon_lbl.setAttribute(Qt.WA_TransparentForMouseEvents)
                except Exception:
                    btn.setText(fallback_char)
            else:
                btn.setText(fallback_char)
            btn.clicked.connect(lambda: QDesktopServices.openUrl(QUrl(url)))
            return btn

        bb_layout.addWidget(lbl_ver)
        bb_layout.addSpacing(6)
        bb_layout.addWidget(_icon_link_btn("icons/github.svg", "⌥", "https://github.com/cesarkali/Flow-Voice", "GitHub"))
        bb_layout.addWidget(_icon_link_btn("icons/instagram.svg", "📷", "https://www.instagram.com/cesar.kali/", "Instagram"))
        bb_layout.addWidget(_icon_link_btn("icons/globe.svg", "🌐", "https://caliberda.com.br/", "Site"))
        bb_layout.addStretch()

        btn_update = QPushButton(tr("btn_update"))
        btn_update.setObjectName("btn_check_update")
        btn_update.setFixedHeight(32)
        btn_update.setCursor(Qt.PointingHandCursor)
        btn_update.clicked.connect(self.check_updates_manually)
        btn_update.setStyleSheet("""
            QPushButton {
                background: transparent;
                border: 1px solid rgba(139,92,246,80);
                border-radius: 7px;
                color: rgba(139,92,246,200);
                font-size: 11px; font-weight: 600;
                font-family: 'Segoe UI', sans-serif;
                padding: 0 14px;
            }
            QPushButton:hover { background: rgba(139,92,246,20); border-color: #8b5cf6; color: #fff; }
        """)

        btn_cancel = QPushButton(tr("btn_cancel"))
        btn_cancel.setObjectName("btn_cancel")
        btn_cancel.setFixedHeight(32)
        btn_cancel.setCursor(Qt.PointingHandCursor)
        btn_cancel.clicked.connect(lambda: self.fade_out_and_close(False))
        btn_cancel.setStyleSheet("""
            QPushButton {
                background: transparent;
                border: 1px solid rgba(255,255,255,20);
                border-radius: 7px;
                color: rgba(255,255,255,140);
                font-size: 11px; font-weight: 600;
                font-family: 'Segoe UI', sans-serif;
                padding: 0 18px;
            }
            QPushButton:hover { background: rgba(255,255,255,10); border-color: rgba(255,255,255,50); color: #fff; }
        """)

        btn_save = QPushButton(tr("btn_save"))
        btn_save.setObjectName("btn_save")
        btn_save.setFixedHeight(32)
        btn_save.setCursor(Qt.PointingHandCursor)
        btn_save.clicked.connect(self.save_settings)
        btn_save.setStyleSheet("""
            QPushButton {
                background: #8b5cf6;
                border: none;
                border-radius: 7px;
                color: #ffffff;
                font-size: 11px; font-weight: 700;
                font-family: 'Segoe UI', sans-serif;
                padding: 0 18px;
            }
            QPushButton:hover { background: #7c3aed; }
            QPushButton:pressed { background: #6d28d9; }
        """)

        bb_layout.addWidget(btn_update)
        bb_layout.addWidget(btn_cancel)
        bb_layout.addWidget(btn_save)
        root_layout.addWidget(bottom_bar)

        # ── Build Pages ────────────────────────────────────────────
        self._build_page_home()
        self._build_page_general()
        self._build_page_connections()
        self._build_page_whisper()
        self._build_page_hotkeys()

    def _show_lang_menu(self):
        """Show language selection popup menu below the language button."""
        menu = QMenu(self)
        menu.setStyleSheet("""
            QMenu {
                background-color: #0d0d0d;
                border: 1px solid rgba(139,92,246,80);
                border-radius: 8px;
                padding: 4px;
                font-family: 'Segoe UI Emoji', 'Segoe UI', sans-serif;
                font-size: 12px;
                color: #ffffff;
            }
            QMenu::item { padding: 7px 20px; border-radius: 5px; }
            QMenu::item:selected { background: rgba(139,92,246,40); color: #fff; }
            QMenu::item:checked { color: #8b5cf6; font-weight: 700; }
        """)
        global _LANG
        for code, label in [("pt", " Português"), ("en", " English"), ("es", " Español")]:
            act = QAction(label, self, checkable=True)
            flag_path = get_resource_path(f"icons/flag_{code}.svg")
            if os.path.exists(flag_path):
                act.setIcon(QIcon(flag_path))
            act.setChecked(get_language() == code)
            act.triggered.connect(lambda checked, c=code: self._change_language(c))
            menu.addAction(act)
        btn_pos = self._lang_btn.mapToGlobal(self._lang_btn.rect().bottomLeft())
        from PySide6.QtCore import QPoint
        menu.exec(btn_pos + QPoint(0, 4))

    def _change_language(self, lang_code):
        """Switch UI language, persist to config, and reopen settings."""
        set_language(lang_code, self.config_manager)
        # Reopen settings with new language
        app_ref = self.app
        config_ref = self.config_manager
        self.fade_out_and_close(False)
        QTimer.singleShot(320, lambda: SettingsDialog(config_ref, app=app_ref).exec())

    def _build_page_home(self):
        scroll, page = self._make_scroll_page()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(28, 28, 28, 28)
        layout.setSpacing(0)

        # Hero greeting
        hero = QFrame()
        hero.setStyleSheet("""
            QFrame {
                background: qlineargradient(x1:0, y1:0, x2:1, y2:1,
                    stop:0 rgba(139,92,246,30), stop:1 rgba(236,72,153,15));
                border: 1px solid rgba(139,92,246,50);
                border-radius: 12px;
            }
        """)
        hero_layout = QVBoxLayout(hero)
        hero_layout.setContentsMargins(24, 20, 24, 20)
        hero_layout.setSpacing(6)
        lbl_hi = QLabel(tr("hero_title"))
        lbl_hi.setStyleSheet("color: #ffffff; font-size: 16px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        lbl_sub = QLabel(tr("hero_sub"))
        lbl_sub.setStyleSheet("color: rgba(255,255,255,160); font-size: 12px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        lbl_sub.setWordWrap(True)
        hero_layout.addWidget(lbl_hi)
        hero_layout.addWidget(lbl_sub)
        layout.addWidget(hero)
        layout.addSpacing(20)

        # Status cards
        layout.addWidget(self._section_label(tr("sec_status")))
        layout.addSpacing(10)

        cards_row = QHBoxLayout()
        cards_row.setSpacing(10)

        provider = self.config_manager.get("provider", "—")
        style = self.config_manager.get("active_style", "—")
        mode_raw = self.config_manager.get("operation_mode", "ditado")
        mode_display = {"ditado": tr("mode_display_ditado"), "traducao": tr("mode_display_traducao"), "pesquisa": tr("mode_display_pesquisa")}.get(mode_raw, mode_raw)
        hotkey = self.config_manager.get("hotkey", "—")

        for icon, title, value in [
            ("🤖", tr("card_provider"), provider),
            ("✍️", tr("card_style"), style),
            ("🎯", tr("card_mode"), mode_display),
            ("⌨️", tr("card_shortcut"), hotkey),
        ]:
            card = QFrame()
            card.setStyleSheet("""
                QFrame {
                    background: rgba(255,255,255,5);
                    border: 1px solid rgba(255,255,255,12);
                    border-radius: 10px;
                }
            """)
            cl = QVBoxLayout(card)
            cl.setContentsMargins(14, 12, 14, 12)
            cl.setSpacing(4)
            lbl_ic = QLabel(f"{icon}  {title}")
            lbl_ic.setStyleSheet("color: rgba(255,255,255,100); font-size: 10px; font-weight: 600; letter-spacing: 0.5px; background: transparent; border: none;")
            lbl_val = QLabel(value)
            lbl_val.setStyleSheet("color: #ffffff; font-size: 13px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            cl.addWidget(lbl_ic)
            cl.addWidget(lbl_val)
            cards_row.addWidget(card)

        layout.addLayout(cards_row)
        layout.addSpacing(20)

        # Quick guide
        layout.addWidget(self._section_label(tr("sec_how_to")))
        layout.addSpacing(10)
        steps = [
            ("1", tr("step1_title"), tr("step1_desc")),
            ("2", tr("step2_title"), tr("step2_desc")),
            ("3", tr("step3_title"), tr("step3_desc", hotkey)),
        ]
        for num, title, desc in steps:
            step_row = QHBoxLayout()
            step_row.setSpacing(12)
            num_lbl = QLabel(num)
            num_lbl.setFixedSize(28, 28)
            num_lbl.setAlignment(Qt.AlignCenter)
            num_lbl.setStyleSheet("background: rgba(139,92,246,40); border: 1px solid rgba(139,92,246,80); border-radius: 14px; color: #ffffff; font-size: 11px; font-weight: 700;")
            txt_col = QVBoxLayout()
            txt_col.setSpacing(2)
            t = QLabel(title)
            t.setStyleSheet("color: rgba(255,255,255,220); font-size: 12px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            d = QLabel(desc)
            d.setStyleSheet("color: rgba(255,255,255,100); font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            d.setWordWrap(True)
            txt_col.addWidget(t)
            txt_col.addWidget(d)
            step_row.addWidget(num_lbl)
            step_row.addLayout(txt_col, 1)
            layout.addLayout(step_row)
            layout.addSpacing(10)

        layout.addStretch()
        self.pages_stack.addWidget(scroll)

    def _build_page_general(self):
        scroll, page = self._make_scroll_page()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(28, 28, 28, 28)
        layout.setSpacing(0)

        layout.addWidget(self._section_label(tr("sec_ai_behavior")))
        layout.addSpacing(10)

        layout.addWidget(self._field_label(tr("field_writing_style")))
        self.combo_style = self._combo(["Profissional", "Casual", "Direto"], self.config_manager.get("active_style", "Profissional"))
        layout.addWidget(self.combo_style)
        layout.addWidget(self._desc(tr("desc_writing_style")))
        layout.addSpacing(6)

        layout.addWidget(self._field_label(tr("field_prompt_version")))
        self.combo_prompt_version = self._combo(["v2 (Beta)", "v1"], "v2 (Beta)" if self.config_manager.get("prompt_version", "v2") == "v2" else "v1")
        layout.addWidget(self.combo_prompt_version)
        layout.addWidget(self._desc(tr("desc_prompt_version")))
        layout.addSpacing(14)

        layout.addWidget(self._separator())
        layout.addSpacing(8)
        layout.addWidget(self._section_label(tr("sec_default_op")))
        layout.addSpacing(10)

        layout.addWidget(self._field_label(tr("field_operation_mode")))
        mode_map = {"ditado": tr("combo_dictation"), "traducao": tr("combo_translation"), "pesquisa": tr("combo_search")}
        self.combo_mode = self._combo([tr("combo_dictation"), tr("combo_translation"), tr("combo_search")], mode_map.get(self.config_manager.get("operation_mode", "ditado"), tr("combo_dictation")))
        layout.addWidget(self.combo_mode)
        layout.addWidget(self._desc(tr("desc_operation_mode")))
        layout.addSpacing(14)

        layout.addWidget(self._separator())
        layout.addSpacing(8)
        layout.addWidget(self._section_label(tr("sec_web_search")))
        layout.addSpacing(10)

        # Info banner
        info_banner = QFrame()
        info_banner.setStyleSheet("QFrame { background: rgba(139,92,246,10); border: 1px solid rgba(139,92,246,40); border-radius: 8px; }")
        info_l = QVBoxLayout(info_banner)
        info_l.setContentsMargins(14, 10, 14, 10)
        info_l.setSpacing(3)
        info_title = QLabel(tr("web_info_title"))
        info_title.setStyleSheet("color: #a78bfa; font-size: 11px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        info_body = QLabel(tr("web_info_body"))
        info_body.setStyleSheet("color: rgba(255,255,255,130); font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        info_body.setWordWrap(True)
        info_l.addWidget(info_title)
        info_l.addWidget(info_body)
        layout.addWidget(info_banner)
        layout.addSpacing(10)

        _sp_groq = tr("combo_search_provider_groq")
        _sp_gemini = tr("combo_search_provider_gemini")
        _sp_auto = tr("combo_search_provider_auto")
        layout.addWidget(self._field_label(tr("field_search_provider")))
        self.combo_search_provider = self._combo(
            [_sp_groq, _sp_gemini, _sp_auto],
            {"groq": _sp_groq, "gemini": _sp_gemini, "auto": _sp_auto}.get(
                self.config_manager.get("search_provider", "groq"), _sp_groq
            )
        )
        layout.addWidget(self.combo_search_provider)
        layout.addWidget(self._desc(tr("desc_search_provider")))
        layout.addSpacing(6)

        _sm_compound = tr("combo_search_model_compound")
        _sm_mini = tr("combo_search_model_mini")
        _sm_llama = tr("combo_search_model_llama")
        layout.addWidget(self._field_label(tr("field_search_model")))
        self.combo_search_model = self._combo(
            [_sm_compound, _sm_mini, _sm_llama],
            {
                "groq/compound": _sm_compound,
                "groq/compound-mini": _sm_mini,
                "llama-3.3-70b-versatile": _sm_llama
            }.get(self.config_manager.get("search_model", "groq/compound"), _sm_compound)
        )
        layout.addWidget(self.combo_search_model)
        layout.addWidget(self._desc(tr("desc_search_model")))
        layout.addSpacing(6)

        layout.addWidget(self._field_label(tr("field_translation_lang")))
        self.combo_target_lang = self._combo(["Inglês", "Espanhol", "Francês", "Alemão", "Italiano"], self.config_manager.get("translation_target", "Inglês"))
        layout.addWidget(self.combo_target_lang)
        layout.addWidget(self._desc(tr("desc_translation_lang")))
        layout.addSpacing(14)

        layout.addWidget(self._separator())
        layout.addSpacing(8)
        layout.addWidget(self._section_label(tr("sec_system")))
        layout.addSpacing(10)

        startup_label = tr("field_startup") if sys.platform == 'win32' else tr("field_startup_linux")
        self.chk_startup = QCheckBox(startup_label)
        self.chk_startup.setChecked(is_run_at_startup_enabled() or self.config_manager.get("start_with_windows", False))
        self.chk_startup.setStyleSheet(self._checkbox_style)
        layout.addWidget(self.chk_startup)
        layout.addWidget(self._desc(tr("desc_startup")))
        layout.addSpacing(6)

        self.chk_mute = QCheckBox(tr("field_mute"))
        self.chk_mute.setChecked(self.config_manager.get("mute_on_record", False))
        self.chk_mute.setStyleSheet(self._checkbox_style)
        layout.addWidget(self.chk_mute)
        layout.addWidget(self._desc(tr("desc_mute")))
        layout.addSpacing(14)

        layout.addWidget(self._separator())
        layout.addSpacing(8)
        layout.addWidget(self._section_label(tr("sec_security")))
        layout.addSpacing(10)
        layout.addWidget(self._field_label(tr("field_pwd_label")))

        # Hidden real field used only for saving; shown/editable only after auth
        self.txt_keys_password = self._lineedit(self.config_manager.get("keys_password", ""), "", password=True)
        self.txt_keys_password.hide()

        # Display row: masked preview + change/set button
        pwd_display_row = QHBoxLayout()
        pwd_display_row.setSpacing(6)
        _has_pwd = bool(self.config_manager.get("keys_password", ""))
        self._pwd_status_lbl = QLabel(tr("pwd_masked") if _has_pwd else tr("pwd_no_password"))
        self._pwd_status_lbl.setStyleSheet("color: %s; font-size: 12px; font-family: 'Segoe UI', sans-serif; background: rgba(255,255,255,5); border: 1px solid rgba(255,255,255,15); border-radius: 7px; padding: 8px 12px;" % ("rgba(255,255,255,160)" if _has_pwd else "rgba(255,255,255,60)"))
        btn_change_pwd = QPushButton(tr("pwd_change") if _has_pwd else tr("pwd_set"))
        btn_change_pwd.setFixedHeight(36)
        btn_change_pwd.setCursor(Qt.PointingHandCursor)
        btn_change_pwd.setStyleSheet("QPushButton { background: rgba(255,255,255,8); border: 1px solid rgba(255,255,255,20); border-radius: 7px; color: rgba(255,255,255,160); font-size: 11px; font-family: 'Segoe UI', sans-serif; padding: 0 14px; } QPushButton:hover { background: rgba(255,255,255,15); color: #fff; }")
        btn_remove_pwd = QPushButton(tr("pwd_remove"))
        btn_remove_pwd.setFixedHeight(36)
        btn_remove_pwd.setCursor(Qt.PointingHandCursor)
        btn_remove_pwd.setVisible(_has_pwd)
        btn_remove_pwd.setStyleSheet("QPushButton { background: rgba(239,68,68,10); border: 1px solid rgba(239,68,68,40); border-radius: 7px; color: #f87171; font-size: 11px; font-family: 'Segoe UI', sans-serif; padding: 0 12px; } QPushButton:hover { background: rgba(239,68,68,22); color: #fff; }")
        pwd_display_row.addWidget(self._pwd_status_lbl, 1)
        pwd_display_row.addWidget(btn_change_pwd)
        pwd_display_row.addWidget(btn_remove_pwd)
        layout.addLayout(pwd_display_row)

        # "Esqueci a senha" link — only shown when a password exists
        self._btn_forgot_pwd = QPushButton(tr("pwd_forgot"))
        self._btn_forgot_pwd.setCursor(Qt.PointingHandCursor)
        self._btn_forgot_pwd.setVisible(_has_pwd)
        self._btn_forgot_pwd.setStyleSheet("QPushButton { background: transparent; border: none; color: rgba(139,92,246,180); font-size: 11px; font-family: 'Segoe UI', sans-serif; text-align: left; padding: 2px 0; } QPushButton:hover { color: #8b5cf6; }")
        layout.addWidget(self._btn_forgot_pwd)

        layout.addWidget(self._desc(tr("desc_pwd")))

        def _do_change_pwd():
            current_pwd = self.config_manager.get("keys_password", "")
            if current_pwd:
                # Must verify current password first
                dlg, inp, err_lbl, btn_ok = self._make_pwd_dialog(
                    tr("pwd_confirm_title"),
                    tr("pwd_confirm_subtitle"),
                    show_reset=False
                )
                def _verify():
                    if inp.text() == current_pwd:
                        dlg.accept()
                    else:
                        err_lbl.setText(tr("pwd_wrong_error"))
                        inp.clear(); inp.setFocus()
                btn_ok.clicked.connect(_verify)
                inp.returnPressed.connect(_verify)
                if dlg.exec() != QDialog.Accepted:
                    return
            # Now ask for new password
            dlg2 = QDialog(self)
            dlg2.setWindowFlags(Qt.FramelessWindowHint | Qt.Dialog)
            dlg2.setAttribute(Qt.WA_TranslucentBackground, True)
            dlg2.setFixedSize(380, 250)
            outer2 = QFrame(dlg2)
            outer2.setGeometry(8, 8, 364, 234)
            outer2.setStyleSheet("QFrame { background: #0d0d0d; border: 1px solid rgba(255,255,255,25); border-radius: 12px; }")
            shadow2 = QGraphicsDropShadowEffect(dlg2)
            shadow2.setBlurRadius(20); shadow2.setColor(QColor(0, 0, 0, 180)); shadow2.setOffset(0, 4)
            outer2.setGraphicsEffect(shadow2)
            vl2 = QVBoxLayout(outer2)
            vl2.setContentsMargins(24, 20, 24, 20)
            vl2.setSpacing(10)
            lbl_t = QLabel("🔑  %s" % (tr("pwd_change_title")[5:] if current_pwd else tr("pwd_set_title")[5:]))
            lbl_t.setStyleSheet("color: #ffffff; font-size: 13px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            inp_new = QLineEdit()
            inp_new.setEchoMode(QLineEdit.Password)
            inp_new.setPlaceholderText(tr("pwd_new_placeholder"))
            inp_new.setFixedHeight(36)
            inp_new.setStyleSheet("QLineEdit { background: rgba(255,255,255,8); border: 1px solid rgba(255,255,255,20); border-radius: 7px; padding: 6px 12px; color: #fff; font-size: 12px; font-family: 'Segoe UI', sans-serif; } QLineEdit:focus { border-color: #8b5cf6; }")
            inp_conf = QLineEdit()
            inp_conf.setEchoMode(QLineEdit.Password)
            inp_conf.setPlaceholderText(tr("pwd_confirm_placeholder"))
            inp_conf.setFixedHeight(36)
            inp_conf.setStyleSheet(inp_new.styleSheet())
            err2 = QLabel("")
            err2.setStyleSheet("color: #f87171; font-size: 11px; background: transparent; border: none;")
            btn_row2 = QHBoxLayout()
            btn_row2.addStretch()
            btn_c2 = QPushButton(tr("btn_cancel"))
            btn_c2.setFixedHeight(32)
            btn_c2.setCursor(Qt.PointingHandCursor)
            btn_c2.setStyleSheet("QPushButton { background: transparent; border: 1px solid rgba(255,255,255,20); border-radius: 7px; color: rgba(255,255,255,140); font-size: 11px; font-family: 'Segoe UI', sans-serif; padding: 0 14px; } QPushButton:hover { background: rgba(255,255,255,8); color: #fff; }")
            btn_ok2 = QPushButton(tr("btn_save_short"))
            btn_ok2.setFixedHeight(32)
            btn_ok2.setCursor(Qt.PointingHandCursor)
            btn_ok2.setStyleSheet("QPushButton { background: #8b5cf6; border: none; border-radius: 7px; color: #fff; font-size: 11px; font-weight: 700; font-family: 'Segoe UI', sans-serif; padding: 0 16px; } QPushButton:hover { background: #7c3aed; }")
            btn_row2.addWidget(btn_c2); btn_row2.addWidget(btn_ok2)
            vl2.addWidget(lbl_t); vl2.addWidget(inp_new); vl2.addWidget(inp_conf); vl2.addWidget(err2); vl2.addLayout(btn_row2)
            btn_c2.clicked.connect(dlg2.reject)
            def _save_new():
                v1, v2 = inp_new.text(), inp_conf.text()
                if not v1:
                    err2.setText(tr("pwd_empty_error"))
                    return
                if v1 != v2:
                    err2.setText(tr("pwd_mismatch_error"))
                    inp_conf.clear(); inp_conf.setFocus()
                    return
                dlg2.done(1)
            btn_ok2.clicked.connect(_save_new)
            inp_conf.returnPressed.connect(_save_new)
            if dlg2.exec() != 1:
                return
            new_val = inp_new.text()
            self.txt_keys_password.setText(new_val)
            self.config_manager.set("keys_password", new_val)
            self._pwd_status_lbl.setText(tr("pwd_masked"))
            self._pwd_status_lbl.setStyleSheet("color: rgba(255,255,255,160); font-size: 12px; font-family: 'Segoe UI', sans-serif; background: rgba(255,255,255,5); border: 1px solid rgba(255,255,255,15); border-radius: 7px; padding: 8px 12px;")
            btn_change_pwd.setText(tr("pwd_change"))
            btn_remove_pwd.setVisible(True)
            self._btn_forgot_pwd.setVisible(True)

        def _do_remove_pwd():
            current_pwd = self.config_manager.get("keys_password", "")
            if current_pwd:
                dlg, inp, err_lbl, btn_ok = self._make_pwd_dialog(tr("pwd_verify_remove_title"))
                def _verify_remove():
                    if inp.text() == current_pwd:
                        dlg.accept()
                    else:
                        err_lbl.setText("Senha incorreta.")
                        inp.clear(); inp.setFocus()
                btn_ok.clicked.connect(_verify_remove)
                inp.returnPressed.connect(_verify_remove)
                if dlg.exec() != QDialog.Accepted:
                    return
            self.txt_keys_password.setText("")
            self.config_manager.set("keys_password", "")
            self._pwd_status_lbl.setText(tr("pwd_no_password"))
            self._pwd_status_lbl.setStyleSheet("color: rgba(255,255,255,60); font-size: 12px; font-family: 'Segoe UI', sans-serif; background: rgba(255,255,255,5); border: 1px solid rgba(255,255,255,15); border-radius: 7px; padding: 8px 12px;")
            btn_change_pwd.setText(tr("pwd_set"))
            btn_remove_pwd.setVisible(False)
            self._btn_forgot_pwd.setVisible(False)

        def _do_forgot_pwd():
            dlg = QDialog(self)
            dlg.setWindowFlags(Qt.FramelessWindowHint | Qt.Dialog)
            dlg.setAttribute(Qt.WA_TranslucentBackground, True)
            dlg.setFixedSize(400, 230)
            outer = QFrame(dlg)
            outer.setGeometry(8, 8, 384, 214)
            outer.setStyleSheet("QFrame { background: #0d0d0d; border: 1px solid rgba(255,255,255,25); border-radius: 12px; }")
            shadow = QGraphicsDropShadowEffect(dlg)
            shadow.setBlurRadius(20); shadow.setColor(QColor(0, 0, 0, 180)); shadow.setOffset(0, 4)
            outer.setGraphicsEffect(shadow)
            vl = QVBoxLayout(outer)
            vl.setContentsMargins(24, 20, 24, 20)
            vl.setSpacing(10)
            lbl_t = QLabel(tr("pwd_forgot_title"))
            lbl_t.setStyleSheet("color: #ffffff; font-size: 13px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            lbl_s = QLabel(tr("pwd_forgot_body"))
            lbl_s.setStyleSheet("color: rgba(255,255,255,140); font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            lbl_s.setWordWrap(True)
            btn_row = QHBoxLayout()
            btn_cancel = QPushButton(tr("btn_cancel"))
            btn_cancel.setFixedHeight(32); btn_cancel.setCursor(Qt.PointingHandCursor)
            btn_cancel.setStyleSheet("QPushButton { background: transparent; border: 1px solid rgba(255,255,255,20); border-radius: 7px; color: rgba(255,255,255,140); font-size: 11px; font-family: 'Segoe UI', sans-serif; padding: 0 14px; } QPushButton:hover { background: rgba(255,255,255,8); color: #fff; }")
            btn_reset = QPushButton(tr("pwd_reset_all"))
            btn_reset.setFixedHeight(32); btn_reset.setCursor(Qt.PointingHandCursor)
            btn_reset.setStyleSheet("QPushButton { background: rgba(239,68,68,12); border: 1px solid rgba(239,68,68,50); border-radius: 7px; color: #f87171; font-size: 11px; font-family: 'Segoe UI', sans-serif; padding: 0 12px; } QPushButton:hover { background: rgba(239,68,68,25); color: #fff; }")
            btn_row.addStretch()
            btn_row.addWidget(btn_cancel)
            btn_row.addWidget(btn_reset)
            vl.addWidget(lbl_t); vl.addWidget(lbl_s); vl.addStretch(); vl.addLayout(btn_row)
            btn_cancel.clicked.connect(dlg.reject)
            btn_reset.clicked.connect(dlg.accept)
            if dlg.exec() != QDialog.Accepted:
                return
            # Reset everything
            for key in ("keys_password", "tavily_api_key"):
                self.config_manager.set(key, "")
            for provider in ("groq", "gemini", "openai", "github_models"):
                self.config_manager.set_api_key(provider, "")
            # Update UI fields that exist
            for attr, val in [("txt_keys_password", ""), ("txt_tavily", ""), ("txt_groq", ""), ("txt_gemini", ""), ("txt_openai", ""), ("txt_github_models", "")]:
                field = getattr(self, attr, None)
                if field:
                    field.setText("")
            self._pwd_status_lbl.setText("Nenhuma senha definida")
            self._pwd_status_lbl.setStyleSheet("color: rgba(255,255,255,60); font-size: 12px; font-family: 'Segoe UI', sans-serif; background: rgba(255,255,255,5); border: 1px solid rgba(255,255,255,15); border-radius: 7px; padding: 8px 12px;")
            btn_change_pwd.setText("Definir senha")
            btn_remove_pwd.setVisible(False)
            self._btn_forgot_pwd.setVisible(False)

        btn_change_pwd.clicked.connect(_do_change_pwd)
        btn_remove_pwd.clicked.connect(_do_remove_pwd)
        self._btn_forgot_pwd.clicked.connect(_do_forgot_pwd)

        layout.addStretch()
        self.pages_stack.addWidget(scroll)

    def _build_page_connections(self):
        scroll, page = self._make_scroll_page()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(28, 28, 28, 28)
        layout.setSpacing(0)

        layout.addWidget(self._section_label(tr("sec_preferred_provider")))
        layout.addSpacing(10)
        layout.addWidget(self._field_label(tr("field_ai_provider")))
        self.combo_provider = self._combo(["github_models", "groq", "gemini", "openai"], self.config_manager.get("provider", "groq"))
        layout.addWidget(self.combo_provider)
        layout.addWidget(self._desc(tr("desc_ai_provider")))
        layout.addSpacing(14)

        layout.addWidget(self._separator())
        layout.addSpacing(8)
        layout.addWidget(self._section_label(tr("sec_api_keys")))
        layout.addSpacing(6)

        # Info box
        info = QFrame()
        info.setStyleSheet("""
            QFrame {
                background: rgba(139,92,246,12);
                border: 1px solid rgba(139,92,246,40);
                border-radius: 8px;
            }
        """)
        info_l = QVBoxLayout(info)
        info_l.setContentsMargins(14, 10, 14, 10)
        info_l.setSpacing(4)
        lbl_it = QLabel(tr("conn_info_title"))
        lbl_it.setStyleSheet("color: rgba(255,255,255,200); font-size: 11px; font-weight: 700; background: transparent; border: none;")
        lbl_id = QLabel(tr("conn_info_body"))
        lbl_id.setOpenExternalLinks(True)
        lbl_id.setWordWrap(True)
        lbl_id.setStyleSheet("color: rgba(255,255,255,140); font-size: 11px; background: transparent; border: none;")
        info_l.addWidget(lbl_it)
        info_l.addWidget(lbl_id)
        layout.addWidget(info)
        layout.addSpacing(14)

        for provider_id, label in [
            ("github_models", "GitHub Models"),
            ("groq", "Groq"),
            ("gemini", "Google Gemini"),
            ("openai", "OpenAI"),
        ]:
            if provider_id == "github_models":
                key_icon_path = get_resource_path("icons/github.svg")
            else:
                key_icon_path = get_resource_path(f"icons/{provider_id}.svg")
            
            row = QHBoxLayout()
            row.setSpacing(6)
            row.setContentsMargins(0, 10, 0, 4)
            
            icon_lbl = QLabel()
            icon_lbl.setFixedSize(14, 14)
            icon_lbl.setStyleSheet("background: transparent; border: none; margin: 0; padding: 0;")
            
            pm = QPixmap(14, 14)
            pm.fill(Qt.transparent)
            try:
                from PySide6.QtSvg import QSvgRenderer
                from PySide6.QtCore import QRectF
                renderer = QSvgRenderer(key_icon_path)
                painter = QPainter(pm)
                painter.setRenderHint(QPainter.RenderHint.Antialiasing)
                renderer.render(painter, QRectF(0, 0, 14, 14))
                painter.end()
            except Exception:
                pass
            icon_lbl.setPixmap(pm)
            
            txt_lbl = QLabel(tr("field_api_key_label", label))
            txt_lbl.setStyleSheet("color: rgba(255,255,255,200); font-size: 12px; font-weight: 600; font-family: 'Segoe UI', sans-serif; background: transparent; border: none; margin: 0; padding: 0;")
            
            row.addWidget(icon_lbl)
            row.addWidget(txt_lbl, 1)
            
            layout.addLayout(row)
            
            le = self._lineedit(self.config_manager.get_api_key(provider_id), tr("conn_key_placeholder"), password=True)
            setattr(self, f"txt_{provider_id}", le)
            layout.addWidget(le)
            layout.addSpacing(8)

        layout.addWidget(self._desc(tr("desc_api_keys")))
        layout.addSpacing(14)

        layout.addWidget(self._separator())
        layout.addSpacing(8)
        layout.addWidget(self._section_label(tr("sec_web_search_conn")))
        layout.addSpacing(10)

        tavily_banner = QFrame()
        tavily_banner.setStyleSheet("QFrame { background: rgba(16,185,129,8); border: 1px solid rgba(16,185,129,35); border-radius: 8px; }")
        tavily_l = QVBoxLayout(tavily_banner)
        tavily_l.setContentsMargins(14, 10, 14, 10)
        tavily_l.setSpacing(3)
        tavily_title = QLabel(tr("tavily_title"))
        tavily_title.setStyleSheet("color: #6ee7b7; font-size: 11px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        tavily_body = QLabel(tr("tavily_body"))
        tavily_body.setOpenExternalLinks(True)
        tavily_body.setWordWrap(True)
        tavily_body.setStyleSheet("color: rgba(255,255,255,130); font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        tavily_l.addWidget(tavily_title)
        tavily_l.addWidget(tavily_body)
        layout.addWidget(tavily_banner)
        layout.addSpacing(10)

        layout.addWidget(self._field_label(tr("field_tavily_key")))
        self.txt_tavily = self._lineedit(self.config_manager.get("tavily_api_key", ""), "tvly-...", password=True)
        layout.addWidget(self.txt_tavily)
        layout.addWidget(self._desc(tr("desc_tavily")))
        layout.addSpacing(10)

        # Test connection button + status label
        test_row = QHBoxLayout()
        test_row.setSpacing(10)
        self._btn_test_conn = QPushButton(tr("btn_test_conn"))
        self._btn_test_conn.setFixedHeight(34)
        self._btn_test_conn.setCursor(Qt.PointingHandCursor)
        self._btn_test_conn.setStyleSheet("""
            QPushButton {
                background: rgba(6,182,212,15);
                border: 1px solid rgba(6,182,212,60);
                border-radius: 7px;
                color: rgba(6,182,212,220);
                font-size: 11px; font-weight: 600;
                font-family: 'Segoe UI', sans-serif;
                padding: 0 16px;
            }
            QPushButton:hover { background: rgba(6,182,212,30); border-color: #06b6d4; color: #fff; }
            QPushButton:disabled { opacity: 0.5; }
        """)
        self._lbl_conn_status = QLabel("")
        self._lbl_conn_status.setStyleSheet("font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        self._btn_test_conn.clicked.connect(self._test_connection)
        btn_reveal = QPushButton(tr("btn_reveal_keys"))
        btn_reveal.setFixedHeight(34)
        btn_reveal.setCursor(Qt.PointingHandCursor)
        btn_reveal.setStyleSheet("""
            QPushButton {
                background: rgba(139,92,246,15);
                border: 1px solid rgba(139,92,246,60);
                border-radius: 7px;
                color: rgba(139,92,246,220);
                font-size: 11px; font-weight: 600;
                font-family: 'Segoe UI', sans-serif;
                padding: 0 16px;
            }
            QPushButton:hover { background: rgba(139,92,246,30); border-color: #8b5cf6; color: #fff; }
        """)
        btn_reveal.clicked.connect(self._reveal_keys)
        test_row.addWidget(self._btn_test_conn)
        test_row.addWidget(btn_reveal)
        test_row.addWidget(self._lbl_conn_status, 1)
        layout.addLayout(test_row)

        layout.addStretch()
        self.pages_stack.addWidget(scroll)

    def _make_pwd_dialog(self, title, subtitle="", show_reset=False):
        """Generic password dialog. Returns (dialog, inp, err_lbl). Caller calls dlg.exec()."""
        dlg = QDialog(self)
        dlg.setWindowFlags(Qt.FramelessWindowHint | Qt.Dialog)
        dlg.setAttribute(Qt.WA_TranslucentBackground, True)
        h = 260 if show_reset else 210
        dlg.setFixedSize(380, h)
        outer = QFrame(dlg)
        outer.setGeometry(8, 8, 364, h - 16)
        outer.setStyleSheet("QFrame { background: #0d0d0d; border: 1px solid rgba(255,255,255,25); border-radius: 12px; }")
        shadow = QGraphicsDropShadowEffect(dlg)
        shadow.setBlurRadius(20); shadow.setColor(QColor(0, 0, 0, 180)); shadow.setOffset(0, 4)
        outer.setGraphicsEffect(shadow)
        vl = QVBoxLayout(outer)
        vl.setContentsMargins(24, 20, 24, 20)
        vl.setSpacing(10)
        lbl_title = QLabel(title)
        lbl_title.setStyleSheet("color: #ffffff; font-size: 13px; font-weight: 700; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
        lbl_title.setWordWrap(True)
        vl.addWidget(lbl_title)
        if subtitle:
            lbl_sub = QLabel(subtitle)
            lbl_sub.setStyleSheet("color: rgba(255,255,255,120); font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none;")
            lbl_sub.setWordWrap(True)
            vl.addWidget(lbl_sub)
        inp = QLineEdit()
        inp.setEchoMode(QLineEdit.Password)
        inp.setPlaceholderText(tr("pwd_placeholder"))
        inp.setFixedHeight(36)
        inp.setStyleSheet("QLineEdit { background: rgba(255,255,255,8); border: 1px solid rgba(255,255,255,20); border-radius: 7px; padding: 6px 12px; color: #fff; font-size: 12px; font-family: 'Segoe UI', sans-serif; } QLineEdit:focus { border-color: #8b5cf6; }")
        vl.addWidget(inp)
        err_lbl = QLabel("")
        err_lbl.setStyleSheet("color: #f87171; font-size: 11px; background: transparent; border: none;")
        vl.addWidget(err_lbl)
        btn_row = QHBoxLayout()
        if show_reset:
            btn_reset = QPushButton(tr("pwd_reset_btn"))
            btn_reset.setFixedHeight(32)
            btn_reset.setCursor(Qt.PointingHandCursor)
            btn_reset.setStyleSheet("QPushButton { background: rgba(239,68,68,12); border: 1px solid rgba(239,68,68,50); border-radius: 7px; color: #f87171; font-size: 11px; font-family: 'Segoe UI', sans-serif; padding: 0 12px; } QPushButton:hover { background: rgba(239,68,68,25); color: #fff; }")
            btn_reset.clicked.connect(lambda: dlg.done(2))
            btn_row.addWidget(btn_reset)
        btn_row.addStretch()
        btn_cancel = QPushButton(tr("btn_cancel"))
        btn_cancel.setFixedHeight(32)
        btn_cancel.setCursor(Qt.PointingHandCursor)
        btn_cancel.setStyleSheet("QPushButton { background: transparent; border: 1px solid rgba(255,255,255,20); border-radius: 7px; color: rgba(255,255,255,140); font-size: 11px; font-family: 'Segoe UI', sans-serif; padding: 0 14px; } QPushButton:hover { background: rgba(255,255,255,8); color: #fff; }")
        btn_ok = QPushButton(tr("btn_confirm"))
        btn_ok.setFixedHeight(32)
        btn_ok.setCursor(Qt.PointingHandCursor)
        btn_ok.setStyleSheet("QPushButton { background: #8b5cf6; border: none; border-radius: 7px; color: #fff; font-size: 11px; font-weight: 700; font-family: 'Segoe UI', sans-serif; padding: 0 16px; } QPushButton:hover { background: #7c3aed; }")
        btn_row.addWidget(btn_cancel)
        btn_row.addWidget(btn_ok)
        vl.addLayout(btn_row)
        btn_cancel.clicked.connect(dlg.reject)
        return dlg, inp, err_lbl, btn_ok

    def _reveal_keys(self):
        """Toggles key visibility after optional password check."""
        pwd = self.config_manager.get("keys_password", "")
        key_fields = [
            self.txt_github_models, self.txt_groq,
            self.txt_gemini, self.txt_openai, self.txt_tavily,
        ]
        currently_visible = key_fields[0].echoMode() == QLineEdit.Normal
        if currently_visible:
            for le in key_fields:
                le.setEchoMode(QLineEdit.Password)
            return
        if not pwd:
            for le in key_fields:
                le.setEchoMode(QLineEdit.Normal)
            return
        dlg, inp, err_lbl, btn_ok = self._make_pwd_dialog("🔐  Digite sua senha para ver as chaves")
        def _check():
            if inp.text() == pwd:
                dlg.accept()
            else:
                err_lbl.setText(tr("pwd_wrong_error"))
                inp.clear(); inp.setFocus()
        btn_ok.clicked.connect(_check)
        inp.returnPressed.connect(_check)
        if dlg.exec() != QDialog.Accepted:
            return
        for le in key_fields:
            le.setEchoMode(QLineEdit.Normal)

    def _test_connection(self):
        """Sends a minimal test prompt to the preferred provider and reports status."""
        self._btn_test_conn.setEnabled(False)
        self._btn_test_conn.setText(tr("test_conn_testing"))
        self._lbl_conn_status.setText("")
        self._lbl_conn_status.setStyleSheet("font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none; color: rgba(255,255,255,100);")

        provider = self.combo_provider.currentText()
        key_attr = f"txt_{provider}"
        key = getattr(self, key_attr, None)
        key_val = key.text().strip() if key else self.config_manager.get_api_key(provider)
        if not key_val:
            self._btn_test_conn.setEnabled(True)
            self._btn_test_conn.setText(tr("btn_test_conn"))
            self._lbl_conn_status.setText(tr("test_conn_no_key"))
            self._lbl_conn_status.setStyleSheet("font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none; color: #f59e0b;")
            return

        class _TestWorker(QThread):
            done = Signal(bool, str)
            def __init__(self, provider, key):
                super().__init__()
                self._provider = provider
                self._key = key
            def run(self):
                masked = (self._key[:8] + "...") if len(self._key) > 8 else "..."
                print(f"[Teste de Conexão] Iniciando teste para provedor: {self._provider}")
                print(f"[Teste de Conexão] Chave utilizada: {masked}")
                try:
                    from openai import OpenAI
                    import google.generativeai as genai
                    test_prompt = [{"role": "user", "content": "Reply with just: OK"}]
                    if self._provider == "gemini":
                        print("[Teste de Conexão] Enviando requisição para Gemini (gemini-1.5-flash)...")
                        genai.configure(api_key=self._key)
                        model = genai.GenerativeModel("gemini-1.5-flash")
                        r = model.generate_content("Reply with just: OK")
                        print(f"[Teste de Conexão] Resposta recebida: {r.text.strip()[:60]}")
                        self.done.emit(bool(r.text), r.text.strip()[:60])
                    elif self._provider == "groq":
                        print("[Teste de Conexão] Enviando requisição para Groq (llama-3.3-70b-versatile)...")
                        client = OpenAI(base_url="https://api.groq.com/openai/v1", api_key=self._key)
                        r = client.chat.completions.create(model="llama-3.3-70b-versatile", messages=test_prompt, max_tokens=10)
                        resp = r.choices[0].message.content.strip()[:60]
                        print(f"[Teste de Conexão] Resposta recebida: {resp}")
                        self.done.emit(True, resp)
                    elif self._provider == "github_models":
                        print("[Teste de Conexão] Enviando requisição para GitHub Models (gpt-4o-mini)...")
                        client = OpenAI(base_url="https://models.inference.ai.azure.com", api_key=self._key)
                        r = client.chat.completions.create(model="gpt-4o-mini", messages=test_prompt, max_tokens=10)
                        resp = r.choices[0].message.content.strip()[:60]
                        print(f"[Teste de Conexão] Resposta recebida: {resp}")
                        self.done.emit(True, resp)
                    elif self._provider == "openai":
                        print("[Teste de Conexão] Enviando requisição para OpenAI (gpt-4o-mini)...")
                        client = OpenAI(api_key=self._key)
                        r = client.chat.completions.create(model="gpt-4o-mini", messages=test_prompt, max_tokens=10)
                        resp = r.choices[0].message.content.strip()[:60]
                        print(f"[Teste de Conexão] Resposta recebida: {resp}")
                        self.done.emit(True, resp)
                    else:
                        print(f"[Teste de Conexão] Provedor desconhecido: {self._provider}")
                        self.done.emit(False, "Provedor desconhecido.")
                except Exception as e:
                    print(f"[Teste de Conexão] Erro: {e}")
                    self.done.emit(False, str(e)[:120])

        def _on_done(ok, msg):
            self._btn_test_conn.setEnabled(True)
            self._btn_test_conn.setText(tr("btn_test_conn"))
            if ok:
                self._lbl_conn_status.setText(tr("test_conn_ok", msg))
                self._lbl_conn_status.setStyleSheet("font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none; color: #34d399;")
            else:
                self._lbl_conn_status.setText(tr("test_conn_fail", msg))
                self._lbl_conn_status.setStyleSheet("font-size: 11px; font-family: 'Segoe UI', sans-serif; background: transparent; border: none; color: #f87171;")

        self._test_worker = _TestWorker(provider, key_val.split(",")[0].strip())
        self._test_worker.done.connect(_on_done)
        self._test_worker.start()

    def _build_page_whisper(self):
        scroll, page = self._make_scroll_page()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(28, 28, 28, 28)
        layout.setSpacing(0)

        layout.addWidget(self._section_label(tr("sec_local_transcription")))
        layout.addSpacing(6)

        note = QFrame()
        note.setStyleSheet("""
            QFrame {
                background: rgba(6,182,212,10);
                border: 1px solid rgba(6,182,212,40);
                border-radius: 8px;
            }
        """)
        note_l = QVBoxLayout(note)
        note_l.setContentsMargins(14, 10, 14, 10)
        lbl_n = QLabel(tr("whisper_note"))
        lbl_n.setWordWrap(True)
        lbl_n.setStyleSheet("color: rgba(255,255,255,160); font-size: 11px; background: transparent; border: none;")
        note_l.addWidget(lbl_n)
        layout.addWidget(note)
        layout.addSpacing(14)

        layout.addWidget(self._field_label(tr("field_whisper_model")))
        self.combo_whisper = self._combo(
            ["tiny", "base", "small", "medium", "large-v2", "large-v3", "large-v3-turbo"],
            self.config_manager.get("whisper", {}).get("model_size", "base")
        )
        layout.addWidget(self.combo_whisper)
        layout.addWidget(self._desc(tr("desc_whisper_model")))
        layout.addSpacing(10)

        layout.addWidget(self._field_label(tr("field_whisper_device")))
        self.combo_whisper_device = self._combo(["cpu", "cuda"], self.config_manager.get("whisper", {}).get("device", "cpu"))
        layout.addWidget(self.combo_whisper_device)
        layout.addWidget(self._desc(tr("desc_whisper_device")))

        layout.addStretch()
        self.pages_stack.addWidget(scroll)

    def _build_page_hotkeys(self):
        scroll, page = self._make_scroll_page()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(28, 28, 28, 28)
        layout.setSpacing(0)

        layout.addWidget(self._section_label(tr("sec_keyboard_shortcuts")))
        layout.addSpacing(6)

        hint = QFrame()
        hint.setStyleSheet("""
            QFrame {
                background: rgba(255,255,255,4);
                border: 1px solid rgba(255,255,255,12);
                border-radius: 8px;
            }
        """)
        hint_l = QVBoxLayout(hint)
        hint_l.setContentsMargins(14, 10, 14, 10)
        lbl_h = QLabel(tr("hotkey_hint"))
        lbl_h.setWordWrap(True)
        lbl_h.setStyleSheet("color: rgba(255,255,255,150); font-size: 11px; background: transparent; border: none;")
        hint_l.addWidget(lbl_h)
        layout.addWidget(hint)
        layout.addSpacing(16)

        for field_name, label, config_key, default in [
            ("txt_hotkey", tr("hotkey_dictation_label"), "hotkey", "<ctrl>+<shift>+<space>"),
            ("txt_hotkey_translation", tr("hotkey_translation_label"), "hotkey_translation", "<ctrl>+<shift>+<y>"),
            ("txt_hotkey_pesquisa", tr("hotkey_search_label"), "hotkey_pesquisa", "<ctrl>+<shift>+<u>"),
        ]:
            layout.addWidget(self._field_label(label))
            le, row = self.create_hotkey_row(label, config_key, default)
            setattr(self, field_name, le)
            layout.addLayout(row)
            layout.addSpacing(12)

        layout.addStretch()
        self.pages_stack.addWidget(scroll)

    def _open_wizard(self):
        self.fade_out_and_close(False)
        wizard = SetupWizard(self.config_manager, self.app, self.parent(), allow_close=True)
        wizard.exec()

    # Window Dragging
    def mousePressEvent(self, event):
        if event.button() == Qt.LeftButton and event.position().y() < 48:
            self.drag_position = event.globalPosition().toPoint() - self.frameGeometry().topLeft()
            event.accept()
        else:
            self.drag_position = None
            super().mousePressEvent(event)

    def mouseMoveEvent(self, event):
        if event.buttons() == Qt.LeftButton and self.drag_position is not None:
            self.move(event.globalPosition().toPoint() - self.drag_position)
            event.accept()
        else:
            super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event):
        self.drag_position = None
        super().mouseReleaseEvent(event)

    def fade_out_and_close(self, accept_dialog=False):
        geom = self.geometry()
        self.pos_anim = QPropertyAnimation(self, b"geometry")
        self.pos_anim.setDuration(200)
        self.pos_anim.setStartValue(geom)
        self.pos_anim.setEndValue(QRect(geom.x(), geom.y() + 12, geom.width(), geom.height()))
        self.pos_anim.setEasingCurve(QEasingCurve.InCubic)
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(200)
        self.opacity_anim.setStartValue(self.windowOpacity())
        self.opacity_anim.setEndValue(0.0)
        self.anim_group = QParallelAnimationGroup()
        self.anim_group.addAnimation(self.pos_anim)
        self.anim_group.addAnimation(self.opacity_anim)
        if accept_dialog:
            self.anim_group.finished.connect(self.accept)
        else:
            self.anim_group.finished.connect(self.reject)
        self.anim_group.start()

    def save_settings(self):
        self.config_manager.set("provider", self.combo_provider.currentText())
        self.config_manager.set("active_style", self.combo_style.currentText())
        pv_raw = self.combo_prompt_version.currentText()
        self.config_manager.set("prompt_version", "v2" if pv_raw.startswith("v2") else "v1")
        self.config_manager.set_api_key("gemini", self.txt_gemini.text().strip())
        self.config_manager.set_api_key("openai", self.txt_openai.text().strip())
        self.config_manager.set_api_key("groq", self.txt_groq.text().strip())
        self.config_manager.set_api_key("github_models", self.txt_github_models.text().strip())
        self.config_manager.set("tavily_api_key", self.txt_tavily.text().strip())
        reverse_mode_map = {"Ditado": "ditado", "Tradução": "traducao", "Pesquisa": "pesquisa"}
        self.config_manager.set("operation_mode", reverse_mode_map.get(self.combo_mode.currentText(), "ditado"))
        self.config_manager.set("translation_target", self.combo_target_lang.currentText())
        reverse_search_provider = {
            "Groq (recomendado — acesso à web)": "groq",
            "Gemini (Google Search)": "gemini",
            "Mesmo do ditado": "auto"
        }
        self.config_manager.set("search_provider", reverse_search_provider.get(self.combo_search_provider.currentText(), "groq"))
        reverse_search_model = {
            "groq/compound (web em tempo real)": "groq/compound",
            "groq/compound-mini (mais rápido)": "groq/compound-mini",
            "llama-3.3-70b-versatile (sem web)": "llama-3.3-70b-versatile"
        }
        self.config_manager.set("search_model", reverse_search_model.get(self.combo_search_model.currentText(), "groq/compound"))
        self.config_manager.set("hotkey", self.txt_hotkey.text().strip())
        self.config_manager.set("hotkey_translation", self.txt_hotkey_translation.text().strip())
        self.config_manager.set("hotkey_pesquisa", self.txt_hotkey_pesquisa.text().strip())
        whisper_cfg = self.config_manager.get("whisper", {})
        whisper_cfg["model_size"] = self.combo_whisper.currentText()
        whisper_cfg["device"] = self.combo_whisper_device.currentText()
        self.config_manager.set("whisper", whisper_cfg)
        self.config_manager.set("start_with_windows", self.chk_startup.isChecked())
        set_run_at_startup(self.chk_startup.isChecked())
        self.config_manager.set("mute_on_record", self.chk_mute.isChecked())
        self.config_manager.set("keys_password", self.txt_keys_password.text().strip())
        # Animate save button to "Salvo!" then close
        btn_save = self.findChild(QPushButton, "btn_save")
        if btn_save:
            btn_save.setEnabled(False)
            btn_save.setText(tr("saved"))
            btn_save.setStyleSheet("""
                QPushButton {
                    background: #34d399;
                    border: none;
                    border-radius: 7px;
                    color: #ffffff;
                    font-size: 11px; font-weight: 700;
                    font-family: 'Segoe UI', sans-serif;
                    padding: 0 18px;
                }
            """)
            QTimer.singleShot(480, lambda: self.fade_out_and_close(True))
        else:
            self.fade_out_and_close(True)

    def _set_update_btn(self, text, color=None, enabled=True):  # noqa: E501
        btn = self.findChild(QPushButton, "btn_check_update")
        if not btn:
            return
        btn.setEnabled(enabled)
        btn.setText(text)
        if color:
            btn.setStyleSheet(f"""
                QPushButton {{
                    background: transparent;
                    border: 1px solid {color};
                    border-radius: 7px;
                    color: {color};
                    font-size: 11px; font-weight: 600;
                    font-family: 'Segoe UI', sans-serif;
                    padding: 0 14px;
                }}
            """)
        else:
            btn.setStyleSheet("""
                QPushButton {
                    background: transparent;
                    border: 1px solid rgba(139,92,246,80);
                    border-radius: 7px;
                    color: rgba(139,92,246,200);
                    font-size: 11px; font-weight: 600;
                    font-family: 'Segoe UI', sans-serif;
                    padding: 0 14px;
                }
                QPushButton:hover { background: rgba(139,92,246,20); border-color: #8b5cf6; color: #fff; }
            """)

    def check_updates_manually(self):
        self._set_update_btn(tr("checking_update"), enabled=False)
        self.manual_checker = UpdateCheckerWorker()
        self.manual_checker.update_available.connect(self.on_manual_update_available)
        self.manual_checker.no_update_found.connect(self.on_manual_no_update)
        self.manual_checker.error.connect(self.on_manual_update_error)
        self.manual_checker.start()

    def on_manual_update_available(self, version, download_url):
        self._set_update_btn(tr("update_available", version), color="#34d399")
        QTimer.singleShot(4000, lambda: self._set_update_btn(tr("btn_update")))
        self.update_dialog = UpdateDialog(version, download_url, self)
        self.update_dialog.show()

    def on_manual_no_update(self):
        self._set_update_btn(tr("up_to_date"), color="#34d399")
        QTimer.singleShot(3000, lambda: self._set_update_btn(tr("btn_update")))

    def on_manual_update_error(self, err_msg):
        self._set_update_btn(tr("update_error"), color="#f87171")
        QTimer.singleShot(4000, lambda: self._set_update_btn(tr("btn_update")))
        print(f"Erro ao verificar atualizações: {err_msg}")



# Sound Wave Visualizer Widget for Live Mic Feedback (Siri-like liquid waves)
class SoundVisualizer(QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedSize(50, 24)
        self.amplitude = 0.0
        self.target_amplitude = 0.0
        self.phase = 0.0
        
        # Smooth interpolation timer (60 FPS)
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_animation)
        self.timer.start(16)
        
    def set_amplitude(self, value):
        # Scale and clip amplitude (increased sensitivity from 45.0 to 75.0)
        self.target_amplitude = min(max(value * 75.0, 0.0), 1.0)
        
    def update_animation(self):
        # Smooth interpolation
        self.amplitude += (self.target_amplitude - self.amplitude) * 0.2
        # Idle wave phase
        self.phase += 0.12
        if self.phase > 6.283:
            self.phase -= 6.283
        self.update()
        
    def paintEvent(self, event):
        import math
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)
        
        w = self.width()
        h = self.height()
        mid_y = h / 2.0
        
        # 3 overlapping vibrant gradient liquid waves
        wave_configs = [
            (QColor(139, 92, 246, 200), 0.95, 0.0, 0.08), # Vibrant Purple
            (QColor(236, 72, 153, 160), 0.70, 1.8, 0.12), # Vibrant Pink
            (QColor(6, 182, 212, 120), 0.45, 3.5, 0.06)   # Cyan
        ]
        
        from PySide6.QtGui import QPainterPath
        for color, amp_scale, phase_offset, freq in wave_configs:
            path = QPainterPath()
            path.moveTo(0, mid_y)
            
            # Determine maximum wave height scaled by input amplitude
            max_amp = (self.amplitude * 0.85 + 0.15) * amp_scale * (h / 2.2)
            
            for x in range(0, w + 1, 2):
                # Bell-shaped curve to taper edges cleanly at x=0 and x=w
                taper = 4.0 * (x / w) * (1.0 - (x / w))
                y = mid_y + taper * max_amp * math.sin(x * freq + self.phase + phase_offset)
                path.lineTo(x, y)
                
            painter.setPen(QPen(color, 2.0))
            painter.setBrush(Qt.NoBrush)
            path_obj = path
            painter.drawPath(path_obj)


class LoadingSpinner(QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedSize(50, 24)
        self.position = 0.0
        self.direction = 1
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_animation)
        self.timer.start(16) # ~60 FPS
        
    def update_animation(self):
        # Sweep back and forth
        self.position += 0.025 * self.direction
        if self.position >= 1.0:
            self.position = 1.0
            self.direction = -1
        elif self.position <= 0.0:
            self.position = 0.0
            self.direction = 1
        self.update()
        
    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)
        
        w = self.width()
        h = self.height()
        mid_y = h / 2.0
        
        # Draw background track
        painter.setPen(Qt.NoPen)
        painter.setBrush(QBrush(QColor(255, 255, 255, 15)))
        painter.drawRoundedRect(0, int(mid_y - 2), w, 4, 2, 2)
        
        # Draw sweeping neon laser gradient pill
        from PySide6.QtGui import QLinearGradient
        grad = QLinearGradient(0, 0, w, 0)
        
        pos = self.position
        # Make the glowing segment move smoothly
        grad.setColorAt(max(0.0, pos - 0.25), QColor(245, 158, 11, 0))
        grad.setColorAt(pos, QColor(251, 191, 36, 255))
        grad.setColorAt(min(1.0, pos + 0.25), QColor(245, 158, 11, 0))
        
        painter.setBrush(QBrush(grad))
        painter.drawRoundedRect(0, int(mid_y - 2), w, 4, 2, 2)


# Custom frame with sweeping orange/amber shimmer highlight
class GlowFrame(QFrame):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.is_processing = False
        self.current_state = "listening"
        self.shimmer_pos = 0.1
        self.shimmer_dir = 1
        self.wave_phase = 0.0
        self.amplitude = 0.0
        self.target_amplitude = 0.0
        self.single_sweep = 1.5
        self.done_sweep_pos = -0.2
        self.done_sweep_dir = 1
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_shimmer)
        self.timer.start(16) # ~60 FPS
        
    def set_amplitude(self, amp):
        # Scale and clip target amplitude
        self.target_amplitude = min(max(amp * 80.0, 0.0), 1.0)
        
    def update_shimmer(self):
        # Slower wave movement for elegant siri wave (0.045)
        self.wave_phase += 0.045
        if self.wave_phase > 6.283:
            self.wave_phase -= 6.283
            
        # Smoothly interpolate amplitude to prevent jumps
        self.amplitude += (self.target_amplitude - self.amplitude) * 0.2
            
        if self.is_processing:
            # Shift shimmer back and forth (narrower limits to prevent dark pause)
            self.shimmer_pos += 0.02 * self.shimmer_dir
            if self.shimmer_pos >= 0.9:
                self.shimmer_pos = 0.9
                self.shimmer_dir = -1
            elif self.shimmer_pos <= 0.1:
                self.shimmer_pos = 0.1
                self.shimmer_dir = 1
                
        if self.current_state in ["done", "error"]:
            self.done_sweep_pos += 0.006 * self.done_sweep_dir
            if self.done_sweep_pos >= 1.2:
                self.done_sweep_pos = 1.2
                self.done_sweep_dir = -1
            elif self.done_sweep_pos <= -0.2:
                self.done_sweep_pos = -0.2
                self.done_sweep_dir = 1
                
        self.update()
            
    def paintEvent(self, event):
        from PySide6.QtGui import QLinearGradient
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)
        
        w = self.width()
        h = self.height()
        rect = self.rect()
        
        bg_color = QColor(10, 10, 10, 235)
        
        if self.current_state == "listening":
            # Draw standard background
            painter.setPen(Qt.NoPen)
            painter.setBrush(QBrush(bg_color))
            painter.drawRoundedRect(rect, 18, 18)
            
            # Draw Siri waves in the background of the modal
            mid_y = h / 2.0
            wave_configs = [
                (QColor(139, 92, 246, 55), 0.90, 0.0, 0.08),  # Purple
                (QColor(236, 72, 153, 40), 0.65, 1.8, 0.12),  # Pink
                (QColor(6, 182, 212, 25), 0.40, 3.5, 0.06)    # Cyan
            ]
            from PySide6.QtGui import QPainterPath
            import math
            for color, amp_scale, phase_offset, freq in wave_configs:
                path = QPainterPath()
                path.moveTo(0, mid_y)
                
                # Scale wave height by microphone input amplitude
                max_amp = (self.amplitude * 0.85 + 0.15) * amp_scale * (h / 2.3)
                
                for x in range(0, w + 1, 2):
                    # Siri wave across entire width with gentle taper at very edges
                    taper = 0.35 + 0.65 * math.sin((x / w) * math.pi)
                    y = mid_y + taper * max_amp * math.sin(x * freq + self.wave_phase + phase_offset)
                    path.lineTo(x, y)
                    
                painter.setPen(QPen(color, 1.5))
                painter.setBrush(Qt.NoBrush)
                painter.drawPath(path)
                
            # Slow, breathing purple border glow (frequency scaled down by 0.6 for calming rhythm)
            breath = 0.5 + 0.5 * math.sin(self.wave_phase * 0.6)
            glow_intensity = 0.45 * breath + 0.55 * self.amplitude
            border_opacity = int(90 + 155 * glow_intensity)
            
            pen = QPen(QColor(139, 92, 246, border_opacity), 1.5 + 0.8 * glow_intensity)
            painter.setPen(pen)
            painter.setBrush(Qt.NoBrush)
            painter.drawRoundedRect(rect.adjusted(1, 1, -1, -1), 18, 18)
            
        else:
            # Draw standard background and borders for other states
            if self.current_state == "done":
                border_color = QColor(16, 185, 129, 160) # Green
            elif self.current_state == "error":
                border_color = QColor(239, 68, 68, 160) # Red
            else:
                border_color = QColor(255, 255, 255, 30) # Default
                
            if self.is_processing:
                # Draw filled background
                painter.setPen(Qt.NoPen)
                painter.setBrush(QBrush(bg_color))
                painter.drawRoundedRect(rect, 18, 18)
                
                pos = self.shimmer_pos
                
                # Smooth blending near boundaries (keeps minimum glow of 0.2 so it's never empty)
                fade_factor = 1.0
                if pos < 0.2:
                    fade_factor = max(0.2, (pos + 0.1) / 0.3)
                elif pos > 0.8:
                    fade_factor = max(0.2, (1.1 - pos) / 0.3)
                
                # Draw inside shimmer/glow highlight
                shimmer_grad = QLinearGradient(0, 0, w, 0)
                shimmer_grad.setColorAt(max(0.0, min(1.0, pos - 0.3)), QColor(245, 158, 11, 0))
                shimmer_grad.setColorAt(max(0.0, min(1.0, pos)), QColor(245, 158, 11, int(22 * fade_factor)))
                shimmer_grad.setColorAt(max(0.0, min(1.0, pos + 0.3)), QColor(245, 158, 11, 0))
                
                painter.setBrush(QBrush(shimmer_grad))
                painter.drawRoundedRect(rect, 18, 18)
                
                # Draw sweeping orange/amber gradient border
                border_grad = QLinearGradient(0, 0, w, 0)
                border_grad.setColorAt(max(0.0, min(1.0, pos - 0.25)), QColor(255, 255, 255, int(30 * fade_factor)))
                border_grad.setColorAt(max(0.0, min(1.0, pos)), QColor(245, 158, 11, int(200 * fade_factor)))
                border_grad.setColorAt(max(0.0, min(1.0, pos + 0.25)), QColor(255, 255, 255, int(30 * fade_factor)))
                
                pen = QPen(border_grad, 1.5)
                painter.setPen(pen)
                painter.setBrush(Qt.NoBrush)
                painter.drawRoundedRect(rect.adjusted(1, 1, -1, -1), 18, 18)
            else:
                # Draw standard background
                painter.setPen(Qt.NoPen)
                painter.setBrush(QBrush(bg_color))
                painter.drawRoundedRect(rect, 18, 18)
                
                # Rotating border highlight
                import math
                angle = self.wave_phase * 0.45
                dx = math.cos(angle)
                dy = math.sin(angle)
                
                cx = w / 2.0
                cy = h / 2.0
                
                # Project the gradient direction based on rotation angle
                x1 = cx - dx * (w / 1.5)
                y1 = cy - dy * (h / 1.5)
                x2 = cx + dx * (w / 1.5)
                y2 = cy + dy * (h / 1.5)
                
                border_grad = QLinearGradient(x1, y1, x2, y2)
                
                # Colors based on state
                base_color = QColor(16, 185, 129, 30) if self.current_state == "done" else QColor(239, 68, 68, 20)
                active_color = QColor(16, 185, 129, 225) if self.current_state == "done" else QColor(239, 68, 68, 180)
                glow_color = QColor(16, 185, 129, 14) if self.current_state == "done" else QColor(239, 68, 68, 10)
                
                # Gradient stops for the border: a bright segment moving around
                border_grad.setColorAt(0.0, base_color)
                border_grad.setColorAt(0.35, base_color)
                border_grad.setColorAt(0.5, active_color)
                border_grad.setColorAt(0.65, base_color)
                border_grad.setColorAt(1.0, base_color)
                
                # Inside background sweep glow (also rotating to match)
                shimmer_grad = QLinearGradient(x1, y1, x2, y2)
                shimmer_grad.setColorAt(0.0, QColor(0, 0, 0, 0))
                shimmer_grad.setColorAt(0.35, QColor(0, 0, 0, 0))
                shimmer_grad.setColorAt(0.5, glow_color)
                shimmer_grad.setColorAt(0.65, QColor(0, 0, 0, 0))
                shimmer_grad.setColorAt(1.0, QColor(0, 0, 0, 0))
                
                painter.setBrush(QBrush(shimmer_grad))
                painter.drawRoundedRect(rect, 18, 18)
                
                # Gentle breathing for border width
# Frameless Glassmorphism Overlay (Sleek pill with slide and fade transitions)
class FloatingOverlay(QWidget):
    ACTIVE_STATES = frozenset({"listening", "processing"})

    def __init__(self, parent=None):
        super().__init__(parent)
        self.current_target_w = 260
        self.current_target_h = 68
        self.is_showing = False
        self._current_state = None
        self.anim_group = None
        self.size_anim_group = None
        self.frame_anim = None
        self.opacity_anim = None
        self.text_fade_group = None
        self._dismiss_timer = QTimer(self)
        self._dismiss_timer.setSingleShot(True)
        self._dismiss_timer.timeout.connect(self.fade_out)
        self.init_ui()

    def _stop_animations(self):
        """Stop all running geometry/opacity animations safely."""
        for group in (self.anim_group, self.size_anim_group):
            try:
                if group is not None and group.state() == QAbstractAnimation.State.Running:
                    group.stop()
            except RuntimeError:
                pass
        try:
            if hasattr(self, 'opacity_anim') and self.opacity_anim is not None:
                if self.opacity_anim.state() == QAbstractAnimation.State.Running:
                    self.opacity_anim.stop()
        except RuntimeError:
            self.opacity_anim = None
        try:
            if hasattr(self, 'opacity_anim_in') and self.opacity_anim_in is not None:
                if self.opacity_anim_in.state() == QAbstractAnimation.State.Running:
                    self.opacity_anim_in.stop()
        except RuntimeError:
            self.opacity_anim_in = None

    def _stop_text_animations(self):
        """Stop running text fade animations and ensure opacity is 1.0 safely."""
        try:
            if self.text_fade_group is not None and self.text_fade_group.state() == QAbstractAnimation.State.Running:
                self.text_fade_group.stop()
        except RuntimeError:
            pass
        self.text_fade_group = None
        self.header_opacity.setOpacity(1.0)
        self.content_opacity.setOpacity(1.0)

    def _cancel_dismiss(self):
        self._dismiss_timer.stop()

    def init_ui(self):
        # Frameless, translucent, floating window settings
        self.setWindowFlags(Qt.WindowStaysOnTopHint | Qt.FramelessWindowHint | Qt.Tool | Qt.SubWindow)
        self.setAttribute(Qt.WA_TranslucentBackground, True)
        self.setAttribute(Qt.WA_ShowWithoutActivating, True)
        self.setFixedSize(500, 450)

        # Base Frame container for glassmorphism styling
        self.main_frame = GlowFrame(self)
        self.main_frame.setObjectName("main_frame")

        # Drop shadow effect
        self.shadow_effect = QGraphicsDropShadowEffect(self)
        self.shadow_effect.setBlurRadius(16)
        self.shadow_effect.setColor(QColor(0, 0, 0, 180))
        self.shadow_effect.setOffset(0, 2)
        self.main_frame.setGraphicsEffect(self.shadow_effect)

        # Layout inside the frame
        layout = QHBoxLayout(self.main_frame)
        layout.setContentsMargins(16, 8, 16, 8)
        layout.setSpacing(10)

        # Sound wave visualizer (Siri Waves)
        self.visualizer = SoundVisualizer(self.main_frame)
        layout.addWidget(self.visualizer, 0, Qt.AlignVCenter)

        # Loading spinner
        self.spinner = LoadingSpinner(self.main_frame)
        layout.addWidget(self.spinner, 0, Qt.AlignVCenter)

        # Static state indicator dot (done/error)
        self.indicator = QLabel(self.main_frame)
        self.indicator.setFixedSize(8, 8)
        self.indicator.setStyleSheet("background-color: #ffffff; border-radius: 4px;")
        layout.addWidget(self.indicator, 0, Qt.AlignVCenter)

        # Vertical Text Layout
        text_layout = QVBoxLayout()
        text_layout.setSpacing(2)
        text_layout.setContentsMargins(0, 0, 0, 0)
        text_layout.addStretch()

        self.label_header = QLabel(self.main_frame)
        header_font = QFont("Segoe UI", 7)
        header_font.setBold(True)
        self.label_header.setFont(header_font)
        self.label_header.setStyleSheet("color: rgba(255, 255, 255, 120); letter-spacing: 1px;")
        self.label_header.setAlignment(Qt.AlignLeft | Qt.AlignVCenter)
        self.label_header.setFixedHeight(0)
        text_layout.addWidget(self.label_header)

        self.label_content = QLabel("GRAVANDO...", self.main_frame)
        content_font = QFont("Segoe UI", 8)
        content_font.setBold(True)
        self.label_content.setFont(content_font)
        self.label_content.setStyleSheet("color: #ffffff; letter-spacing: 0.5px;")
        self.label_content.setWordWrap(False)
        self.label_content.setAlignment(Qt.AlignCenter)
        self.label_content.setFixedHeight(20)
        text_layout.addWidget(self.label_content)

        text_layout.addStretch()
        layout.addLayout(text_layout, 1)

        # NO QGraphicsOpacityEffect on labels — it causes rendering outside parent frame bounds.
        # Window-level opacity fade (windowOpacity) handles all fade animations.
        # Dummy objects to avoid breaking code that references header_opacity/content_opacity:
        class _NoOpOpacity:
            def setOpacity(self, v): pass
            def opacity(self): return 1.0
            def state(self): return 0
        self.header_opacity = _NoOpOpacity()
        self.content_opacity = _NoOpOpacity()

        # Initial widget visibility
        self.visualizer.hide()
        self.spinner.hide()
        self.indicator.hide()
        self.label_header.hide()

    def get_centered_geometry(self, width, height):
        screen = QApplication.primaryScreen().availableGeometry()
        x = screen.x() + (screen.width() - width) // 2
        y = screen.y() + screen.height() - height - 20
        return QRect(x, y, width, height)

    def get_frame_geometry(self, target_w, target_h):
        """Returns the geometry of the inner frame, anchored bottom-center inside the fixed window."""
        frame_w = target_w - 20
        frame_h = target_h - 20
        frame_x = (self.width() - frame_w) // 2
        frame_y = (self.height() - 10) - frame_h
        return QRect(frame_x, frame_y, frame_w, frame_h)

    def center_on_screen(self):
        """Positions the overlay bottom-center on the primary available screen."""
        geom = self.get_centered_geometry(self.width(), self.height())
        self.move(geom.topLeft())

    def animate_to_size(self, target_width, target_height):
        """Directly resizes the inner main_frame inside the fixed window with a smooth animation."""
        self._stop_animations()
        self.current_target_w = target_width
        self.current_target_h = target_height
        target_geom = self.get_frame_geometry(target_width, target_height)
        
        self.frame_anim = QPropertyAnimation(self.main_frame, b"geometry")
        self.frame_anim.setDuration(160)
        self.frame_anim.setStartValue(self.main_frame.geometry())
        self.frame_anim.setEndValue(target_geom)
        self.frame_anim.setEasingCurve(QEasingCurve.OutCubic)
        
        self.anim_group = QParallelAnimationGroup()
        self.anim_group.addAnimation(self.frame_anim)
        self.anim_group.start()
        
        if self.main_frame.layout():
            self.main_frame.layout().activate()

    def show_state(self, state, text=None):
        """Updates the UI look and dimensions based on state."""
        self._current_state = state
        if state in self.ACTIVE_STATES:
            self._cancel_dismiss()
        self._stop_animations()
        self.apply_state_change(state, text)

    def apply_state_change(self, state, text=None):
        """Applies layout geometry changes and state indicators."""
        is_text_display = False
        display_text = text if text else ""
        header_text = ""

        if state == "done" and text and text not in ["CONCLUÍDO!", "COPIADO!"]:
            if text.startswith("Traduzido: "):
                header_text = "TRADUÇÃO COPIADA"
                display_text = text[len("Traduzido: "):]
            elif text.startswith("Sem IA / Tradução Falhou: "):
                header_text = "SEM CHAVE / TRADUÇÃO FALHOU"
                display_text = text[len("Sem IA / Tradução Falhou: "):]
            elif text.startswith("Sem IA / Texto Cru: "):
                header_text = "TEXTO CRU COPIADO (SEM IA)"
                display_text = text[len("Sem IA / Texto Cru: "):]
            elif text.startswith("Pesquisando: "):
                header_text = "PESQUISANDO NO GOOGLE"
                display_text = text[len("Pesquisando: "):]
            elif text.startswith("Copiado: "):
                header_text = "TEXTO COPIADO"
                display_text = text[len("Copiado: "):]
            else:
                header_text = "TEXTO COPIADO"
                display_text = text
        elif state == "error" and text and text not in ["CONCLUÍDO!", "COPIADO!"]:
            header_text = "ERRO!"
            display_text = text[len("Erro: "):] if text.startswith("Erro: ") else text

        # Determine target dimensions
        from PySide6.QtGui import QFontMetrics
        if state in ["listening", "processing"]:
            # Dynamic width based on text
            label_text = text.upper() if text else ("GRAVANDO" if state == "listening" else "POLINDO")
            if not label_text.endswith("..."):
                label_text += "..."
            target_font = QFont("Segoe UI", 8)
            target_font.setBold(True)
            metrics = QFontMetrics(target_font)
            text_w = metrics.horizontalAdvance(label_text)
            frame_w = text_w + 72
            target_w = max(frame_w + 20, 140)
            target_w = min(target_w, 360)
            target_h = 60
        elif header_text or (text and text not in ["CONCLUÍDO!", "COPIADO!"] and state in ["done", "error"]):
            is_text_display = True
            max_len = 120
            if len(display_text) > max_len:
                display_text = display_text[:max_len].strip() + "..."
            target_w = 380
            target_h = 100
        else:
            target_w = 200
            target_h = 60

        self.current_target_w = target_w
        self.current_target_h = target_h

        # Lifecycle (Task Fase 3): only fade_in when hidden
        if self.is_showing or self.isVisible():
            self._stop_animations()
            
            # Start a brief fade out of the window opacity
            self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
            self.opacity_anim.setDuration(70)
            self.opacity_anim.setStartValue(self.windowOpacity())
            self.opacity_anim.setEndValue(0.25)
            self.opacity_anim.setEasingCurve(QEasingCurve.OutQuad)
            
            def on_fade_down():
                # Apply visual states
                self.main_frame.current_state = state
                if state in ["done", "error"]:
                    self.main_frame.done_sweep_pos = -0.2
                    self.main_frame.done_sweep_dir = 1
                
                # Height settings for labels
                if state in ["listening", "processing"]:
                    self.label_header.setFixedHeight(0)
                    self.label_content.setFixedHeight(20)
                elif is_text_display:
                    self.label_header.setFixedHeight(16)
                    self.label_content.setFixedHeight(42)
                else:
                    self.label_header.setFixedHeight(0)
                    self.label_content.setFixedHeight(20)
                
                self.update_text_with_fade(state, text, header_text, display_text)
                
                # Animate geometry smoothly
                self.animate_to_size(target_w, target_h)
                
                # Fade window opacity back in
                self.opacity_anim_in = QPropertyAnimation(self, b"windowOpacity")
                self.opacity_anim_in.setDuration(160)
                self.opacity_anim_in.setStartValue(0.25)
                self.opacity_anim_in.setEndValue(1.0)
                self.opacity_anim_in.setEasingCurve(QEasingCurve.OutCubic)
                
                if self.anim_group:
                    self.anim_group.addAnimation(self.opacity_anim_in)
                else:
                    self.anim_group = QParallelAnimationGroup()
                    self.anim_group.addAnimation(self.opacity_anim_in)
                    self.anim_group.start()
                
                if self.main_frame.layout():
                    self.main_frame.layout().activate()
            
            self.opacity_anim.finished.connect(on_fade_down)
            self.opacity_anim.start()
        else:
            # First show: apply instantly and fade_in
            self.main_frame.current_state = state
            if state in ["done", "error"]:
                self.main_frame.done_sweep_pos = -0.2
                self.main_frame.done_sweep_dir = 1
                
            if state in ["listening", "processing"]:
                self.label_header.setFixedHeight(0)
                self.label_content.setFixedHeight(20)
            elif is_text_display:
                self.label_header.setFixedHeight(16)
                self.label_content.setFixedHeight(42)
            else:
                self.label_header.setFixedHeight(0)
                self.label_content.setFixedHeight(20)
                
            self.update_text_with_fade(state, text, header_text, display_text)
            
            geom = self.get_frame_geometry(target_w, target_h)
            self.main_frame.setGeometry(geom)
            if self.main_frame.layout():
                self.main_frame.layout().activate()
            self.fade_in()

        if state in ["done", "error"]:
            delay = 3500 if is_text_display else 1500
            if state == "error":
                delay = 2500
            self._cancel_dismiss()
            self._dismiss_timer.start(delay)

    def update_text_with_fade(self, state, text, header_text, display_text):
        """Apply state widgets immediately. Window-level opacity handles the visual fade."""
        if self.text_fade_group is not None and hasattr(self.text_fade_group, 'state'):
            try:
                if self.text_fade_group.state() == QAbstractAnimation.State.Running:
                    self.text_fade_group.stop()
            except Exception:
                pass
        self.text_fade_group = None
        self.update_state_widgets(state, text, header_text, display_text)

    def update_state_widgets(self, state, text=None, header_text=None, display_text=None):
        """Hides and shows the correct visual indicator according to the state."""
        if state == "listening":
            self.main_frame.is_processing = False
            self.shadow_effect.setColor(QColor(139, 92, 246, 110))
            self.shadow_effect.setBlurRadius(24)
            
            self.label_header.hide()
            label_text = text.upper() if text else tr("overlay_recording")
            if not label_text.endswith("..."):
                label_text += "..."
            self.label_content.setText(label_text)
            self.label_content.setWordWrap(False)
            self.label_content.setAlignment(Qt.AlignCenter)
            
            # Setup font via API to override stylesheet inheritance cleanly
            font = QFont("Segoe UI", 8)
            font.setBold(True)
            self.label_content.setFont(font)
            self.label_content.setStyleSheet("color: #ffffff; letter-spacing: 0.5px;")
            
            self.visualizer.hide()
            self.spinner.hide()
            self.indicator.hide()
            
        elif state == "processing":
            self.main_frame.is_processing = True
            self.shadow_effect.setColor(QColor(245, 158, 11, 90))
            self.shadow_effect.setBlurRadius(24)
            
            self.label_header.hide()
            label_text = text.upper() if text else tr("overlay_processing")
            if not label_text.endswith("..."):
                label_text += "..."
            self.label_content.setText(label_text)
            self.label_content.setWordWrap(False)
            self.label_content.setAlignment(Qt.AlignCenter)
            
            font = QFont("Segoe UI", 8)
            font.setBold(True)
            self.label_content.setFont(font)
            self.label_content.setStyleSheet("color: #ffffff; letter-spacing: 0.5px;")
            
            self.visualizer.hide()
            self.spinner.hide()
            self.indicator.hide()
            
        elif state == "done":
            self.main_frame.is_processing = False
            self.shadow_effect.setColor(QColor(16, 185, 129, 90))
            self.shadow_effect.setBlurRadius(24)
            self.label_content.setAlignment(Qt.AlignLeft | Qt.AlignVCenter)
            self.label_header.setAlignment(Qt.AlignLeft | Qt.AlignVCenter)
            
            if header_text or (text and text not in ["CONCLUÍDO!", "COPIADO!"]):
                if not header_text or not display_text:
                    if text.startswith("Traduzido: "):
                        header_text = tr("overlay_translated_header")
                        display_text = text[len("Traduzido: "):]
                    elif text.startswith("Sem IA / Tradução Falhou: "):
                        header_text = tr("overlay_no_key_translation")
                        display_text = text[len("Sem IA / Tradução Falhou: "):]
                    elif text.startswith("Sem IA / Texto Cru: "):
                        header_text = tr("overlay_no_key_raw")
                        display_text = text[len("Sem IA / Texto Cru: "):]
                    elif text.startswith("Pesquisando: "):
                        header_text = tr("overlay_searching_google")
                        display_text = text[len("Pesquisando: "):]
                    elif text.startswith("Copiado: "):
                        header_text = tr("overlay_copied_header")
                        display_text = text[len("Copiado: "):]
                    else:
                        header_text = tr("overlay_copied_header")
                        display_text = text
                
                header_font = QFont("Segoe UI", 7)
                header_font.setBold(True)
                self.label_header.setFont(header_font)
                self.label_header.setStyleSheet("color: #34d399; letter-spacing: 1.5px;")
                self.label_header.setText(header_text)
                self.label_header.show()
                
                content_font = QFont("Segoe UI", 8)
                content_font.setBold(False)
                self.label_content.setFont(content_font)
                self.label_content.setText(display_text)
                self.label_content.setStyleSheet("color: rgba(255, 255, 255, 230);")
                self.label_content.setWordWrap(True)
            else:
                self.label_header.hide()
                self.label_content.setText(text if text else tr("overlay_done"))
                self.label_content.setWordWrap(False)
                
                font = QFont("Segoe UI", 8)
                font.setBold(True)
                self.label_content.setFont(font)
                self.label_content.setStyleSheet("color: #ffffff; letter-spacing: 0.5px;")
                
            self.visualizer.hide()
            self.spinner.hide()
            self.indicator.setStyleSheet("background-color: #10b981; border-radius: 4px;")
            self.indicator.show()
            
        elif state == "error":
            self.main_frame.is_processing = False
            self.shadow_effect.setColor(QColor(239, 68, 68, 110))
            self.shadow_effect.setBlurRadius(24)
            self.label_content.setAlignment(Qt.AlignLeft | Qt.AlignVCenter)
            self.label_header.setAlignment(Qt.AlignLeft | Qt.AlignVCenter)
            
            if header_text or (text and text not in ["CONCLUÍDO!", "COPIADO!"]):
                if not header_text or not display_text:
                    header_text = tr("overlay_error_header")
                    if text.startswith("Erro: "):
                        display_text = text[len("Erro: "):]
                    else:
                        display_text = text
                
                self.label_header.setText(header_text)
                header_font = QFont("Segoe UI", 7)
                header_font.setBold(True)
                self.label_header.setFont(header_font)
                self.label_header.setStyleSheet("color: #ef4444; letter-spacing: 1.5px;")
                self.label_header.show()
                
                content_font = QFont("Segoe UI", 8)
                content_font.setBold(False)
                self.label_content.setFont(content_font)
                self.label_content.setText(display_text)
                self.label_content.setStyleSheet("color: rgba(255, 255, 255, 230);")
                self.label_content.setWordWrap(True)
            else:
                self.label_header.hide()
                self.label_content.setText(text if text else "ERRO!")
                self.label_content.setWordWrap(False)
                
                font = QFont("Segoe UI", 8)
                font.setBold(True)
                self.label_content.setFont(font)
                self.label_content.setStyleSheet("color: #ef4444; letter-spacing: 0.5px;")
            
            self.visualizer.hide()
            self.spinner.hide()
            self.indicator.setStyleSheet("background-color: #ef4444; border-radius: 4px;")
            self.indicator.show()

    def fade_in(self):
        """Triggers a smooth fade-in of the overlay. Frame is placed directly at target position (no slide)."""
        self.is_showing = True
        self.center_on_screen()

        # Place frame at final target position immediately — prevents text from rendering outside
        target_geom = self.get_frame_geometry(self.current_target_w, self.current_target_h)
        self.main_frame.setGeometry(target_geom)

        # Force layout calculation at correct position before showing
        if self.main_frame.layout():
            self.main_frame.layout().activate()

        self.setWindowOpacity(0.0)
        self.show()

        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(220)
        self.opacity_anim.setStartValue(0.0)
        self.opacity_anim.setEndValue(1.0)
        self.opacity_anim.setEasingCurve(QEasingCurve.OutCubic)

        self.anim_group = QParallelAnimationGroup()
        self.anim_group.addAnimation(self.opacity_anim)
        self.anim_group.start()
        
    def fade_out(self):
        """Triggers a smooth slide-down and fade-out animation."""
        if self._current_state in self.ACTIVE_STATES:
            return
        if not self.is_showing and not self.isVisible():
            return

        self._stop_animations()

        current_geom = self.main_frame.geometry()
        # Move 10px down
        target_geom = QRect(current_geom.x(), current_geom.y() + 10, current_geom.width(), current_geom.height())
        
        self.frame_anim = QPropertyAnimation(self.main_frame, b"geometry")
        self.frame_anim.setDuration(220)
        self.frame_anim.setStartValue(current_geom)
        self.frame_anim.setEndValue(target_geom)
        self.frame_anim.setEasingCurve(QEasingCurve.InCubic)
        
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(220)
        self.opacity_anim.setStartValue(self.windowOpacity())
        self.opacity_anim.setEndValue(0.0)
        
        self.anim_group = QParallelAnimationGroup()
        self.anim_group.addAnimation(self.frame_anim)
        self.anim_group.addAnimation(self.opacity_anim)
        self.anim_group.finished.connect(self.on_fade_out_finished)
        self.anim_group.start()
        
    def on_fade_out_finished(self):
        self.is_showing = False
        self.hide()
        self.setWindowOpacity(1.0) # Reset to full opacity for next trigger
        # Reset label opacities to full to avoid getting stuck invisible next time
        self.header_opacity.setOpacity(1.0)
        self.content_opacity.setOpacity(1.0)

    def update_volume_level(self, level):
        """Passes microphone volume levels to the visualizer."""
        self.visualizer.set_amplitude(level)
        self.main_frame.set_amplitude(level)

# Helper function to generate a solid color QIcon for system tray
def create_color_icon(color_hex):
    from PySide6.QtGui import QPixmap
    pixmap = QPixmap(16, 16)
    pixmap.fill(QColor(color_hex))
    return QIcon(pixmap)

# Main Application Core
from PySide6.QtCore import QObject

class HotkeySignalBridge(QObject):
    triggered = Signal(str)

class AudioLevelBridge(QObject):
    level_changed = Signal(float)

class FlowVoiceApp(QApplication):
    def __init__(self, sys_argv):
        super().__init__(sys_argv)
        self.setQuitOnLastWindowClosed(False)

        # Core Components
        self.config_manager = ConfigManager()
        # Load persisted UI language
        set_language(self.config_manager.get("ui_language", "pt"))
        self.level_bridge = AudioLevelBridge()
        self.recorder = AudioRecorder(level_callback=self.level_bridge.level_changed.emit)
        self.processor = AIProcessor(self.config_manager)
        self.paster = TextPaster()

        # UI Components
        self.overlay = FloatingOverlay()
        self.level_bridge.level_changed.connect(self.overlay.update_volume_level)
        self.tray_icon = None
        
        # State Variables
        self.is_recording = False
        self.ai_worker = None
        self.current_recording_mode = "ditado"
        self.latest_checked_version = None
        self.update_download_url = None

        # Bridge to route hotkey triggers safely to the main GUI thread
        self.hotkey_bridge = HotkeySignalBridge()
        self.hotkey_bridge.triggered.connect(self.toggle_dictation)

        # Hotkey listener initialization with multiple hotkeys dictionary
        self.hotkey_listener = HotkeyListener(self.get_hotkeys_map())
        self.hotkey_listener.start()

        self.setup_tray()

        # Configure registry startup value according to config setting
        set_run_at_startup(self.config_manager.get("start_with_windows", True))

        # Show first-run wizard only for genuinely new users.
        # If any API key is already configured, the user pre-dates the wizard — skip it.
        has_any_key = any(
            self.config_manager.get_api_key(p)
            for p in ["gemini", "openai", "groq", "github_models"]
        )
        wizard_done = self.config_manager.get("wizard_completed", False)
        if not wizard_done and not has_any_key:
            QTimer.singleShot(600, self.show_wizard)
        else:
            if not wizard_done:
                self.config_manager.set("wizard_completed", True)
            self.check_api_keys()

        # Check for updates in the background on startup
        self.update_checker = None
        QTimer.singleShot(1500, self.start_background_update_check)
        
        # Periodic check every 1 hour (3600000 ms)
        self.update_timer = QTimer(self)
        self.update_timer.timeout.connect(self.start_background_update_check)
        self.update_timer.start(3600000)

    def get_hotkeys_map(self):
        """Returns a mapping of hotkeys to their corresponding mode triggers."""
        h_dict = {}
        ditado_hk = self.config_manager.get("hotkey", "<ctrl>+<shift>+<space>")
        translation_hk = self.config_manager.get("hotkey_translation", "<ctrl>+<shift>+<y>")
        pesquisa_hk = self.config_manager.get("hotkey_pesquisa", "<ctrl>+<shift>+<u>")

        if ditado_hk:
            h_dict[ditado_hk] = lambda: self.hotkey_bridge.triggered.emit("default")
        if translation_hk:
            h_dict[translation_hk] = lambda: self.hotkey_bridge.triggered.emit("traducao")
        if pesquisa_hk:
            h_dict[pesquisa_hk] = lambda: self.hotkey_bridge.triggered.emit("pesquisa")

        return h_dict

    def show_wizard(self):
        wizard = SetupWizard(self.config_manager, app=self)
        if wizard.exec() == QDialog.Accepted:
            self.hotkey_listener.update_hotkeys(self.get_hotkeys_map())
            self.update_menu_checked_states()

    def check_api_keys(self):
        """Warns the user if the selected cloud provider lacks an API key."""
        provider = self.config_manager.get("provider", "gemini")
        if provider in ["gemini", "openai"] and not self.config_manager.get_api_key(provider):
            QTimer.singleShot(1000, self.show_settings_dialog)

    def start_background_update_check(self):
        self.update_checker = UpdateCheckerWorker()
        self.update_checker.update_available.connect(self.on_background_update_available)
        self.update_checker.start()

    def on_background_update_available(self, version, download_url):
        self.latest_checked_version = version
        self.update_download_url = download_url
        self.show_update_dialog(version, download_url)

    @Slot(str, str)
    def show_update_dialog(self, version, download_url):
        if self.tray_icon:
            self.tray_icon.showMessage(
                tr("update_notif_title"),
                tr("update_notif_msg", version),
                QSystemTrayIcon.Information,
                10000
            )
            try:
                self.tray_icon.messageClicked.disconnect()
            except Exception:
                pass
            
            from PySide6.QtGui import QDesktopServices
            from PySide6.QtCore import QUrl
            self.tray_icon.messageClicked.connect(lambda: QDesktopServices.openUrl(QUrl(download_url)))

    def setup_tray(self):
        """Initializes the System Tray Icon and its context menu."""
        self.tray_icon = QSystemTrayIcon(self)
        
        # Load custom app icon if exists
        icon_path = get_resource_path("icon.png")
        if os.path.exists(icon_path):
            app_icon = QIcon(icon_path)
            self.tray_icon.setIcon(app_icon)
            self.setWindowIcon(app_icon)
        else:
            self.tray_icon.setIcon(create_color_icon("#ffffff"))
            
        self.tray_icon.setToolTip(tr("tray_tooltip"))

        # Create menu
        menu = QMenu()
        menu.setStyleSheet("""
            QMenu {
                background-color: #0a0a0a;
                color: #ffffff;
                border: 1px solid #222222;
                padding: 4px;
                font-family: 'Segoe UI', sans-serif;
                font-size: 12px;
            }
            QMenu::item {
                padding: 6px 20px;
                background-color: transparent;
            }
            QMenu::item:selected {
                background-color: #ffffff;
                color: #000000;
                border-radius: 4px;
            }
        """)

        # Style Sub-menu
        style_menu = menu.addMenu(tr("tray_style_menu"))
        self.action_prof = QAction(tr("tray_style_prof"), self, checkable=True)
        self.action_prof.triggered.connect(lambda: self.change_active_style("Profissional"))
        self.action_casual = QAction(tr("tray_style_casual"), self, checkable=True)
        self.action_casual.triggered.connect(lambda: self.change_active_style("Casual"))
        self.action_raw = QAction(tr("tray_style_raw"), self, checkable=True)
        self.action_raw.triggered.connect(lambda: self.change_active_style("Direto"))

        style_menu.addAction(self.action_prof)
        style_menu.addAction(self.action_casual)
        style_menu.addAction(self.action_raw)

        # Mode Sub-menu
        mode_menu = menu.addMenu(tr("tray_mode_menu"))
        self.action_mode_dictation = QAction(tr("tray_mode_dictation"), self, checkable=True)
        self.action_mode_dictation.triggered.connect(lambda: self.change_operation_mode("ditado"))
        self.action_mode_translation = QAction(tr("tray_mode_translation"), self, checkable=True)
        self.action_mode_translation.triggered.connect(lambda: self.change_operation_mode("traducao"))
        self.action_mode_search = QAction(tr("tray_mode_search"), self, checkable=True)
        self.action_mode_search.triggered.connect(lambda: self.change_operation_mode("pesquisa"))

        mode_menu.addAction(self.action_mode_dictation)
        mode_menu.addAction(self.action_mode_translation)
        mode_menu.addAction(self.action_mode_search)

        # Translation Language Sub-menu
        lang_menu = menu.addMenu(tr("tray_lang_menu"))
        self.lang_actions = {}
        for lang in ["Inglês", "Espanhol", "Francês", "Alemão", "Italiano"]:
            act = QAction(lang, self, checkable=True)
            act.triggered.connect(lambda checked, l=lang: self.change_translation_target(l))
            lang_menu.addAction(act)
            self.lang_actions[lang] = act

        self.update_menu_checked_states()

        # Settings action
        action_settings = QAction(tr("tray_settings"), self)
        action_settings.triggered.connect(self.show_settings_dialog)
        menu.addAction(action_settings)

        # Test record action
        action_test_record = QAction(tr("tray_test_record"), self)
        action_test_record.triggered.connect(lambda: self.toggle_dictation("default"))
        menu.addAction(action_test_record)

        menu.addSeparator()

        # Quit action
        action_quit = QAction(tr("tray_quit"), self)
        action_quit.triggered.connect(self.quit_app)
        menu.addAction(action_quit)

        self.tray_icon.setContextMenu(menu)
        self.tray_icon.show()
        
        # Double click opens settings
        self.tray_icon.activated.connect(self.on_tray_activated)

    def on_tray_activated(self, reason):
        if reason == QSystemTrayIcon.DoubleClick:
            self.show_settings_dialog()

    def update_menu_checked_states(self):
        """Checks the active formatting style and operational modes in the menu items."""
        active_style = self.config_manager.get("active_style", "Profissional")
        self.action_prof.setChecked(active_style == "Profissional")
        self.action_casual.setChecked(active_style == "Casual")
        self.action_raw.setChecked(active_style == "Direto")

        active_mode = self.config_manager.get("operation_mode", "ditado")
        self.action_mode_dictation.setChecked(active_mode == "ditado")
        self.action_mode_translation.setChecked(active_mode == "traducao")
        self.action_mode_search.setChecked(active_mode == "pesquisa")

        active_lang = self.config_manager.get("translation_target", "Inglês")
        for lang, act in self.lang_actions.items():
            act.setChecked(lang == active_lang)

    def change_active_style(self, style_name):
        self.config_manager.set("active_style", style_name)
        self.update_menu_checked_states()
        print(f"Estilo ativo alterado para: {style_name}")

    def change_operation_mode(self, mode_name):
        self.config_manager.set("operation_mode", mode_name)
        self.update_menu_checked_states()
        print(f"Modo de operação alterado para: {mode_name}")

    def change_translation_target(self, lang_name):
        self.config_manager.set("translation_target", lang_name)
        self.update_menu_checked_states()
        print(f"Idioma de tradução alterado para: {lang_name}")

    def show_settings_dialog(self):
        dialog = SettingsDialog(self.config_manager, app=self)
        if dialog.exec() == QDialog.Accepted:
            # Re-initialize all hotkeys in the listener
            self.hotkey_listener.update_hotkeys(self.get_hotkeys_map())
            self.update_menu_checked_states()

    def toggle_dictation(self, trigger_mode="default"):
        """Triggered by the hotkeys. Switches between recording states."""
        # Check if currently processing AI - if so, block new recording until finished
        if self.ai_worker and self.ai_worker.isRunning():
            print("Processamento em andamento. Aguarde...")
            return

        if not self.is_recording:
            # Determine actual mode to use
            if trigger_mode == "default":
                self.current_recording_mode = self.config_manager.get("operation_mode", "ditado")
            else:
                self.current_recording_mode = trigger_mode

            self.is_recording = True
            
            # Show appropriate recording overlay
            overlay_text = "GRAVANDO"
            if self.current_recording_mode == "traducao":
                target = self.config_manager.get("translation_target", "Inglês")
                overlay_text = f"TRADUZINDO ({target.upper()})"
            elif self.current_recording_mode == "pesquisa":
                overlay_text = "PESQUISANDO"

            QTimer.singleShot(0, lambda: self.overlay.show_state("listening", overlay_text))
            if self.config_manager.get("mute_on_record", False):
                mute_system_audio()
            self.recorder.start()
        else:
            # Stop dictation
            self.is_recording = False
            
            # Show appropriate processing overlay
            processing_text = "POLINDO"
            if self.current_recording_mode == "traducao":
                processing_text = "TRADUZINDO"
            elif self.current_recording_mode == "pesquisa":
                processing_text = "BUSCANDO"

            QTimer.singleShot(0, lambda: self.overlay.show_state("processing", processing_text))
            audio_path = self.recorder.stop()
            if self.config_manager.get("mute_on_record", False):
                unmute_system_audio()
            if audio_path:
                self.process_audio(audio_path)

    def process_audio(self, audio_path):
        """Launches AIWorker thread to transcribe and format audio file."""
        target_lang = self.config_manager.get("translation_target", "Inglês")
        self.ai_worker = AIWorker(self.processor, audio_path, self.current_recording_mode, target_lang)
        self.ai_worker.status_changed.connect(self.on_ai_status_changed)
        self.ai_worker.finished.connect(self.on_ai_success)
        self.ai_worker.error.connect(self.on_ai_error)
        self.ai_worker.start()

    @Slot(str)
    def on_ai_status_changed(self, status_msg):
        self.overlay.show_state("processing", status_msg)

    @Slot(str, str)
    def on_ai_success(self, text, mode):
        print(f"Transcrição concluída ({mode}): {text}")
        
        is_raw_fallback = False
        if text.startswith("RawFallback:"):
            text = text[len("RawFallback:"):]
            is_raw_fallback = True
            
        if mode == "pesquisa":
            if " ||| " in text:
                query, answer = text.split(" ||| ", 1)
            else:
                query, answer = text, "Não foi possível obter uma resposta."
            
            if answer.startswith("RawFallback:"):
                answer = answer[len("RawFallback:"):]
                
            self.overlay.show_state("done", f"Pesquisa: {query}")
            
            # Show the SearchResultCard modelessly so it floats on top without blocking the app
            self.search_card = SearchResultCard(query, answer, self.processor, self.recorder)
            self.search_card.show()
        else:
            overlay_text = text
            if mode == "traducao":
                if is_raw_fallback:
                    overlay_text = f"Sem IA / Tradução Falhou: {text}"
                else:
                    overlay_text = f"Traduzido: {text}"
            else:
                if is_raw_fallback:
                    overlay_text = f"Sem IA / Texto Cru: {text}"
                else:
                    overlay_text = f"Copiado: {text}"
                
            self.paster.paste_text(text)
            self.overlay.show_state("done", overlay_text)
            
        # Cleanup temporary audio files
        self.recorder.cleanup()

    @Slot(str)
    def on_ai_error(self, err_msg):
        print(f"Erro no processamento da IA: {err_msg}")
        self.overlay.show_state("error", f"Erro: {err_msg}")
        self.recorder.cleanup()

    def quit_app(self):
        print("Saindo do FlowVoice...")
        self.hotkey_listener.stop()
        self.recorder.cleanup()
        self.quit()

if __name__ == "__main__":
    import logging
    log_dir = get_app_data_dir()
    log_filepath = os.path.join(log_dir, "flowvoice.log")
    logging.basicConfig(
        filename=log_filepath,
        filemode='a',
        format='%(asctime)s - %(levelname)s - %(message)s',
        level=logging.INFO,
        encoding='utf-8'
    )
    
    class StreamToLogger:
        def __init__(self, logger_func):
            self.logger_func = logger_func
        def write(self, buf):
            for line in buf.rstrip().splitlines():
                self.logger_func(line.rstrip())
        def flush(self):
            pass
            
    if getattr(sys, 'frozen', False):
        sys.stdout = StreamToLogger(logging.info)
        sys.stderr = StreamToLogger(logging.error)
        
    logging.info("FlowVoice iniciado.")
    
    app = FlowVoiceApp(sys.argv)
    sys.exit(app.exec())
