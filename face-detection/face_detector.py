#!/usr/bin/env python3
"""
Face Detection Test Program
Uses YOLOv12 model with multiple modes and configurations
Supports both CLI and Tkinter GUI modes
"""

import argparse
import os
import sys
import time
from datetime import datetime
from pathlib import Path

import cv2
from ultralytics import YOLO


class FaceDetector:
    """Face detector using YOLOv12 model."""
    
    def __init__(self, model_path: str = None, confidence: float = 0.5):
        if model_path is None:
            model_path = Path(__file__).parent / "yolov12n-face.pt"
        
        self.model = YOLO(str(model_path))
        self.confidence = confidence
        self.colors = {
            'box': (0, 255, 0),
            'text_bg': (0, 255, 0),
            'text': (0, 0, 0)
        }
    
    def detect(self, frame, use_tracking=False):
        if use_tracking:
            results = self.model.track(frame, conf=self.confidence, verbose=False, persist=True)
        else:
            results = self.model(frame, conf=self.confidence, verbose=False)
        detections = []
        
        for result in results:
            if result.boxes is not None:
                for i, box in enumerate(result.boxes):
                    x1, y1, x2, y2 = box.xyxy[0].cpu().numpy()
                    conf = float(box.conf[0])
                    cls = int(box.cls[0])
                    # Get track ID if available
                    track_id = None
                    if use_tracking and result.boxes.id is not None:
                        track_id = int(result.boxes.id[i])
                    detections.append((int(x1), int(y1), int(x2), int(y2), conf, cls, track_id))
        
        return detections
    
    def draw_detections(self, frame, detections, show_confidence=True, box_thickness=2, show_track_id=False):
        annotated = frame.copy()
        
        # Color palette for tracking
        track_colors = [
            (255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0),
            (255, 0, 255), (0, 255, 255), (128, 0, 255), (255, 128, 0),
            (0, 128, 255), (128, 255, 0), (255, 0, 128), (0, 255, 128)
        ]
        
        for detection in detections:
            x1, y1, x2, y2, conf, cls = detection[:6]
            track_id = detection[6] if len(detection) > 6 else None
            
            # Choose color based on track ID or default
            if track_id is not None and show_track_id:
                color = track_colors[track_id % len(track_colors)]
            else:
                color = self.colors['box']
            
            cv2.rectangle(annotated, (x1, y1), (x2, y2), color, box_thickness)
            
            if show_confidence or (show_track_id and track_id is not None):
                parts = []
                if show_track_id and track_id is not None:
                    parts.append(f"ID:{track_id}")
                if show_confidence:
                    parts.append(f"{conf:.2f}")
                label = " ".join(parts)
                
                (label_w, label_h), baseline = cv2.getTextSize(
                    label, cv2.FONT_HERSHEY_SIMPLEX, 0.6, 2
                )
                cv2.rectangle(
                    annotated,
                    (x1, y1 - label_h - 10),
                    (x1 + label_w + 5, y1),
                    color,
                    -1
                )
                cv2.putText(
                    annotated,
                    label,
                    (x1 + 2, y1 - 5),
                    cv2.FONT_HERSHEY_SIMPLEX,
                    0.6,
                    (255, 255, 255),
                    2
                )
        
        return annotated
    
    def crop_faces(self, frame, detections, padding=10):
        """Extract cropped face images from frame."""
        crops = []
        h, w = frame.shape[:2]
        
        for (x1, y1, x2, y2, conf, cls) in detections:
            # Add padding
            x1_pad = max(0, x1 - padding)
            y1_pad = max(0, y1 - padding)
            x2_pad = min(w, x2 + padding)
            y2_pad = min(h, y2 + padding)
            
            crop = frame[y1_pad:y2_pad, x1_pad:x2_pad]
            crops.append((crop, conf))
        
        return crops


def run_cli(camera_id: int = 0, confidence: float = 0.5, model_path: str = None):
    """Run face detection in CLI mode with OpenCV window."""
    print("=" * 50)
    print("  Face Detection - CLI Mode")
    print("=" * 50)
    print(f"Camera ID: {camera_id}")
    print(f"Confidence: {confidence}")
    print("Press 'q' to quit, 's' to save screenshot")
    print("=" * 50)
    
    detector = FaceDetector(model_path, confidence)
    cap = cv2.VideoCapture(camera_id)
    
    if not cap.isOpened():
        print(f"Error: Could not open camera {camera_id}")
        sys.exit(1)
    
    cap.set(cv2.CAP_PROP_FRAME_WIDTH, 1280)
    cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 720)
    
    fps_counter = 0
    fps_start = time.time()
    fps = 0
    
    try:
        while True:
            ret, frame = cap.read()
            if not ret:
                print("Error: Could not read frame")
                break
            
            detections = detector.detect(frame)
            annotated = detector.draw_detections(frame, detections)
            
            fps_counter += 1
            if time.time() - fps_start >= 1.0:
                fps = fps_counter
                fps_counter = 0
                fps_start = time.time()
            
            stats = f"FPS: {fps} | Faces: {len(detections)}"
            cv2.putText(annotated, stats, (10, 30), cv2.FONT_HERSHEY_SIMPLEX, 0.8, (0, 255, 255), 2)
            cv2.imshow("Face Detection", annotated)
            
            key = cv2.waitKey(1) & 0xFF
            if key == ord('q'):
                break
            elif key == ord('s'):
                filename = f"screenshot_{int(time.time())}.png"
                cv2.imwrite(filename, annotated)
                print(f"Screenshot saved: {filename}")
    
    finally:
        cap.release()
        cv2.destroyAllWindows()
        print("\nDetection stopped.")


