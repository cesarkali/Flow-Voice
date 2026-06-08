import sys
import os
import time
import urllib.request
import json
import re
import tempfile
from PySide6.QtWidgets import (
    QApplication, QWidget, QHBoxLayout, QVBoxLayout, QLabel, 
    QSystemTrayIcon, QMenu, QDialog, QFormLayout, QLineEdit, 
    QComboBox, QPushButton, QMessageBox, QFrame, QGraphicsDropShadowEffect,
    QTextEdit, QCheckBox, QProgressBar
)
from PySide6.QtCore import Qt, QTimer, QThread, Signal, Slot, QPropertyAnimation, QParallelAnimationGroup, QEasingCurve, QRect
from PySide6.QtGui import QIcon, QColor, QFont, QAction, QPainter, QBrush, QPen
from PySide6 import QtSvg

CURRENT_VERSION = "1.2.0"

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
from config import ConfigManager
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

# Helper functions for version updates
def get_latest_release():
    url = "https://api.github.com/repos/cesarkali/Flow-Voice/releases/latest"
    req = urllib.request.Request(
        url,
        headers={'User-Agent': 'FlowVoice-Updater'}
    )
    try:
        with urllib.request.urlopen(req, timeout=5) as response:
            data = json.loads(response.read().decode('utf-8'))
            tag_name = data.get("tag_name", "")
            # Clean version tag (e.g. "v1.0.1" -> "1.0.1")
            version_match = re.search(r"(\d+\.\d+\.\d+)", tag_name)
            if not version_match:
                return None
            latest_ver = version_match.group(1)
            
            # Find FlowVoiceSetup.exe asset
            download_url = None
            for asset in data.get("assets", []):
                if asset.get("name") == "FlowVoiceSetup.exe":
                    download_url = asset.get("browser_download_url")
                    break
            if not download_url:
                # Fallback: construct download URL from tag_name if asset not found in JSON yet
                download_url = f"https://github.com/cesarkali/Flow-Voice/releases/download/{tag_name}/FlowVoiceSetup.exe"
                
            return latest_ver, download_url
    except Exception as e:
        print(f"Erro ao verificar atualizações: {e}")
        return None

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
        res = get_latest_release()
        if res:
            latest_ver, download_url = res
            if is_version_newer(CURRENT_VERSION, latest_ver):
                self.update_available.emit(latest_ver, download_url)
            else:
                self.no_update_found.emit()
        else:
            self.error.emit("Não foi possível conectar ao servidor de atualizações.")

# Background thread to download the installer
class DownloadWorker(QThread):
    progress = Signal(int) # percentage
    finished = Signal(str) # temp file path
    error = Signal(str)
    
    def __init__(self, url):
        super().__init__()
        self.url = url
        
    def run(self):
        try:
            temp_dir = tempfile.gettempdir()
            dest_path = os.path.join(temp_dir, "FlowVoiceSetup_update.exe")
            
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
        self.download_worker = DownloadWorker(self.download_url)
        self.download_worker.progress.connect(self.on_download_progress)
        self.download_worker.finished.connect(self.on_download_finished)
        self.download_worker.error.connect(self.on_download_error)
        self.download_worker.start()

    def on_download_progress(self, percent):
        self.progress_bar.setValue(percent)
        self.lbl_progress.setText(f"Baixando... {percent}%")

    def on_download_finished(self, dest_path):
        self.lbl_desc.setText("Pronto! Iniciando o instalador...")
        self.lbl_progress.setText("Instalador executando...")
        QTimer.singleShot(1000, lambda: self.launch_installer_and_exit(dest_path))

    def launch_installer_and_exit(self, dest_path):
        try:
            # Launch setup.exe in default OS shell
            os.startfile(dest_path)
            # Quit PySide6 App to release files so installer can overwrite them
            QApplication.quit()
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
        self.setPlaceholderText("Pressione as teclas (Esc p/ cancelar)...")
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

def set_run_at_startup(enabled=True):
    import sys
    if not getattr(sys, 'frozen', False):
        return
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

def is_run_at_startup_enabled():
    import sys
    if not getattr(sys, 'frozen', False):
        return False
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

