use crate::cli::Args;
use crate::database::Database;
use crate::extractor::{extract_emails, extract_links, is_same_domain};
use colored::*;
use rand::Rng;
use reqwest::Client;
use std::sync::Arc;
use url::Url;

// Common user agents for stealth
const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
];

/// Crawler state
pub struct Crawler {
    db: Arc<Database>,
    args: Args,
    base_domain: String,
}

impl Crawler {
    /// Create a new crawler instance
    pub fn new(args: Args, db: Arc<Database>) -> Result<Self, Box<dyn std::error::Error>> {
        let start_url = Url::parse(&args.url)?;
        let base_domain = start_url
            .host_str()
            .ok_or("Invalid URL: no host")?
            .to_string();

        Ok(Crawler {
            db,
            args,
            base_domain,
        })
    }

    /// Initialize the crawl (queue start URL or resume)
    pub fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.args.resume {
            // Reset any URLs that were processing when interrupted
            let reset = self.db.reset_processing()?;
            if reset > 0 {
                println!("Resumed {} interrupted URLs", reset);
            }
            let pending = self.db.pending_count()?;
            println!("Pending URLs in queue: {}", pending);
        } else {
            // Clear queue and start fresh
            self.db.clear_queue()?;
            self.db.queue_url(&self.args.url, 1)?;
        }
        Ok(())
    }

    /// Run the crawler
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Spawn workers
        let mut handles = vec![];
        
        for _ in 0..self.args.workers {
            let db = self.db.clone();
            let args = self.args.clone();
            let base_domain = self.base_domain.clone();
            
            handles.push(tokio::spawn(async move {
                worker_loop(db, args, base_domain).await;
            }));
        }

        // Wait for all workers
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }
}

/// Create a stealthy HTTP client with random user agent
fn create_stealth_client(timeout_ms: u64) -> Result<Client, reqwest::Error> {
    let mut rng = rand::thread_rng();
    let user_agent = USER_AGENTS[rng.gen_range(0..USER_AGENTS.len())];
    
    Client::builder()
        .user_agent(user_agent)
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
}

/// Random delay for stealth (50-200ms)
async fn stealth_delay() {
    let delay = {
        let mut rng = rand::thread_rng();
        rng.gen_range(50..200)
    };
    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
}

async fn worker_loop(
    db: Arc<Database>,
    args: Args,
    base_domain: String,
) {
    let mut idle_count = 0;
    
    loop {
        // Try to get a task from the database queue
        let task = db.pop_url().ok().flatten();

        match task {
            Some((url, depth)) => {
                idle_count = 0;
                
                // Stealth delay between requests
                stealth_delay().await;
                
                process_url(&db, &args, &base_domain, &url, depth).await;
                let _ = db.complete_url(&url);
            }
            None => {
                // No task available, wait a bit
                idle_count += 1;
                
                // Exit after being idle for too long (500ms total)
                if idle_count > 10 {
                    break;
                }
                
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }
}

async fn process_url(
    db: &Arc<Database>,
    args: &Args,
    base_domain: &str,
    url: &str,
    depth: u32,
) {
    // Check if already visited
    if db.is_visited(url).unwrap_or(true) {
        return;
    }
    let _ = db.mark_visited(url);

    if args.verbose {
        println!("{}", format!("[Crawling] {} (depth: {})", url, depth).white());
    }

    // Parse URL
    let parsed_url = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => return,
    };

    // Create a new client for each request (with random user agent)
    let client = match create_stealth_client(args.timeout) {
        Ok(c) => c,
        Err(e) => {
            if args.verbose {
                eprintln!("{}", format!("[Error] {}: {}", url, e).red());
            }
            return;
        }
    };

    // Fetch the page
    let html = match fetch_page(&client, &parsed_url).await {
        Ok(content) => content,
        Err(e) => {
            if args.verbose {
                eprintln!("{}", format!("[Error] {}: {}", url, e).red());
            }
            return;
        }
    };

    // Extract emails
    let emails = extract_emails(&html);
    let mut new_emails = 0;
    for email in &emails {
        match db.insert_email(email, url) {
            Ok(true) => new_emails += 1,
            Ok(false) => {}
            Err(e) => {
                if args.verbose {
                    eprintln!("{}", format!("[DB Error] {}", e).red());
                }
            }
        }
    }

    if !emails.is_empty() {
        println!("{}", format!("Found {} emails ({} new) on {}", emails.len(), new_emails, url).green());
    }

    // Check depth limit
    let should_follow_links = args.depth == 0 || depth < args.depth;
    
    if should_follow_links {
        // Extract and queue new links
        let links = extract_links(&html, &parsed_url);
        
        for link in links {
            // Check domain constraint
            if args.stay_on_domain && !is_same_domain(&link, base_domain) {
                continue;
            }

            let link_str = link.to_string();
            
            // Check if already visited before queuing
            if !db.is_visited(&link_str).unwrap_or(true) {
                let _ = db.queue_url(&link_str, depth + 1);
            }
        }
    }
}

async fn fetch_page(client: &Client, url: &Url) -> Result<String, reqwest::Error> {
    let response = client
        .get(url.as_str())
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.5")
        .header("Accept-Encoding", "gzip, deflate, br")
        .header("Connection", "keep-alive")
        .header("Upgrade-Insecure-Requests", "1")
        .send()
        .await?;
    
    // Only process HTML content
    if let Some(content_type) = response.headers().get("content-type") {
        if let Ok(ct) = content_type.to_str() {
            if !ct.contains("text/html") && !ct.contains("text/plain") {
                return Ok(String::new());
            }
        }
    }

    response.text().await
}
