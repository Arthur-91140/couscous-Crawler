use clap::Parser;

/// Couscous Crawler - A fast async web crawler that extracts emails
#[derive(Parser, Debug, Clone)]
#[command(name = "couscous-crawler")]
#[command(author = "Arthur")]
#[command(version = "0.1.0")]
#[command(about = "Crawl websites and extract emails to SQLite", long_about = None)]
pub struct Args {
    /// Starting URL to crawl
    #[arg(required = true)]
    pub url: String,

    /// Maximum crawl depth (0 = unlimited)
    #[arg(short, long, default_value_t = 0)]
    pub depth: u32,

    /// Stay on the same domain only
    #[arg(short, long, default_value_t = false)]
    pub stay_on_domain: bool,

    /// Number of async workers
    #[arg(short, long, default_value_t = 10)]
    pub workers: usize,

    /// SQLite database path
    #[arg(long, default_value = "emails.db")]
    pub db: String,

    /// HTTP request timeout in milliseconds
    #[arg(short = 't', long, default_value_t = 30000)]
    pub timeout: u64,

    /// Resume from existing database
    #[arg(short, long, default_value_t = false)]
    pub resume: bool,

    /// Verbose output
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// Disable SSL certificate verification
    #[arg(short = 'k', long, default_value_t = false)]
    pub insecure: bool,

    /// Enable image face detection
    #[arg(long, default_value_t = false)]
    pub extract_images: bool,

    /// Path to YOLO face detection model (.pt file)
    #[arg(long, default_value = "face-detection/yolov12l-face.pt")]
    pub yolo_model: String,

    /// Minimum image width for face detection
    #[arg(long, default_value_t = 128)]
    pub min_image_width: u32,

    /// Minimum image height for face detection
    #[arg(long, default_value_t = 128)]
    pub min_image_height: u32,

    /// Output directory for images with faces
    #[arg(long, default_value = "faces")]
    pub faces_dir: String,
}

pub fn parse_args() -> Args {
    Args::parse()
}