# Settings/Wizard Dialog
class SettingsDialog(QDialog):
    def __init__(self, config_manager, parent=None):
        super().__init__(parent)
        self.config_manager = config_manager
        self.drag_position = None
        self.init_ui()

    def create_hotkey_row(self, label_text, config_key, default_val):
        h_layout = QHBoxLayout()
        h_layout.setSpacing(6)
        h_layout.setContentsMargins(0, 0, 0, 0)
        
        line_edit = HotkeyLineEdit()
        line_edit.setText(self.config_manager.get(config_key, default_val))
        
        btn_capture = QPushButton("Capturar")
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
                btn.setText("Gravando...")
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
            btn.setText("Capturar")
            btn.setStyleSheet("")
            
        btn_capture.clicked.connect(lambda: on_capture_clicked(line_edit, btn_capture))
        
        h_layout.addWidget(line_edit, 1)
        h_layout.addWidget(btn_capture)
        
        return line_edit, h_layout

    def init_ui(self):
        self.setWindowFlags(Qt.FramelessWindowHint | Qt.Dialog)
        self.setAttribute(Qt.WA_TranslucentBackground, True)
        self.setFixedSize(620, 780)

        # Main container for shadow and borders
        self.container_frame = QFrame(self)
        self.container_frame.setObjectName("container_frame")
        self.container_frame.setGeometry(10, 10, 600, 760)
        
        self.container_frame.setStyleSheet("""
            QFrame#container_frame {
                background-color: rgba(15, 15, 15, 245);
                border: 1px solid rgba(255, 255, 255, 35);
                border-radius: 12px;
            }
        """)

        # Drop shadow
        shadow = QGraphicsDropShadowEffect(self)
        shadow.setBlurRadius(20)
        shadow.setColor(QColor(0, 0, 0, 220))
        shadow.setOffset(0, 5)
        self.container_frame.setGraphicsEffect(shadow)

        # Stylesheet for inner elements
        self.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 220);
                font-size: 13px;
                font-weight: 600;
                font-family: 'Segoe UI', sans-serif;
            }
            QLineEdit, QComboBox {
                background-color: rgba(255, 255, 255, 8);
                border: 1px solid rgba(255, 255, 255, 20);
                border-radius: 6px;
                padding: 4px 10px;
                min-height: 28px;
                color: #ffffff;
                font-size: 13px;
                font-family: 'Segoe UI', sans-serif;
            }
            QLineEdit:focus, QComboBox:focus {
                border: 1px solid rgba(255, 255, 255, 100);
                background-color: rgba(255, 255, 255, 15);
            }
            QPushButton#btn_save {
                background-color: #ffffff;
                border: none;
                border-radius: 6px;
                color: #000000;
                font-weight: 700;
                padding: 8px 18px;
                font-size: 12px;
            }
            QPushButton#btn_save:hover {
                background-color: rgba(255, 255, 255, 220);
            }
            QPushButton#btn_cancel {
                background-color: transparent;
                border: 1px solid rgba(255, 255, 255, 25);
                border-radius: 6px;
                color: rgba(255, 255, 255, 160);
                padding: 8px 18px;
                font-size: 12px;
            }
            QPushButton#btn_cancel:hover {
                background-color: rgba(255, 255, 255, 12);
                color: #ffffff;
                border: 1px solid rgba(255, 255, 255, 60);
            }
            QPushButton#btn_close {
                background-color: transparent;
                border: none;
                color: rgba(255, 255, 255, 120);
                font-size: 15px;
                font-weight: bold;
            }
            QPushButton#btn_close:hover {
                color: #ff5555;
            }
        """)

        # Main Layout inside container
        main_layout = QVBoxLayout(self.container_frame)
        main_layout.setContentsMargins(20, 15, 20, 20)
        main_layout.setSpacing(18)

        # 1. Custom Title Bar Layout
        header_layout = QHBoxLayout()
        header_layout.setContentsMargins(0, 0, 0, 0)
        
        lbl_title = QLabel("FLOWVOICE CONFIGURAÇÕES")
        title_font = QFont("Segoe UI", 9)
        title_font.setBold(True)
        lbl_title.setFont(title_font)
        lbl_title.setStyleSheet("color: #ffffff; letter-spacing: 1.5px;")
        header_layout.addWidget(lbl_title)
        
        header_layout.addStretch()
        
        btn_close = QPushButton("✕")
        btn_close.setObjectName("btn_close")
        btn_close.setFixedSize(24, 24)
        btn_close.clicked.connect(lambda: self.fade_out_and_close(False))
        header_layout.addWidget(btn_close)
        
        main_layout.addLayout(header_layout)

        # Separator line
        sep = QFrame()
        sep.setFixedHeight(1)
        sep.setStyleSheet("background-color: rgba(255, 255, 255, 20);")
        main_layout.addWidget(sep)

        # 2. Form Layout for Inputs
        form_layout = QFormLayout()
        form_layout.setSpacing(18)
        form_layout.setContentsMargins(0, 5, 0, 5)

        self.combo_provider = QComboBox()
        self.combo_provider.addItems(["gemini", "openai", "groq", "github_models"])
        self.combo_provider.setCurrentText(self.config_manager.get("provider", "gemini"))
        form_layout.addRow("Provedor Preferencial:", self.combo_provider)

        self.combo_style = QComboBox()
        self.combo_style.addItems(["Profissional", "Casual", "Direto"])
        self.combo_style.setCurrentText(self.config_manager.get("active_style", "Profissional"))
        form_layout.addRow("Estilo de Escrita Ativo:", self.combo_style)

        self.txt_gemini = QLineEdit()
        self.txt_gemini.setEchoMode(QLineEdit.Password)
        self.txt_gemini.setText(self.config_manager.get_api_key("gemini"))
        self.txt_gemini.setPlaceholderText("Chaves separadas por vírgula")
        form_layout.addRow("Chaves Gemini (sep. por vírgula):", self.txt_gemini)

        self.txt_openai = QLineEdit()
        self.txt_openai.setEchoMode(QLineEdit.Password)
        self.txt_openai.setText(self.config_manager.get_api_key("openai"))
        self.txt_openai.setPlaceholderText("Chaves separadas por vírgula")
        form_layout.addRow("Chaves OpenAI (sep. por vírgula):", self.txt_openai)

        self.txt_groq = QLineEdit()
        self.txt_groq.setEchoMode(QLineEdit.Password)
        self.txt_groq.setText(self.config_manager.get_api_key("groq"))
        self.txt_groq.setPlaceholderText("Chaves separadas por vírgula")
        form_layout.addRow("Chaves Groq (sep. por vírgula):", self.txt_groq)

        self.txt_github_models = QLineEdit()
        self.txt_github_models.setEchoMode(QLineEdit.Password)
        self.txt_github_models.setText(self.config_manager.get_api_key("github_models"))
        self.txt_github_models.setPlaceholderText("Chaves separadas por vírgula")
        form_layout.addRow("Chaves GitHub Models:", self.txt_github_models)

        self.combo_whisper = QComboBox()
        self.combo_whisper.addItems(["tiny", "base", "small"])
        self.combo_whisper.setCurrentText(self.config_manager.get("whisper", {}).get("model_size", "base"))
        form_layout.addRow("Modelo Whisper (Local):", self.combo_whisper)

        self.combo_mode = QComboBox()
        self.combo_mode.addItems(["Ditado", "Tradução", "Pesquisa"])
        mode_map = {"ditado": "Ditado", "traducao": "Tradução", "pesquisa": "Pesquisa"}
        self.combo_mode.setCurrentText(mode_map.get(self.config_manager.get("operation_mode", "ditado"), "Ditado"))
        form_layout.addRow("Modo de Operação Padrão:", self.combo_mode)

        self.combo_target_lang = QComboBox()
        self.combo_target_lang.addItems(["Inglês", "Espanhol", "Francês", "Alemão", "Italiano"])
        self.combo_target_lang.setCurrentText(self.config_manager.get("translation_target", "Inglês"))
        form_layout.addRow("Idioma de Tradução:", self.combo_target_lang)

        # Hotkeys with Capture buttons
        self.txt_hotkey, layout_hk = self.create_hotkey_row("Atalho Ditado Padrão:", "hotkey", "<ctrl>+<shift>+<space>")
        form_layout.addRow("Atalho Ditado Padrão:", layout_hk)

        self.txt_hotkey_translation, layout_hkt = self.create_hotkey_row("Atalho Tradução:", "hotkey_translation", "<ctrl>+<shift>+<y>")
        form_layout.addRow("Atalho Tradução:", layout_hkt)

        self.txt_hotkey_pesquisa, layout_hkp = self.create_hotkey_row("Atalho Pesquisa Google:", "hotkey_pesquisa", "<ctrl>+<shift>+<u>")
        form_layout.addRow("Atalho Pesquisa Google:", layout_hkp)

        self.chk_startup = QCheckBox("Iniciar junto com o Windows")
        self.chk_startup.setChecked(is_run_at_startup_enabled() or self.config_manager.get("start_with_windows", False))
        
        self.chk_mute = QCheckBox("Mutar áudio do PC durante a gravação")
        self.chk_mute.setChecked(self.config_manager.get("mute_on_record", False))
        
        # Get base directory dynamically
        if getattr(sys, 'frozen', False):
            appdata = os.getenv('APPDATA')
            if appdata:
                base_dir = os.path.join(appdata, "FlowVoice")
                os.makedirs(base_dir, exist_ok=True)
            else:
                base_dir = os.path.dirname(sys.executable)
        else:
            base_dir = os.path.dirname(os.path.abspath(__file__))
            
        checkmark_path = os.path.join(base_dir, "checkmark.svg").replace("\\", "/")
        
        # Write checkmark SVG if it doesn't exist yet
        if not os.path.exists(checkmark_path):
            try:
                with open(checkmark_path, "w", encoding="utf-8") as f:
                    f.write(
                        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="16" height="16">'
                        '<path fill="none" stroke="#ffffff" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" d="M3 8.5l3.5 3.5 6.5-7"/>'
                        '</svg>'
                    )
            except Exception as e:
                print(f"Erro ao criar checkmark.svg: {e}")
                
        checkbox_style = f"""
            QCheckBox {{
                color: rgba(255, 255, 255, 220);
                font-size: 13px;
                font-family: 'Segoe UI', sans-serif;
                spacing: 8px;
            }}
            QCheckBox::indicator {{
                width: 16px;
                height: 16px;
                min-width: 16px;
                max-width: 16px;
                min-height: 16px;
                max-height: 16px;
                border: 1px solid rgba(255, 255, 255, 40);
                border-radius: 3px;
                background-color: rgba(255, 255, 255, 10);
            }}
            QCheckBox::indicator:unchecked:hover {{
                border-color: #8b5cf6;
            }}
            QCheckBox::indicator:checked {{
                background-color: #8b5cf6;
                border: 1px solid #8b5cf6;
                image: url("{checkmark_path}");
            }}
        """
        self.chk_startup.setStyleSheet(checkbox_style)
        self.chk_mute.setStyleSheet(checkbox_style)
        
        form_layout.addRow("", self.chk_startup)
        form_layout.addRow("", self.chk_mute)

        main_layout.addLayout(form_layout)
        main_layout.addSpacing(5)

        # Info Box for API Keys
        info_frame = QFrame()
        info_frame.setStyleSheet("""
            QFrame {
                background-color: rgba(255, 255, 255, 6);
                border: 1px dashed rgba(255, 255, 255, 18);
                border-radius: 8px;
            }
        """)
        info_layout = QVBoxLayout(info_frame)
        info_layout.setContentsMargins(12, 10, 12, 10)
        info_layout.setSpacing(4)
        
        lbl_info_title = QLabel("💡 Precisa de chaves de API grátis?")
        lbl_info_title.setStyleSheet("color: #ffffff; font-size: 11px; font-weight: bold; font-family: 'Segoe UI', sans-serif;")
        
        lbl_info_desc = QLabel(
            "Caso não configure chaves de API, o programa colará apenas o texto cru transcrevido pelo Whisper.<br/>"
            "• <b>Groq:</b> Acesse <a href='https://console.groq.com/keys' style='color:#8b5cf6; text-decoration:none;'>console.groq.com/keys</a> para gerar chaves grátis super rápidas.<br/>"
            "• <b>Gemini:</b> Acesse <a href='https://aistudio.google.com' style='color:#8b5cf6; text-decoration:none;'>aistudio.google.com</a> para obter a chave grátis do Google."
        )
        lbl_info_desc.setOpenExternalLinks(True)
        lbl_info_desc.setWordWrap(True)
        lbl_info_desc.setStyleSheet("color: rgba(255, 255, 255, 160); font-size: 11px; font-weight: normal; font-family: 'Segoe UI', sans-serif; line-height: 1.4;")
        
        info_layout.addWidget(lbl_info_title)
        info_layout.addWidget(lbl_info_desc)
        main_layout.addWidget(info_frame)
        main_layout.addSpacing(5)

        # 3. Action Buttons
        btn_layout = QHBoxLayout()
        btn_layout.setSpacing(10)
        
        btn_cancel = QPushButton("Cancelar")
        btn_cancel.setObjectName("btn_cancel")
        btn_cancel.clicked.connect(lambda: self.fade_out_and_close(False))
        
        btn_check_update = QPushButton("Verificar Atualizações")
        btn_check_update.setObjectName("btn_check_update")
        btn_check_update.setCursor(Qt.PointingHandCursor)
        btn_check_update.clicked.connect(self.check_updates_manually)
        btn_check_update.setStyleSheet("""
            QPushButton#btn_check_update {
                background-color: transparent;
                border: 1px solid rgba(139, 92, 246, 120);
                border-radius: 6px;
                color: #8b5cf6;
                padding: 8px 14px;
                font-size: 12px;
                font-weight: bold;
                min-height: 32px;
            }
            QPushButton#btn_check_update:hover {
                background-color: rgba(139, 92, 246, 25);
                border: 1px solid #8b5cf6;
                color: #ffffff;
            }
        """)
        
        btn_save = QPushButton("Salvar Configurações")
        btn_save.setObjectName("btn_save")
        btn_save.clicked.connect(self.save_settings)
        btn_save.setFocus()

        btn_layout.addWidget(btn_cancel)
        btn_layout.addWidget(btn_check_update)
        btn_layout.addWidget(btn_save)
        main_layout.addLayout(btn_layout)

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

    # Smooth Window Transitions (Fade & Slide)
    def showEvent(self, event):
        geom = self.geometry()
        self.setWindowOpacity(0.0)
        
        self.pos_anim = QPropertyAnimation(self, b"geometry")
        self.pos_anim.setDuration(220)
        self.pos_anim.setStartValue(QRect(geom.x(), geom.y() + 15, geom.width(), geom.height()))
        self.pos_anim.setEndValue(geom)
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

    def fade_out_and_close(self, accept_dialog=False):
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
        
        if accept_dialog:
            self.anim_group.finished.connect(self.accept)
        else:
            self.anim_group.finished.connect(self.reject)
        self.anim_group.start()

    def save_settings(self):
        """Saves current fields to config_manager."""
        self.config_manager.set("provider", self.combo_provider.currentText())
        self.config_manager.set("active_style", self.combo_style.currentText())
        self.config_manager.set_api_key("gemini", self.txt_gemini.text().strip())
        self.config_manager.set_api_key("openai", self.txt_openai.text().strip())
        self.config_manager.set_api_key("groq", self.txt_groq.text().strip())
        self.config_manager.set_api_key("github_models", self.txt_github_models.text().strip())
        
        reverse_mode_map = {"Ditado": "ditado", "Tradução": "traducao", "Pesquisa": "pesquisa"}
        self.config_manager.set("operation_mode", reverse_mode_map.get(self.combo_mode.currentText(), "ditado"))
        self.config_manager.set("translation_target", self.combo_target_lang.currentText())
        self.config_manager.set("hotkey", self.txt_hotkey.text().strip())
        self.config_manager.set("hotkey_translation", self.txt_hotkey_translation.text().strip())
        self.config_manager.set("hotkey_pesquisa", self.txt_hotkey_pesquisa.text().strip())
        
        whisper_cfg = self.config_manager.get("whisper", {})
        whisper_cfg["model_size"] = self.combo_whisper.currentText()
        self.config_manager.set("whisper", whisper_cfg)
        
        # Save Windows Startup and Mute config
        start_with_win = self.chk_startup.isChecked()
        self.config_manager.set("start_with_windows", start_with_win)
        set_run_at_startup(start_with_win)
        
        mute_on_rec = self.chk_mute.isChecked()
        self.config_manager.set("mute_on_record", mute_on_rec)
        
        self.fade_out_and_close(True)

    def check_updates_manually(self):
        # Find the updates button and disable it during the search
        btn_update = self.findChild(QPushButton, "btn_check_update")
        if btn_update:
            btn_update.setEnabled(False)
            btn_update.setText("Verificando...")
        
        self.manual_checker = UpdateCheckerWorker()
        self.manual_checker.update_available.connect(self.on_manual_update_available)
        self.manual_checker.no_update_found.connect(self.on_manual_no_update)
        self.manual_checker.error.connect(self.on_manual_update_error)
        self.manual_checker.start()

    def on_manual_update_available(self, version, download_url):
        btn_update = self.findChild(QPushButton, "btn_check_update")
        if btn_update:
            btn_update.setEnabled(True)
            btn_update.setText("Verificar Atualizações")
            
        # Show UpdateDialog
        self.update_dialog = UpdateDialog(version, download_url, self)
        self.update_dialog.show()

    def on_manual_no_update(self):
        btn_update = self.findChild(QPushButton, "btn_check_update")
        if btn_update:
            btn_update.setEnabled(True)
            btn_update.setText("Verificar Atualizações")
        QMessageBox.information(self, "FlowVoice", f"O FlowVoice já está atualizado!\nVersão atual: v{CURRENT_VERSION}")

    def on_manual_update_error(self, err_msg):
        btn_update = self.findChild(QPushButton, "btn_check_update")
        if btn_update:
            btn_update.setEnabled(True)
            btn_update.setText("Verificar Atualizações")
        QMessageBox.warning(self, "FlowVoice", f"Não foi possível verificar atualizações:\n{err_msg}")



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
        # Scale and clip amplitude
        self.target_amplitude = min(max(value * 15.0, 0.0), 1.0)
        
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
        
        # 3 overlapping liquid waves with different properties
        wave_configs = [
            (QColor(255, 255, 255, 180), 0.9, 0.0, 0.08),
            (QColor(255, 255, 255, 90), 0.6, 2.0, 0.12),
            (QColor(255, 255, 255, 40), 0.3, 4.0, 0.06)
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
                
            painter.setPen(QPen(color, 1.5))
            painter.setBrush(Qt.NoBrush)
            path_obj = path
            painter.drawPath(path_obj)


# Rotating Circular Spinner Widget (Mac/Web-style premium fading tail)
class LoadingSpinner(QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedSize(20, 20)
        self.angle = 0
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.rotate)
        self.timer.start(16) # ~60 FPS
        
    def rotate(self):
        self.angle = (self.angle + 6) % 360
        self.update()
        
    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)
        
        w = self.width()
        h = self.height()
        radius = 7.0
        
        painter.translate(w / 2.0, h / 2.0)
        painter.rotate(self.angle)
        
        # Draw segments with fading opacity for a smooth tail effect
        for i in range(8):
            opacity = int(255 * (i / 7.0))
            color = QColor(255, 255, 255, opacity)
            pen = QPen(color, 2.0, Qt.SolidLine, Qt.RoundCap)
            painter.setPen(pen)
            painter.drawArc(int(-radius), int(-radius), int(radius * 2), int(radius * 2), i * 45 * 16, 30 * 16)


# Frameless Glassmorphism Overlay (Sleek pill with slide and fade transitions)
class FloatingOverlay(QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.current_target_w = 220
        self.current_target_h = 56
        self.is_showing = False
        self.anim_group = None
        self.frame_anim = None
        self.opacity_anim = None
        self.init_ui()

    def init_ui(self):
        # Frameless, translucent, floating window settings
        self.setWindowFlags(Qt.WindowStaysOnTopHint | Qt.FramelessWindowHint | Qt.Tool | Qt.SubWindow)
        self.setAttribute(Qt.WA_TranslucentBackground, True)
        self.setAttribute(Qt.WA_ShowWithoutActivating, True) # Prevents stealing window focus
        self.setFixedSize(500, 320)

        # Base Frame container for glassmorphism styling
        self.main_frame = QFrame(self)
        self.main_frame.setObjectName("main_frame")
        
        # Apply premium Vercel-style deep black acrylic style
        self.main_frame.setStyleSheet("""
            QFrame#main_frame {
                background-color: rgba(10, 10, 10, 235);
                border: 1px solid rgba(255, 255, 255, 30);
                border-radius: 18px;
            }
        """)

        # Drop shadow effect
        shadow = QGraphicsDropShadowEffect(self)
        shadow.setBlurRadius(16)
        shadow.setColor(QColor(0, 0, 0, 180))
        shadow.setOffset(0, 3)
        self.main_frame.setGraphicsEffect(shadow)

        # Layout inside the frame
        layout = QHBoxLayout(self.main_frame)
        layout.setContentsMargins(14, 0, 14, 0)
        layout.setSpacing(10)

        # 1. Sound wave visualizer (Siri Waves)
        self.visualizer = SoundVisualizer(self.main_frame)
        layout.addWidget(self.visualizer)

        # 2. Loading spinner (Rotating tail)
        self.spinner = LoadingSpinner(self.main_frame)
        layout.addWidget(self.spinner)

        # 3. Static state indicator dot (for done/error states)
        self.indicator = QLabel(self.main_frame)
        self.indicator.setFixedSize(8, 8)
        self.indicator.setStyleSheet("background-color: #ffffff; border-radius: 4px;")
        layout.addWidget(self.indicator)

        # 4. Vertical Text Layout for Header and Content
        text_layout = QVBoxLayout()
        text_layout.setSpacing(1)
        text_layout.setContentsMargins(0, 0, 0, 0)
        text_layout.setAlignment(Qt.AlignVCenter)

        self.label_header = QLabel(self.main_frame)
        header_font = QFont("Segoe UI", 7)
        header_font.setBold(True)
        self.label_header.setFont(header_font)
        self.label_header.setStyleSheet("color: rgba(255, 255, 255, 120); letter-spacing: 1px;")
        text_layout.addWidget(self.label_header)

        self.label_content = QLabel("GRAVANDO", self.main_frame)
        content_font = QFont("Segoe UI", 9)
        content_font.setBold(True)
        self.label_content.setFont(content_font)
        self.label_content.setStyleSheet("color: #ffffff; letter-spacing: 0.5px;")
        self.label_content.setWordWrap(True)
        text_layout.addWidget(self.label_content)

        layout.addLayout(text_layout, 1)

        # Set initial state UI visibility
        self.visualizer.hide()
        self.spinner.hide()
        self.indicator.hide()
        self.label_header.hide()

    def calculate_text_height(self, text, font, width):
        from PySide6.QtGui import QFontMetrics
        metrics = QFontMetrics(font)
        rect = metrics.boundingRect(0, 0, width, 9999, Qt.TextWordWrap, text)
        return rect.height()

    def get_centered_geometry(self, width, height):
        screen = QApplication.primaryScreen().availableGeometry()
        x = screen.x() + (screen.width() - width) // 2
        # Position 20 pixels above the bottom edge of the available screen area (above taskbar)
        y = screen.y() + screen.height() - height - 20
        return QRect(x, y, width, height)

    def get_frame_geometry(self, target_w, target_h):
        # Center frame horizontally within the fixed window
        frame_w = target_w - 20
        frame_h = target_h - 20
        frame_x = (self.width() - frame_w) // 2
        # Anchor frame's bottom statically to a margin from window bottom (e.g. 10px margin -> bottom at 310)
        frame_y = (self.height() - 10) - frame_h
        return QRect(frame_x, frame_y, frame_w, frame_h)

    def center_on_screen(self):
        """Positions the overlay bottom-center on the primary available screen."""
        geom = self.get_centered_geometry(self.width(), self.height())
        self.move(geom.topLeft())

    def animate_to_size(self, target_width, target_height):
        """Smoothly resizes only the inner main_frame inside the fixed window."""
        self.current_target_w = target_width
        self.current_target_h = target_height
        
        target_geom = self.get_frame_geometry(target_width, target_height)
        current_geom = self.main_frame.geometry()

        self.frame_anim = QPropertyAnimation(self.main_frame, b"geometry")
        self.frame_anim.setDuration(280)
        self.frame_anim.setStartValue(current_geom)
        self.frame_anim.setEndValue(target_geom)
        self.frame_anim.setEasingCurve(QEasingCurve.OutCubic)

        self.size_anim_group = QParallelAnimationGroup()
        self.size_anim_group.addAnimation(self.frame_anim)
        self.size_anim_group.start()

    def show_state(self, state, text=None):
        """Updates the UI look and dimensions based on state: listening, processing, done, error."""
        target_w, target_h = 220, 56
        is_text_display = False
        
        from PySide6.QtGui import QFontMetrics
        
        if state in ["listening", "processing"]:
            label_text = text.upper() if text else ("GRAVANDO" if state == "listening" else "POLINDO")
            metrics = QFontMetrics(self.label_content.font())
            text_w = metrics.horizontalAdvance(label_text)
            
            if state == "listening":
                # Dynamically calculate width based on layout elements:
                # margin_left(14) + visualizer(50) + spacing(10) + text_w + margin_right(14) + padding_buffer(8) + outer_window_delta(20) = text_w + 116
                target_w = max(text_w + 116, 160)
            else: # processing
                # margin_left(14) + spinner(20) + spacing(10) + text_w + margin_right(14) + padding_buffer(8) + outer_window_delta(20) = text_w + 86
                target_w = max(text_w + 86, 130)
                
            target_w = min(target_w, 480)
                
        elif text and text not in ["CONCLUÍDO!", "COPIADO!"] and state in ["done", "error"]:
            target_w = 460
            is_text_display = True
            
            # Determine display text based on prefixes
            display_text = text
            header_text = ""
            if state == "done":
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
                else:
                    header_text = "TEXTO COPIADO"
                    display_text = text
            elif state == "error":
                header_text = "ERRO!"
            
            # Available text width inside frame
            text_width = 380
            
            # Calculate heights
            header_h = 0
            if header_text:
                header_h = self.calculate_text_height(header_text, self.label_header.font(), text_width) + 2
                
            content_h = self.calculate_text_height(display_text, self.label_content.font(), text_width)
            
            # Use calculated height + margins
            target_h = max(56, header_h + content_h + 40)
            
        self.current_target_w = target_w
        self.current_target_h = target_h

        # Update widget configurations before showing
        self.update_state_widgets(state, text)

        # Adjust dimensions
        if self.is_showing:
            self.animate_to_size(target_w, target_h)
        else:
            geom = self.get_frame_geometry(target_w, target_h)
            self.main_frame.setGeometry(geom)

        # Trigger smooth fade-in if not currently showing
        if not self.is_showing:
            self.fade_in()

        # Automatic dismiss on done/error state
        if state in ["done", "error"]:
            delay = 3500 if is_text_display else 1500
            if state == "error":
                delay = 2500
            QTimer.singleShot(delay, self.fade_out)

    def update_state_widgets(self, state, text=None):
        """Hides and shows the correct visual indicator according to the state."""
        if state == "listening":
            self.label_header.hide()
            self.label_content.setText(text.upper() if text else "GRAVANDO")
            self.label_content.setStyleSheet("color: #ffffff; font-weight: bold; letter-spacing: 0.5px;")
            self.visualizer.show()
            self.spinner.hide()
            self.indicator.hide()
            
        elif state == "processing":
            self.label_header.hide()
            self.label_content.setText(text.upper() if text else "POLINDO")
            self.label_content.setStyleSheet("color: #ffffff; font-weight: bold; letter-spacing: 0.5px;")
            self.visualizer.hide()
            self.spinner.show()
            self.indicator.hide()
            
        elif state == "done":
            if text and text not in ["CONCLUÍDO!", "COPIADO!"]:
                if text.startswith("Traduzido: "):
                    self.label_header.setText("TRADUÇÃO COPIADA")
                    display_text = text[len("Traduzido: "):]
                elif text.startswith("Sem IA / Tradução Falhou: "):
                    self.label_header.setText("SEM CHAVE / TRADUÇÃO FALHOU")
                    display_text = text[len("Sem IA / Tradução Falhou: "):]
                elif text.startswith("Sem IA / Texto Cru: "):
                    self.label_header.setText("TEXTO CRU COPIADO (SEM IA)")
                    display_text = text[len("Sem IA / Texto Cru: "):]
                elif text.startswith("Pesquisando: "):
                    self.label_header.setText("PESQUISANDO NO GOOGLE")
                    display_text = text[len("Pesquisando: "):]
                else:
                    self.label_header.setText("TEXTO COPIADO")
                    display_text = text
                
                self.label_header.show()
                self.label_content.setText(display_text)
                self.label_content.setStyleSheet("color: rgba(255, 255, 255, 220); font-weight: 500;")
            else:
                self.label_header.hide()
                self.label_content.setText(text if text else "COPIADO!")
                self.label_content.setStyleSheet("color: #ffffff; font-weight: bold; letter-spacing: 0.5px;")
                
            self.visualizer.hide()
            self.spinner.hide()
            self.indicator.setStyleSheet("background-color: #ffffff; border-radius: 4px;")
            self.indicator.show()
            
        elif state == "error":
            self.label_header.hide()
            self.label_content.setText(text if text else "ERRO!")
            self.label_content.setStyleSheet("color: #ff5555; font-weight: bold; letter-spacing: 0.5px;")
            self.visualizer.hide()
            self.spinner.hide()
            self.indicator.setStyleSheet("background-color: #ff3333; border-radius: 4px;")
            self.indicator.show()

    def fade_in(self):
        """Triggers a smooth slide-up and fade-in animation of the inner main_frame inside the fixed window."""
        self.is_showing = True
        self.center_on_screen()
        
        target_geom = self.get_frame_geometry(self.current_target_w, self.current_target_h)
        # Start 15px lower
        start_geom = QRect(target_geom.x(), target_geom.y() + 15, target_geom.width(), target_geom.height())
        self.main_frame.setGeometry(start_geom)
        self.setWindowOpacity(0.0)
        self.show()
        
        self.frame_anim = QPropertyAnimation(self.main_frame, b"geometry")
        self.frame_anim.setDuration(250)
        self.frame_anim.setStartValue(start_geom)
        self.frame_anim.setEndValue(target_geom)
        self.frame_anim.setEasingCurve(QEasingCurve.OutCubic)
        
        self.opacity_anim = QPropertyAnimation(self, b"windowOpacity")
        self.opacity_anim.setDuration(250)
        self.opacity_anim.setStartValue(0.0)
        self.opacity_anim.setEndValue(1.0)
        
        self.anim_group = QParallelAnimationGroup()
        self.anim_group.addAnimation(self.frame_anim)
        self.anim_group.addAnimation(self.opacity_anim)
        self.anim_group.start()
        
    def fade_out(self):
        """Triggers a smooth slide-down and fade-out animation."""
        if not self.is_showing:
            return
        self.is_showing = False
        
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
        self.hide()
        self.setWindowOpacity(1.0) # Reset to full opacity for next trigger
        # Reset to initial geometry to prevent jumping next time it appears
        geom = self.get_frame_geometry(220, 56)
        self.main_frame.setGeometry(geom)

    def update_volume_level(self, level):
        """Passes microphone volume levels to the visualizer."""
        self.visualizer.set_amplitude(level)

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

        # Bridge to route hotkey triggers safely to the main GUI thread
        self.hotkey_bridge = HotkeySignalBridge()
        self.hotkey_bridge.triggered.connect(self.toggle_dictation)

        # Hotkey listener initialization with multiple hotkeys dictionary
        self.hotkey_listener = HotkeyListener(self.get_hotkeys_map())
        self.hotkey_listener.start()

        self.setup_tray()

        # Configure registry startup value according to config setting
        set_run_at_startup(self.config_manager.get("start_with_windows", True))

        # Initial check: show warning if API keys are missing on cloud providers
        self.check_api_keys()

        # Check for updates in the background on startup
        self.update_checker = None
        QTimer.singleShot(1500, self.start_background_update_check)

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

    def check_api_keys(self):
        """Warns the user if the selected cloud provider lacks an API key."""
        provider = self.config_manager.get("provider", "gemini")
        if provider in ["gemini", "openai"] and not self.config_manager.get_api_key(provider):
            QTimer.singleShot(1000, self.show_settings_dialog)

    def start_background_update_check(self):
        self.update_checker = UpdateCheckerWorker()
        self.update_checker.update_available.connect(self.show_update_dialog)
        self.update_checker.start()

    @Slot(str, str)
    def show_update_dialog(self, version, download_url):
        self.update_dialog = UpdateDialog(version, download_url)
        self.update_dialog.show()

    def setup_tray(self):
        """Initializes the System Tray Icon and its context menu."""
        self.tray_icon = QSystemTrayIcon(self)
        
        # Load custom app icon if exists
        icon_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "icon.png")
        if os.path.exists(icon_path):
            app_icon = QIcon(icon_path)
            self.tray_icon.setIcon(app_icon)
            self.setWindowIcon(app_icon)
        else:
            self.tray_icon.setIcon(create_color_icon("#ffffff"))
            
        self.tray_icon.setToolTip("FlowVoice - Ditado Inteligente por IA")

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
        style_menu = menu.addMenu("Estilo de Escrita")
        self.action_prof = QAction("Profissional", self, checkable=True)
        self.action_prof.triggered.connect(lambda: self.change_active_style("Profissional"))
        self.action_casual = QAction("Casual", self, checkable=True)
        self.action_casual.triggered.connect(lambda: self.change_active_style("Casual"))
        self.action_raw = QAction("Direto/Cru", self, checkable=True)
        self.action_raw.triggered.connect(lambda: self.change_active_style("Direto"))

        style_menu.addAction(self.action_prof)
        style_menu.addAction(self.action_casual)
        style_menu.addAction(self.action_raw)

        # Mode Sub-menu
        mode_menu = menu.addMenu("Modo de Operação")
        self.action_mode_dictation = QAction("Ditado", self, checkable=True)
        self.action_mode_dictation.triggered.connect(lambda: self.change_operation_mode("ditado"))
        self.action_mode_translation = QAction("Tradução", self, checkable=True)
        self.action_mode_translation.triggered.connect(lambda: self.change_operation_mode("traducao"))
        self.action_mode_search = QAction("Pesquisa Google", self, checkable=True)
        self.action_mode_search.triggered.connect(lambda: self.change_operation_mode("pesquisa"))

        mode_menu.addAction(self.action_mode_dictation)
        mode_menu.addAction(self.action_mode_translation)
        mode_menu.addAction(self.action_mode_search)

        # Translation Language Sub-menu
        lang_menu = menu.addMenu("Idioma de Tradução")
        self.lang_actions = {}
        for lang in ["Inglês", "Espanhol", "Francês", "Alemão", "Italiano"]:
            act = QAction(lang, self, checkable=True)
            act.triggered.connect(lambda checked, l=lang: self.change_translation_target(l))
            lang_menu.addAction(act)
            self.lang_actions[lang] = act

        self.update_menu_checked_states()

        # Settings action
        action_settings = QAction("Configurações...", self)
        action_settings.triggered.connect(self.show_settings_dialog)
        menu.addAction(action_settings)

        menu.addSeparator()

        # Quit action
        action_quit = QAction("Sair", self)
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
        dialog = SettingsDialog(self.config_manager)
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
                
            self.overlay.show_state("done", overlay_text)
            
            # Paste text instantly
            self.paster.paste_text(text)
            
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
    appdata = os.getenv('APPDATA')
    if appdata:
        log_dir = os.path.join(appdata, "FlowVoice")
        os.makedirs(log_dir, exist_ok=True)
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
                
        sys.stdout = StreamToLogger(logging.info)
        sys.stderr = StreamToLogger(logging.error)
        
    logging.info("FlowVoice iniciado.")
    
    app = FlowVoiceApp(sys.argv)
    sys.exit(app.exec())
