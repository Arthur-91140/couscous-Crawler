#!/usr/bin/env python3
"""
Face detection script using YOLOv12 for the Couscous Crawler.
Usage: python face_detect.py <model_path> <image_path>
Exit code: 0 if face detected, 1 if no face or error
"""
import sys

def main():
    if len(sys.argv) != 3:
        print("Usage: python face_detect.py <model_path> <image_path>", file=sys.stderr)
        sys.exit(1)

    model_path = sys.argv[1]
    image_path = sys.argv[2]

    try:
        from ultralytics import YOLO
        
        # Load the model
        model = YOLO(model_path)
        
        # Run inference
        results = model(image_path, verbose=False)
        
        # Check if any faces were detected
        if len(results) > 0 and len(results[0].boxes) > 0:
            print(f"Face detected: {len(results[0].boxes)} face(s) found")
            sys.exit(0)
        else:
            print("No face detected")
            sys.exit(1)
            
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
