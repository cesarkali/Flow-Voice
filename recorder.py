import os
import wave
import queue
import tempfile
import threading
import numpy as np
import sounddevice as sd

class AudioRecorder:
    def __init__(self, samplerate=16000, level_callback=None):
        self.samplerate = samplerate
        self.channels = 1
        self.audio_queue = queue.Queue()
        self.recording = False
        self.thread = None
        self.temp_filepath = None
        self.stream = None
        self.level_callback = level_callback

    def _audio_callback(self, indata, frames, time, status):
        """This is called for each audio block by sounddevice."""
        if status:
            print(f"Status do microfone: {status}")
        self.audio_queue.put(indata.copy())
        if self.level_callback:
            try:
                rms = np.sqrt(np.mean(indata**2))
                self.level_callback(float(rms))
            except:
                pass

    def start(self):
        """Starts recording audio in a background thread."""
        if self.recording:
            return
        
        self.recording = True
        self.audio_queue = queue.Queue()
        
        # Create a temporary file to save the WAV audio
        fd, self.temp_filepath = tempfile.mkstemp(suffix=".wav", prefix="flowvoice_")
        os.close(fd) # Close file descriptor so wave can open it by path safely

        self.thread = threading.Thread(target=self._record_loop, daemon=True)
        self.thread.start()
        print(f"Gravação iniciada. Salvando temporariamente em: {self.temp_filepath}")

    def stop(self):
        """Stops the recording, waits for the thread to finish, and returns the path to the WAV file."""
        if not self.recording:
            return None
        
        self.recording = False
        if self.stream:
            self.stream.stop()
            self.stream.close()
            self.stream = None
            
        if self.thread:
            self.thread.join(timeout=2.0)
            self.thread = None
            
        print("Gravação finalizada.")
        return self.temp_filepath

    def _record_loop(self):
        """Background recording loop."""
        try:
            # Open wave file for writing PCM 16-bit
            wf = wave.open(self.temp_filepath, "wb")
            wf.setnchannels(self.channels)
            wf.setsampwidth(2) # 2 bytes = 16-bit
            wf.setframerate(self.samplerate)

            # Start sounddevice input stream
            self.stream = sd.InputStream(
                samplerate=self.samplerate,
                channels=self.channels,
                callback=self._audio_callback
            )
            self.stream.start()

            while self.recording or not self.audio_queue.empty():
                try:
                    # Retrieve audio block from queue
                    data = self.audio_queue.get(timeout=0.1)
                    # Convert float32 [-1.0, 1.0] from sounddevice to int16 PCM
                    pcm_data = (data * 32767.0).clip(-32768, 32767).astype(np.int16)
                    wf.writeframes(pcm_data.tobytes())
                except queue.Empty:
                    continue
                except Exception as e:
                    print(f"Erro no loop de gravação: {e}")
                    break
            
            wf.close()
        except Exception as e:
            print(f"Erro crítico no gravador de áudio: {e}")
            self.recording = False

    def cleanup(self):
        """Deletes the temporary WAV file if it exists."""
        if self.temp_filepath and os.path.exists(self.temp_filepath):
            try:
                os.remove(self.temp_filepath)
                print(f"Arquivo temporário removido: {self.temp_filepath}")
            except Exception as e:
                print(f"Erro ao remover arquivo temporário: {e}")
            self.temp_filepath = None
