use crate::database::Database;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use url::Url;
use uuid::Uuid;

/// Image processor for downloading and detecting faces
pub struct ImageProcessor {
    output_dir: PathBuf,
    model_path: PathBuf,
    min_width: u32,
    min_height: u32,
}

impl ImageProcessor {
    /// Create a new image processor
    pub fn new(output_dir: &str, model_path: &str, min_width: u32, min_height: u32) -> Self {
        ImageProcessor {
            output_dir: PathBuf::from(output_dir),
            model_path: PathBuf::from(model_path),
            min_width,
            min_height,
        }
    }

    /// Extract image URLs from HTML content
    pub fn extract_image_urls(html: &str, base_url: &Url) -> Vec<Url> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("img[src]").unwrap();
        let mut images: HashSet<Url> = HashSet::new();

        for element in document.select(&selector) {
            if let Some(src) = element.value().attr("src") {
                // Skip data URIs and empty sources
                if src.starts_with("data:") || src.is_empty() {
                    continue;
                }

                // Try to resolve the URL
                if let Ok(resolved) = base_url.join(src) {
                    // Only keep http/https links
                    if resolved.scheme() == "http" || resolved.scheme() == "https" {
                        // Check if it looks like an image URL
                        let path = resolved.path().to_lowercase();
                        if path.ends_with(".jpg")
                            || path.ends_with(".jpeg")
                            || path.ends_with(".png")
                            || path.ends_with(".webp")
                            || path.ends_with(".gif")
                            || path.contains("/image")
                            || path.contains("/photo")
                        {
                            images.insert(resolved);
                        }
                    }
                }
            }
        }

