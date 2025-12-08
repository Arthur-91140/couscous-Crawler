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

    /// Verbose output
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
}

pub fn parse_args() -> Args {
    Args::parse()
}