def run_gui(camera_id: int = 0, confidence: float = 0.5, model_path: str = None):
    """Run face detection with advanced Tkinter GUI."""
    import tkinter as tk
    from tkinter import ttk, filedialog, messagebox
    from PIL import Image, ImageTk
    
    class FaceDetectionApp:
        def __init__(self, root):
            self.root = root
            self.root.title("Face Detection - YOLOv12")
            self.root.configure(bg='#1a1a2e')
            self.root.geometry("1100x800")
            self.root.minsize(900, 700)
            
            # State variables
            self.running = False
            self.detector = None
            self.cap = None
            self.current_frame = None
            self.fps = 0
            self.fps_counter = 0
            self.fps_start = time.time()
            self.total_faces_detected = 0
            self.processed_files = 0
            
            # Configuration variables
            self.mode = tk.StringVar(value="webcam")
            self.camera_id = tk.IntVar(value=camera_id)
            self.confidence = tk.DoubleVar(value=confidence)
            self.show_confidence = tk.BooleanVar(value=True)
            self.auto_save = tk.BooleanVar(value=False)
            self.save_crops = tk.BooleanVar(value=False)
            self.output_dir = tk.StringVar(value=str(Path.home() / "face_detection_output"))
            self.box_thickness = tk.IntVar(value=2)
            self.crop_padding = tk.IntVar(value=10)
            self.continuous_mode = tk.BooleanVar(value=True)
            self.file_action = tk.StringVar(value="display")
            self.input_path = tk.StringVar(value="")
            
            # IP Camera settings
            self.ip_address = tk.StringVar(value="10.137.84.153")
            self.ip_port = tk.StringVar(value="8080")
            self.ip_stream_type = tk.StringVar(value="mjpeg")  # mjpeg or shot
            self.shot_interval = tk.IntVar(value=100)  # ms between shot.jpg requests
            
            # Model selection
            self.available_models = [
                "yolov12n-face.pt",
                "yolov12s-face.pt",
                "yolov12m-face.pt",
                "yolov12l-face.pt"
            ]
            self.selected_model = tk.StringVar(value="yolov12n-face.pt")
            
            # Detection mode (detect or track)
            self.detection_mode = tk.StringVar(value="detect")
            
            self.setup_styles()
            self.setup_ui()
            self.root.protocol("WM_DELETE_WINDOW", self.on_close)
        
        def setup_styles(self):
            """Configure ttk styles for modern look."""
            style = ttk.Style()
            style.theme_use('clam')
            
            # Colors
            bg_dark = '#1a1a2e'
            bg_medium = '#16213e'
            bg_light = '#0f3460'
            accent = '#e94560'
            text = '#eaeaea'
            
            style.configure('TFrame', background=bg_dark)
            style.configure('Card.TFrame', background=bg_medium)
            style.configure('TLabel', background=bg_dark, foreground=text, font=('Segoe UI', 10))
            style.configure('Header.TLabel', background=bg_dark, foreground=text, font=('Segoe UI', 14, 'bold'))
            style.configure('Title.TLabel', background=bg_dark, foreground=accent, font=('Segoe UI', 18, 'bold'))
            style.configure('Stats.TLabel', background=bg_medium, foreground='#00ff88', font=('Consolas', 11))
            style.configure('TButton', padding=8, font=('Segoe UI', 10))
            style.configure('Accent.TButton', background=accent, foreground='white')
            style.configure('TRadiobutton', background=bg_dark, foreground=text, font=('Segoe UI', 10))
            style.configure('TCheckbutton', background=bg_dark, foreground=text, font=('Segoe UI', 10))
            style.configure('TLabelframe', background=bg_dark, foreground=text)
            style.configure('TLabelframe.Label', background=bg_dark, foreground=accent, font=('Segoe UI', 11, 'bold'))
            style.configure('TScale', background=bg_dark)
            style.configure('TEntry', fieldbackground=bg_medium, foreground=text)
            style.configure('TSpinbox', fieldbackground=bg_medium, foreground=text)
        
        def setup_ui(self):
            """Setup the main user interface."""
            # Main container with padding
            main_frame = ttk.Frame(self.root, padding="15")
            main_frame.pack(fill=tk.BOTH, expand=True)
            
            # Title
            title = ttk.Label(main_frame, text="🎯 Face Detection Studio", style='Title.TLabel')
            title.pack(pady=(0, 15))
            
            # Content area - horizontal split
            content = ttk.Frame(main_frame)
            content.pack(fill=tk.BOTH, expand=True)
            
            # Left panel - Configuration
            left_panel = ttk.Frame(content, width=350)
            left_panel.pack(side=tk.LEFT, fill=tk.Y, padx=(0, 10))
            left_panel.pack_propagate(False)
            
            self.setup_config_panel(left_panel)
            
            # Right panel - Preview and output
            right_panel = ttk.Frame(content)
            right_panel.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
            
            self.setup_preview_panel(right_panel)
            
            # Bottom - Controls and stats
            bottom_frame = ttk.Frame(main_frame)
            bottom_frame.pack(fill=tk.X, pady=(10, 0))
            
            self.setup_controls(bottom_frame)
        
        def setup_config_panel(self, parent):
            """Setup the configuration panel."""
            # Mode Selection
            mode_frame = ttk.LabelFrame(parent, text="📷 Mode", padding="10")
            mode_frame.pack(fill=tk.X, pady=(0, 10))
            
            modes = [
                ("Webcam (continu)", "webcam"),
                ("Caméra IP (MJPEG)", "ipcam"),
                ("Fichier image", "file"),
                ("Dossier d'images", "folder"),
                ("Fichier vidéo", "video")
            ]
            
            for text, value in modes:
                rb = ttk.Radiobutton(mode_frame, text=text, variable=self.mode, 
                                     value=value, command=self.on_mode_change)
                rb.pack(anchor=tk.W, pady=2)
            
            # Input Selection
            input_frame = ttk.LabelFrame(parent, text="📁 Entrée", padding="10")
            input_frame.pack(fill=tk.X, pady=(0, 10))
            
            # Camera ID (for webcam mode)
            self.camera_frame = ttk.Frame(input_frame)
            self.camera_frame.pack(fill=tk.X, pady=2)
            ttk.Label(self.camera_frame, text="Caméra ID:").pack(side=tk.LEFT)
            self.camera_spin = ttk.Spinbox(self.camera_frame, from_=0, to=10, 
                                           textvariable=self.camera_id, width=5)
            self.camera_spin.pack(side=tk.RIGHT)
            
            # IP Camera settings (for ipcam mode)
            self.ipcam_frame = ttk.Frame(input_frame)
            
            # IP Address row
            ip_row = ttk.Frame(self.ipcam_frame)
            ip_row.pack(fill=tk.X, pady=2)
            ttk.Label(ip_row, text="IP:").pack(side=tk.LEFT)
            ttk.Entry(ip_row, textvariable=self.ip_address, width=15).pack(side=tk.LEFT, padx=5)
            ttk.Label(ip_row, text="Port:").pack(side=tk.LEFT)
            ttk.Entry(ip_row, textvariable=self.ip_port, width=6).pack(side=tk.LEFT, padx=5)
            
            # Stream type row
            stream_row = ttk.Frame(self.ipcam_frame)
            stream_row.pack(fill=tk.X, pady=2)
            ttk.Label(stream_row, text="Type:").pack(side=tk.LEFT)
            ttk.Radiobutton(stream_row, text="MJPEG (/video)", variable=self.ip_stream_type, 
                           value="mjpeg").pack(side=tk.LEFT, padx=5)
            ttk.Radiobutton(stream_row, text="Shot (/shot.jpg)", variable=self.ip_stream_type, 
                           value="shot").pack(side=tk.LEFT, padx=5)
            
            # Shot interval row
            interval_row = ttk.Frame(self.ipcam_frame)
            interval_row.pack(fill=tk.X, pady=2)
            ttk.Label(interval_row, text="Intervalle shot (ms):").pack(side=tk.LEFT)
            ttk.Spinbox(interval_row, from_=50, to=5000, textvariable=self.shot_interval, 
                       width=6).pack(side=tk.RIGHT)
            
            # File/Folder selection (for file modes)
            self.file_frame = ttk.Frame(input_frame)
            self.path_entry = ttk.Entry(self.file_frame, textvariable=self.input_path, width=25)
            self.path_entry.pack(side=tk.LEFT, fill=tk.X, expand=True)
            self.browse_btn = ttk.Button(self.file_frame, text="...", width=3, 
                                         command=self.browse_input)
            self.browse_btn.pack(side=tk.RIGHT, padx=(5, 0))
            
            # Detection Settings
            detect_frame = ttk.LabelFrame(parent, text="🔍 Détection", padding="10")
            detect_frame.pack(fill=tk.X, pady=(0, 10))
            
            # Model selection
            model_frame = ttk.Frame(detect_frame)
            model_frame.pack(fill=tk.X, pady=2)
            ttk.Label(model_frame, text="Modèle:").pack(side=tk.LEFT)
            self.model_combo = ttk.Combobox(
                model_frame, 
                textvariable=self.selected_model,
                values=self.available_models,
                state="readonly",
                width=18
            )
            self.model_combo.pack(side=tk.RIGHT)
            self.model_combo.bind("<<ComboboxSelected>>", self.on_model_change)
            
            # Confidence slider
            conf_frame = ttk.Frame(detect_frame)
            conf_frame.pack(fill=tk.X, pady=2)
            ttk.Label(conf_frame, text="Confiance:").pack(side=tk.LEFT)
            self.conf_label = ttk.Label(conf_frame, text=f"{confidence:.2f}")
            self.conf_label.pack(side=tk.RIGHT)
            
            self.conf_slider = ttk.Scale(detect_frame, from_=0.1, to=1.0, 
                                         variable=self.confidence, 
                                         command=self.on_confidence_change)
            self.conf_slider.pack(fill=tk.X, pady=2)
            
            # Box thickness
            thick_frame = ttk.Frame(detect_frame)
            thick_frame.pack(fill=tk.X, pady=2)
            ttk.Label(thick_frame, text="Épaisseur cadre:").pack(side=tk.LEFT)
            ttk.Spinbox(thick_frame, from_=1, to=10, textvariable=self.box_thickness, 
                        width=5).pack(side=tk.RIGHT)
            
            # Show confidence checkbox
            ttk.Checkbutton(detect_frame, text="Afficher confiance", 
                           variable=self.show_confidence).pack(anchor=tk.W, pady=2)
            
            # Detection mode selection (detect vs track)
            mode_select_frame = ttk.Frame(detect_frame)
            mode_select_frame.pack(fill=tk.X, pady=5)
            ttk.Label(mode_select_frame, text="Mode:").pack(side=tk.LEFT)
            ttk.Radiobutton(mode_select_frame, text="Detect", variable=self.detection_mode, 
                           value="detect").pack(side=tk.LEFT, padx=5)
            ttk.Radiobutton(mode_select_frame, text="Track", variable=self.detection_mode, 
                           value="track").pack(side=tk.LEFT, padx=5)
            
            # Output Settings
            output_frame = ttk.LabelFrame(parent, text="💾 Sortie", padding="10")
            output_frame.pack(fill=tk.X, pady=(0, 10))
            
            # File action (for file mode)
            self.action_frame = ttk.Frame(output_frame)
            ttk.Label(self.action_frame, text="Action:").pack(anchor=tk.W)
            actions = [
                ("Afficher seulement", "display"),
                ("Sauvegarder annoté", "save"),
                ("Afficher + Sauvegarder", "both")
            ]
            for text, value in actions:
                ttk.Radiobutton(self.action_frame, text=text, variable=self.file_action, 
                               value=value).pack(anchor=tk.W)
            
            # Auto-save options
            ttk.Checkbutton(output_frame, text="Sauvegarde auto screenshots", 
                           variable=self.auto_save).pack(anchor=tk.W, pady=2)
            ttk.Checkbutton(output_frame, text="Extraire visages (crops)", 
                           variable=self.save_crops).pack(anchor=tk.W, pady=2)
            
            # Crop padding
            pad_frame = ttk.Frame(output_frame)
            pad_frame.pack(fill=tk.X, pady=2)
            ttk.Label(pad_frame, text="Padding crops (px):").pack(side=tk.LEFT)
            ttk.Spinbox(pad_frame, from_=0, to=50, textvariable=self.crop_padding, 
                        width=5).pack(side=tk.RIGHT)
            
            # Output directory
            dir_frame = ttk.Frame(output_frame)
            dir_frame.pack(fill=tk.X, pady=5)
            ttk.Label(dir_frame, text="Dossier de sortie:").pack(anchor=tk.W)
            
            dir_input = ttk.Frame(dir_frame)
            dir_input.pack(fill=tk.X)
            ttk.Entry(dir_input, textvariable=self.output_dir, width=25).pack(side=tk.LEFT, fill=tk.X, expand=True)
            ttk.Button(dir_input, text="...", width=3, 
                      command=self.browse_output_dir).pack(side=tk.RIGHT, padx=(5, 0))
            
            # Video/Continuous options
            cont_frame = ttk.LabelFrame(parent, text="🔄 Options Continu", padding="10")
            cont_frame.pack(fill=tk.X, pady=(0, 10))
            
            ttk.Checkbutton(cont_frame, text="Mode continu (vidéo/webcam)", 
                           variable=self.continuous_mode).pack(anchor=tk.W, pady=2)
            
            # Update UI based on initial mode
            self.on_mode_change()
        
        def setup_preview_panel(self, parent):
            """Setup the preview panel."""
            # Preview frame with border
            preview_container = ttk.Frame(parent, style='Card.TFrame')
            preview_container.pack(fill=tk.BOTH, expand=True)
            
            self.video_label = tk.Label(preview_container, bg='#16213e')
            self.video_label.pack(fill=tk.BOTH, expand=True, padx=2, pady=2)
            
            # Create placeholder
            self.show_placeholder("Sélectionnez un mode et cliquez sur Démarrer")
        
        def setup_controls(self, parent):
            """Setup control buttons and stats."""
            # Stats frame
            stats_frame = ttk.Frame(parent, style='Card.TFrame', padding="10")
            stats_frame.pack(fill=tk.X, pady=(0, 10))
            
            stats_inner = ttk.Frame(stats_frame, style='Card.TFrame')
            stats_inner.pack(fill=tk.X)
            
            self.fps_label = ttk.Label(stats_inner, text="FPS: --", style='Stats.TLabel')
            self.fps_label.pack(side=tk.LEFT, padx=15)
            
            self.faces_label = ttk.Label(stats_inner, text="Visages: --", style='Stats.TLabel')
            self.faces_label.pack(side=tk.LEFT, padx=15)
            
            self.total_label = ttk.Label(stats_inner, text="Total détecté: 0", style='Stats.TLabel')
            self.total_label.pack(side=tk.LEFT, padx=15)
            
            self.status_label = ttk.Label(stats_inner, text="Status: Arrêté", style='Stats.TLabel')
            self.status_label.pack(side=tk.RIGHT, padx=15)
            
            # Buttons frame
            btn_frame = ttk.Frame(parent)
            btn_frame.pack(fill=tk.X)
            
            self.start_btn = ttk.Button(btn_frame, text="▶ Démarrer", 
                                        command=self.start_detection, width=15)
            self.start_btn.pack(side=tk.LEFT, padx=5)
            
            self.stop_btn = ttk.Button(btn_frame, text="⏹ Arrêter", 
                                       command=self.stop_detection, width=15, state=tk.DISABLED)
            self.stop_btn.pack(side=tk.LEFT, padx=5)
            
            self.screenshot_btn = ttk.Button(btn_frame, text="📷 Capture", 
                                             command=self.save_screenshot, width=15, state=tk.DISABLED)
            self.screenshot_btn.pack(side=tk.LEFT, padx=5)
            
            ttk.Button(btn_frame, text="📂 Ouvrir sortie", 
                      command=self.open_output_dir, width=15).pack(side=tk.LEFT, padx=5)
            
            ttk.Button(btn_frame, text="✕ Quitter", 
                      command=self.on_close, width=15).pack(side=tk.RIGHT, padx=5)
        
        def show_placeholder(self, text):
            """Show placeholder image with text."""
            placeholder = Image.new('RGB', (640, 480), color='#16213e')
            self.placeholder_img = ImageTk.PhotoImage(placeholder)
            self.video_label.configure(image=self.placeholder_img, text=text, 
                                       compound=tk.CENTER, fg='#666')
        
        def on_mode_change(self):
            """Handle mode selection change."""
            mode = self.mode.get()
            
            # Hide all input frames first
            self.camera_frame.pack_forget()
            self.ipcam_frame.pack_forget()
            self.file_frame.pack_forget()
            self.action_frame.pack_forget()
            
            # Show appropriate frame
            if mode == "webcam":
                self.camera_frame.pack(fill=tk.X, pady=2)
            elif mode == "ipcam":
                self.ipcam_frame.pack(fill=tk.X, pady=2)
            else:
                self.file_frame.pack(fill=tk.X, pady=2)
                self.action_frame.pack(fill=tk.X, pady=5)
        
        def on_model_change(self, event=None):
            """Handle model selection change."""
            if self.detector:
                # Force reload of detector with new model
                self.detector = None
                self.status_label.configure(text="Modèle changé - redémarrer")
        
        def on_confidence_change(self, value):
            """Handle confidence slider change."""
            conf = self.confidence.get()
            self.conf_label.configure(text=f"{conf:.2f}")
            if self.detector:
                self.detector.confidence = conf
        
        def browse_input(self):
            """Open file/folder browser based on mode."""
            mode = self.mode.get()
            
            if mode == "file":
                path = filedialog.askopenfilename(
                    title="Sélectionner une image",
                    filetypes=[("Images", "*.jpg *.jpeg *.png *.bmp *.webp")]
                )
            elif mode == "folder":
                path = filedialog.askdirectory(title="Sélectionner un dossier")
            elif mode == "video":
                path = filedialog.askopenfilename(
                    title="Sélectionner une vidéo",
                    filetypes=[("Vidéos", "*.mp4 *.avi *.mkv *.mov *.webm")]
                )
            else:
                return
            
            if path:
                self.input_path.set(path)
        
        def browse_output_dir(self):
            """Open folder browser for output directory."""
            path = filedialog.askdirectory(title="Sélectionner le dossier de sortie")
            if path:
                self.output_dir.set(path)
        
        def open_output_dir(self):
            """Open output directory in file explorer."""
            output = self.output_dir.get()
            if os.path.exists(output):
                os.startfile(output)
            else:
                messagebox.showinfo("Info", "Le dossier de sortie n'existe pas encore.")
        
        def ensure_output_dir(self):
            """Create output directory if it doesn't exist."""
            output = Path(self.output_dir.get())
            output.mkdir(parents=True, exist_ok=True)
            return output
        
        def start_detection(self):
            """Start detection based on selected mode."""
            mode = self.mode.get()
            self.status_label.configure(text="Status: Chargement...")
            self.root.update()
            
            # Get selected model path
            selected_model_name = self.selected_model.get()
            selected_model_path = Path(__file__).parent / selected_model_name
            
            # Initialize detector if needed
            if self.detector is None:
                self.detector = FaceDetector(str(selected_model_path), self.confidence.get())
            else:
                self.detector.confidence = self.confidence.get()
            
            if mode == "webcam":
                self.start_webcam()
            elif mode == "ipcam":
                self.start_ipcam()
            elif mode == "file":
                self.process_file()
            elif mode == "folder":
                self.process_folder()
            elif mode == "video":
                self.process_video()
        
        def get_ipcam_url(self):
            """Build IP camera URL based on settings."""
            ip = self.ip_address.get()
            port = self.ip_port.get()
            stream_type = self.ip_stream_type.get()
            
            if stream_type == "mjpeg":
                return f"http://{ip}:{port}/video"
            else:
                return f"http://{ip}:{port}/shot.jpg"
        
        def start_ipcam(self):
            """Start IP camera detection."""
            stream_type = self.ip_stream_type.get()
            url = self.get_ipcam_url()
            
            if stream_type == "mjpeg":
                # MJPEG stream - use OpenCV VideoCapture
                self.cap = cv2.VideoCapture(url)
                if not self.cap.isOpened():
                    messagebox.showerror("Erreur", f"Impossible de se connecter \u00e0 {url}")
                    return
                
                self.running = True
                self.fps_counter = 0
                self.fps_start = time.time()
                
                self.start_btn.configure(state=tk.DISABLED)
                self.stop_btn.configure(state=tk.NORMAL)
                self.screenshot_btn.configure(state=tk.NORMAL)
                self.status_label.configure(text=f"IP Cam: {url}")
                
                self.update_webcam_frame()
            else:
                # Shot mode - fetch individual frames
                self.running = True
                self.fps_counter = 0
                self.fps_start = time.time()
                
                self.start_btn.configure(state=tk.DISABLED)
                self.stop_btn.configure(state=tk.NORMAL)
                self.screenshot_btn.configure(state=tk.NORMAL)
                self.status_label.configure(text=f"IP Cam Shot: {url}")
                
                self.update_ipcam_shot()
        
        def update_ipcam_shot(self):
            """Update frame from IP camera shot.jpg endpoint."""
            import urllib.request
            import numpy as np
            
            if not self.running:
                return
            
            try:
                url = self.get_ipcam_url()
                with urllib.request.urlopen(url, timeout=5) as response:
                    img_array = np.asarray(bytearray(response.read()), dtype=np.uint8)
                    frame = cv2.imdecode(img_array, cv2.IMREAD_COLOR)
                    
                    if frame is None:
                        self.root.after(100, self.update_ipcam_shot)
                        return
                    
                    # Detect and draw
                    use_tracking = self.detection_mode.get() == "track"
                    detections = self.detector.detect(frame, use_tracking=use_tracking)
                    annotated = self.detector.draw_detections(
                        frame, detections,
                        show_confidence=self.show_confidence.get(),
                        box_thickness=self.box_thickness.get(),
                        show_track_id=use_tracking
                    )
                    
                    self.current_frame = annotated
                    self.total_faces_detected += len(detections)
                    
                    # Save crops if enabled
                    if self.save_crops.get() and detections:
                        self.save_face_crops(frame, detections)
                    
                    # Update stats
                    self.fps_counter += 1
                    if time.time() - self.fps_start >= 1.0:
                        self.fps = self.fps_counter
                        self.fps_counter = 0
                        self.fps_start = time.time()
                        self.fps_label.configure(text=f"FPS: {self.fps}")
                    
                    self.faces_label.configure(text=f"Visages: {len(detections)}")
                    self.total_label.configure(text=f"Total détecté: {self.total_faces_detected}")
                    
                    # Display frame
                    self.display_frame(annotated)
                    
            except Exception as e:
                self.status_label.configure(text=f"Erreur: {str(e)[:30]}")
            
            # Schedule next update (slower for shot mode)
            self.root.after(self.shot_interval.get(), self.update_ipcam_shot)
        
        def start_webcam(self):
            """Start webcam detection."""
            self.cap = cv2.VideoCapture(self.camera_id.get())
            if not self.cap.isOpened():
                messagebox.showerror("Erreur", f"Impossible d'ouvrir la caméra {self.camera_id.get()}")
                return
            
            self.cap.set(cv2.CAP_PROP_FRAME_WIDTH, 1280)
            self.cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 720)
            
            self.running = True
            self.fps_counter = 0
            self.fps_start = time.time()
            
            self.start_btn.configure(state=tk.DISABLED)
            self.stop_btn.configure(state=tk.NORMAL)
            self.screenshot_btn.configure(state=tk.NORMAL)
            self.status_label.configure(text="Status: En cours")
            
            self.update_webcam_frame()
        
        def update_webcam_frame(self):
            """Update webcam frame."""
            if not self.running:
                return
            
            ret, frame = self.cap.read()
            if not ret:
                self.stop_detection()
                return
            
            # Detect and draw
            use_tracking = self.detection_mode.get() == "track"
            detections = self.detector.detect(frame, use_tracking=use_tracking)
            annotated = self.detector.draw_detections(
                frame, detections, 
                show_confidence=self.show_confidence.get(),
                box_thickness=self.box_thickness.get(),
                show_track_id=use_tracking
            )
            
            self.current_frame = annotated
            self.total_faces_detected += len(detections)
            
            # Save crops if enabled
            if self.save_crops.get() and detections:
                self.save_face_crops(frame, detections)
            
            # Update stats
            self.fps_counter += 1
            if time.time() - self.fps_start >= 1.0:
                self.fps = self.fps_counter
                self.fps_counter = 0
                self.fps_start = time.time()
                self.fps_label.configure(text=f"FPS: {self.fps}")
            
            self.faces_label.configure(text=f"Visages: {len(detections)}")
            self.total_label.configure(text=f"Total détecté: {self.total_faces_detected}")
            
            # Display frame
            self.display_frame(annotated)
            
            # Schedule next update
            self.root.after(10, self.update_webcam_frame)
        
        def process_file(self):
            """Process a single image file."""
            path = self.input_path.get()
            if not path or not os.path.exists(path):
                messagebox.showerror("Erreur", "Veuillez sélectionner un fichier valide.")
                return
            
            frame = cv2.imread(path)
            if frame is None:
                messagebox.showerror("Erreur", "Impossible de lire l'image.")
                return
            
            use_tracking = self.detection_mode.get() == "track"
            detections = self.detector.detect(frame, use_tracking=use_tracking)
            annotated = self.detector.draw_detections(
                frame, detections,
                show_confidence=self.show_confidence.get(),
                box_thickness=self.box_thickness.get(),
                show_track_id=use_tracking
            )
            
            self.current_frame = annotated
            self.total_faces_detected += len(detections)
            
            action = self.file_action.get()
            
            # Display
            if action in ("display", "both"):
                self.display_frame(annotated)
            
            # Save
            if action in ("save", "both"):
                output_dir = self.ensure_output_dir()
                output_path = output_dir / f"detected_{Path(path).name}"
                cv2.imwrite(str(output_path), annotated)
                self.status_label.configure(text=f"Sauvegardé: {output_path.name}")
            
            # Save crops
            if self.save_crops.get() and detections:
                self.save_face_crops(frame, detections)
            
            self.faces_label.configure(text=f"Visages: {len(detections)}")
            self.total_label.configure(text=f"Total détecté: {self.total_faces_detected}")
            self.status_label.configure(text=f"Terminé: {len(detections)} visage(s)")
        
        def process_folder(self):
            """Process all images in a folder."""
            folder = self.input_path.get()
            if not folder or not os.path.isdir(folder):
                messagebox.showerror("Erreur", "Veuillez sélectionner un dossier valide.")
                return
            
            extensions = ('.jpg', '.jpeg', '.png', '.bmp', '.webp')
            files = [f for f in Path(folder).iterdir() 
                     if f.is_file() and f.suffix.lower() in extensions]
            
            if not files:
                messagebox.showinfo("Info", "Aucune image trouvée dans le dossier.")
                return
            
            self.running = True
            self.start_btn.configure(state=tk.DISABLED)
            self.stop_btn.configure(state=tk.NORMAL)
            
            output_dir = self.ensure_output_dir() if self.file_action.get() != "display" else None
            
            for i, file_path in enumerate(files):
                if not self.running:
                    break
                
                self.status_label.configure(text=f"Traitement: {i+1}/{len(files)}")
                self.root.update()
                
                frame = cv2.imread(str(file_path))
                if frame is None:
                    continue
                
                use_tracking = self.detection_mode.get() == "track"
                detections = self.detector.detect(frame, use_tracking=use_tracking)
                annotated = self.detector.draw_detections(
                    frame, detections,
                    show_confidence=self.show_confidence.get(),
                    box_thickness=self.box_thickness.get(),
                    show_track_id=use_tracking
                )
                
                self.current_frame = annotated
                self.processed_files += 1
                self.total_faces_detected += len(detections)
                
                # Display
                if self.file_action.get() in ("display", "both"):
                    self.display_frame(annotated)
                
                # Save
                if output_dir and self.file_action.get() in ("save", "both"):
                    output_path = output_dir / f"detected_{file_path.name}"
                    cv2.imwrite(str(output_path), annotated)
                
                # Save crops
                if self.save_crops.get() and detections:
                    self.save_face_crops(frame, detections, prefix=file_path.stem)
                
                self.faces_label.configure(text=f"Visages: {len(detections)}")
                self.total_label.configure(text=f"Total: {self.total_faces_detected}")
                
                self.root.update()
            
            self.stop_detection()
            self.status_label.configure(text=f"Terminé: {self.processed_files} fichiers")
        
        def process_video(self):
            """Process a video file."""
            path = self.input_path.get()
            if not path or not os.path.exists(path):
                messagebox.showerror("Erreur", "Veuillez sélectionner un fichier vidéo valide.")
                return
            
            self.cap = cv2.VideoCapture(path)
            if not self.cap.isOpened():
                messagebox.showerror("Erreur", "Impossible d'ouvrir la vidéo.")
                return
            
            self.running = True
            self.fps_counter = 0
            self.fps_start = time.time()
            
            self.start_btn.configure(state=tk.DISABLED)
            self.stop_btn.configure(state=tk.NORMAL)
            self.screenshot_btn.configure(state=tk.NORMAL)
            self.status_label.configure(text="Status: Lecture vidéo")
            
            self.update_video_frame()
        
        def update_video_frame(self):
            """Update video frame."""
            if not self.running or not self.cap:
                return
            
            ret, frame = self.cap.read()
            if not ret:
                if self.continuous_mode.get():
                    self.cap.set(cv2.CAP_PROP_POS_FRAMES, 0)
                    ret, frame = self.cap.read()
                    if not ret:
                        self.stop_detection()
                        return
                else:
                    self.stop_detection()
                    self.status_label.configure(text="Status: Vidéo terminée")
                    return
            
            use_tracking = self.detection_mode.get() == "track"
            detections = self.detector.detect(frame, use_tracking=use_tracking)
            annotated = self.detector.draw_detections(
                frame, detections,
                show_confidence=self.show_confidence.get(),
                box_thickness=self.box_thickness.get(),
                show_track_id=use_tracking
            )
            
            self.current_frame = annotated
            self.total_faces_detected += len(detections)
            
            # Update stats
            self.fps_counter += 1
            if time.time() - self.fps_start >= 1.0:
                self.fps = self.fps_counter
                self.fps_counter = 0
                self.fps_start = time.time()
                self.fps_label.configure(text=f"FPS: {self.fps}")
            
            self.faces_label.configure(text=f"Visages: {len(detections)}")
            self.total_label.configure(text=f"Total: {self.total_faces_detected}")
            
            self.display_frame(annotated)
            
            self.root.after(1, self.update_video_frame)
        
        def display_frame(self, frame):
            """Display a frame in the preview area."""
            rgb_frame = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            img = Image.fromarray(rgb_frame)
            
            # Resize to fit while maintaining aspect ratio
            label_w = self.video_label.winfo_width()
            label_h = self.video_label.winfo_height()
            
            if label_w > 10 and label_h > 10:
                img_ratio = img.width / img.height
                label_ratio = label_w / label_h
                
                if img_ratio > label_ratio:
                    new_w = label_w
                    new_h = int(label_w / img_ratio)
                else:
                    new_h = label_h
                    new_w = int(label_h * img_ratio)
                
                img = img.resize((new_w, new_h), Image.Resampling.LANCZOS)
            
            self.photo = ImageTk.PhotoImage(img)
            self.video_label.configure(image=self.photo, text='')
        
        def save_screenshot(self):
            """Save current frame as screenshot."""
            if self.current_frame is not None:
                output_dir = self.ensure_output_dir()
                timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
                filename = output_dir / f"screenshot_{timestamp}.png"
                cv2.imwrite(str(filename), self.current_frame)
                self.status_label.configure(text=f"Capture: {filename.name}")
        
        def save_face_crops(self, frame, detections, prefix=""):
            """Save cropped face images."""
            output_dir = self.ensure_output_dir() / "crops"
            output_dir.mkdir(exist_ok=True)
            
            crops = self.detector.crop_faces(frame, detections, self.crop_padding.get())
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            
            for i, (crop, conf) in enumerate(crops):
                if crop.size > 0:
                    name = f"{prefix}_" if prefix else ""
                    filename = output_dir / f"{name}face_{timestamp}_{i}_{conf:.2f}.png"
                    cv2.imwrite(str(filename), crop)
        
        def stop_detection(self):
            """Stop detection and release resources."""
            self.running = False
            
            if self.cap:
                self.cap.release()
                self.cap = None
            
            self.start_btn.configure(state=tk.NORMAL)
            self.stop_btn.configure(state=tk.DISABLED)
            self.screenshot_btn.configure(state=tk.DISABLED)
            self.status_label.configure(text="Status: Arrêté")
            self.fps_label.configure(text="FPS: --")
        
        def on_close(self):
            """Handle window close."""
            self.stop_detection()
            self.root.destroy()
    
    # Create and run application
    root = tk.Tk()
    app = FaceDetectionApp(root)
    root.mainloop()


def main():
    """Main entry point with CLI argument parsing."""
    parser = argparse.ArgumentParser(
        description="Face Detection using YOLOv12",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python face_detector.py                    # Run with GUI (default)
  python face_detector.py --cli              # Run with CLI/OpenCV window
  python face_detector.py --camera 1         # Use camera 1
  python face_detector.py --confidence 0.7   # Set confidence to 0.7
        """
    )
    
    parser.add_argument('--cli', action='store_true',
                        help='Run in CLI mode with OpenCV window')
    parser.add_argument('--camera', '-c', type=int, default=0,
                        help='Camera device ID (default: 0)')
    parser.add_argument('--confidence', '-conf', type=float, default=0.5,
                        help='Detection confidence threshold (default: 0.5)')
    parser.add_argument('--model', '-m', type=str, default=None,
                        help='Path to YOLO model')
    
    args = parser.parse_args()
    
    if args.cli:
        run_cli(args.camera, args.confidence, args.model)
    else:
        run_gui(args.camera, args.confidence, args.model)


if __name__ == "__main__":
    main()