        images.into_iter().collect()
    }

    /// Download an image with progress bar and return the local path
    async fn download_image(&self, client: &Client, url: &Url) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let response = client
            .get(url.as_str())
            .send()
            .await?;

        // Check if it's an image
        if let Some(content_type) = response.headers().get("content-type") {
            if let Ok(ct) = content_type.to_str() {
                if !ct.contains("image/") {
                    return Err("Not an image".into());
                }
            }
        }

        // Get content length for progress bar
        let total_size = response.content_length().unwrap_or(0);

        // Create temp directory if needed
        let temp_dir = self.output_dir.join("temp");
        fs::create_dir_all(&temp_dir).await?;

        // Generate UUID for filename
        let uuid = Uuid::new_v4().to_string();
        let extension = url.path()
            .rsplit('.')
            .next()
            .unwrap_or("jpg")
            .to_lowercase();
        let extension = if ["jpg", "jpeg", "png", "webp", "gif"].contains(&extension.as_str()) {
            extension
        } else {
            "jpg".to_string()
        };
        let filename = format!("{}.{}", uuid, extension);
        let file_path = temp_dir.join(&filename);

        // Create file
        let mut file = fs::File::create(&file_path).await?;

        // Download with progress bar
        if total_size > 0 {
            let pb = ProgressBar::new(total_size);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"));

            let mut downloaded: u64 = 0;
            let mut stream = response.bytes().await?;
            
            file.write_all(&stream).await?;
            downloaded += stream.len() as u64;
            pb.set_position(downloaded);
            pb.finish_with_message("Downloaded");
        } else {
            // No content length, just download
            let bytes = response.bytes().await?;
            file.write_all(&bytes).await?;
        }

        file.flush().await?;
        drop(file);

        Ok(file_path)
    }

    /// Check if image is large enough by reading its header
    async fn check_image_size(&self, path: &Path) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let bytes = fs::read(path).await?;
        
        // Try to get image dimensions from header
        if let Some((width, height)) = Self::get_image_dimensions(&bytes) {
            Ok(width >= self.min_width && height >= self.min_height)
        } else {
            // If we can't determine size, assume it's valid
            Ok(true)
        }
    }

    /// Get image dimensions from bytes (supports PNG, JPEG, GIF, WEBP)
    fn get_image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
        if bytes.len() < 24 {
            return None;
        }

        // PNG: bytes 16-23 contain width and height as 4-byte big-endian
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
            let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
            return Some((width, height));
        }

        // JPEG: need to parse segments
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            let mut i = 2;
            while i + 9 < bytes.len() {
                if bytes[i] != 0xFF {
                    i += 1;
                    continue;
                }
                let marker = bytes[i + 1];
                // SOF0, SOF1, SOF2 markers
                if marker >= 0xC0 && marker <= 0xC3 {
                    let height = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                    let width = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                    return Some((width, height));
                }
                // Skip to next segment
                if i + 3 < bytes.len() {
                    let length = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
                    i += 2 + length;
                } else {
                    break;
                }
            }
        }

        // GIF: bytes 6-9 contain width and height as 2-byte little-endian
        if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            let width = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
            let height = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
            return Some((width, height));
        }

        // WEBP: need to parse RIFF header
        if bytes.starts_with(b"RIFF") && bytes.len() > 30 && &bytes[8..12] == b"WEBP" {
            // VP8 format
            if &bytes[12..16] == b"VP8 " {
                // Simple VP8
                if bytes.len() > 26 {
                    let width = (u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3FFF) as u32;
                    let height = (u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3FFF) as u32;
                    return Some((width, height));
                }
            }
        }

        None
    }

    /// Detect faces using YOLOv12 model via Python script
    fn detect_face(&self, image_path: &Path, verbose: bool) -> bool {
        // Get the directory where the model is located for the script
        let script_dir = self.model_path.parent().unwrap_or(Path::new("."));
        let script_path = script_dir.join("face_detect.py");

        if verbose {
            println!("{}", format!("[YOLO] Running face detection on {:?}...", image_path.file_name().unwrap()).blue());
            println!("{}", format!("[YOLO] Using 'py -3' with script: {:?}", script_path).blue());
        }

        // Run Python script - use 'py -3' to ensure Python 3.x on Windows
        let output = Command::new("py")
            .arg("-3")
            .arg(&script_path)
            .arg(&self.model_path)
            .arg(image_path)
            .output();

        match output {
            Ok(result) => {
                if verbose {
                    if let Ok(stdout) = String::from_utf8(result.stdout.clone()) {
                        if !stdout.is_empty() {
                            println!("{}", format!("[YOLO] {}", stdout.trim()).blue());
                        }
                    }
                    if !result.status.success() {
                        if let Ok(stderr) = String::from_utf8(result.stderr) {
                            if !stderr.is_empty() {
                                eprintln!("{}", format!("[YOLO Error] {}", stderr.trim()).red());
                            }
                        }
                    }
                }
                result.status.success()
            },
            Err(e) => {
                if verbose {
                    eprintln!("{}", format!("[YOLO Error] Failed to run python3: {}", e).red());
                }
                false
            },
        }
    }

    /// Process an image: download, check size, detect face, save or delete
    pub async fn process_image(
        &self,
        client: &Client,
        url: &Url,
        db: &Arc<Database>,
        verbose: bool,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Download the image
        let temp_path = match self.download_image(client, url).await {
            Ok(path) => path,
            Err(e) => {
                if verbose {
                    eprintln!("{}", format!("[Image Error] {}: {}", url, e).red());
                }
                return Ok(false);
            }
        };

        // Check image size
        let is_large_enough = self.check_image_size(&temp_path).await.unwrap_or(false);
        if !is_large_enough {
            // Delete too small image
            let _ = fs::remove_file(&temp_path).await;
            if verbose {
                println!("{}", format!("[Image] Too small, skipping: {}", url).yellow());
            }
            return Ok(false);
        }

        // Detect face
        let has_face = self.detect_face(&temp_path, verbose);

        if has_face {
            // Extract UUID from filename
            let uuid = temp_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Create UUID directory
            let uuid_dir = self.output_dir.join(&uuid);
            fs::create_dir_all(&uuid_dir).await?;

            // Move file to UUID directory
            let final_path = uuid_dir.join(temp_path.file_name().unwrap());
            fs::rename(&temp_path, &final_path).await?;

            // Insert into database
            let _ = db.insert_image(&uuid, url.as_str());

            println!("{}", format!("[Face Found] Saved {} from {}", uuid, url).green());
            Ok(true)
        } else {
            // Delete image without face
            let _ = fs::remove_file(&temp_path).await;
            if verbose {
                println!("{}", format!("[Image] No face detected: {}", url).white());
            }
            Ok(false)
        }
    }
}
