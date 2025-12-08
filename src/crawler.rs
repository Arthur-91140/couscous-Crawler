use crate::cli::Args;
use crate::database::Database;
use crate::extractor::{extract_emails, extract_links, is_same_domain};
use reqwest::Client;
use std::sync::Arc;
use url::Url;

/// Crawler state
pub struct Crawler {
    client: Client,
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

        let client = Client::builder()
            .user_agent("CouscousCrawler/0.1")
            .timeout(std::time::Duration::from_millis(args.timeout))
            .build()?;

        Ok(Crawler {
            client,
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
            let client = self.client.clone();
            let db = self.db.clone();
            let args = self.args.clone();
            let base_domain = self.base_domain.clone();
            
            handles.push(tokio::spawn(async move {
                worker_loop(client, db, args, base_domain).await;
            }));
        }

        // Wait for all workers
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }
}

async fn worker_loop(
    client: Client,
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
                process_url(&client, &db, &args, &base_domain, &url, depth).await;
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
    client: &Client,
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
        println!("[Crawling] {} (depth: {})", url, depth);
    }

    // Parse URL
    let parsed_url = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => return,
    };

    // Fetch the page
    let html = match fetch_page(client, &parsed_url).await {
        Ok(content) => content,
        Err(e) => {
            if args.verbose {
                eprintln!("[Error] {}: {}", url, e);
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
                    eprintln!("[DB Error] {}", e);
                }
            }
        }
    }

    if !emails.is_empty() {
        println!("Found {} emails ({} new) on {}", emails.len(), new_emails, url);
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
    let response = client.get(url.as_str()).send().await?;
    
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
