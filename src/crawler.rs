use crate::cli::Args;
use crate::database::Database;
use crate::extractor::{extract_emails, extract_links, is_same_domain};
use reqwest::Client;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
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
            .user_agent("CouscousCrawler/0.1 (Educational Web Crawler)")
            .timeout(std::time::Duration::from_millis(args.timeout))
            .build()?;

        Ok(Crawler {
            client,
            db,
            args,
            base_domain,
        })
    }

    /// Run the crawler
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let start_url = Url::parse(&self.args.url)?;
        
        // Shared state
        let visited: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
        let queue: Arc<Mutex<VecDeque<(Url, u32)>>> = Arc::new(Mutex::new(VecDeque::new()));
        
        // Add initial URL
        {
            let mut q = queue.lock().await;
            q.push_back((start_url, 1));
        }

        // Process queue with worker pool
        let mut handles = vec![];
        
        for _ in 0..self.args.workers {
            let client = self.client.clone();
            let db = self.db.clone();
            let visited = visited.clone();
            let queue = queue.clone();
            let args = self.args.clone();
            let base_domain = self.base_domain.clone();
            
            handles.push(tokio::spawn(async move {
                worker_loop(client, db, visited, queue, args, base_domain).await;
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
    visited: Arc<Mutex<HashSet<String>>>,
    queue: Arc<Mutex<VecDeque<(Url, u32)>>>,
    args: Args,
    base_domain: String,
) {
    let mut idle_count = 0;
    
    loop {
        // Try to get a task from the queue
        let task = {
            let mut q = queue.lock().await;
            q.pop_front()
        };

        match task {
            Some((url, depth)) => {
                idle_count = 0;
                process_url(&client, &db, &visited, &queue, &args, &base_domain, url, depth).await;
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
    visited: &Arc<Mutex<HashSet<String>>>,
    queue: &Arc<Mutex<VecDeque<(Url, u32)>>>,
    args: &Args,
    base_domain: &str,
    url: Url,
    depth: u32,
) {
    let url_str = url.to_string();

    // Check if already visited
    {
        let mut v = visited.lock().await;
        if v.contains(&url_str) {
            return;
        }
        v.insert(url_str.clone());
    }

    if args.verbose {
        println!("[Crawling] {} (depth: {})", url_str, depth);
    }

    // Fetch the page
    let html = match fetch_page(client, &url).await {
        Ok(content) => content,
        Err(e) => {
            if args.verbose {
                eprintln!("[Error] {}: {}", url_str, e);
            }
            return;
        }
    };

    // Extract emails
    let emails = extract_emails(&html);
    let mut new_emails = 0;
    for email in &emails {
        match db.insert_email(email, &url_str) {
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
        println!("Found {} emails ({} new) on {}", emails.len(), new_emails, url_str);
    }

    // Check depth limit
    let should_follow_links = args.depth == 0 || depth < args.depth;
    
    if should_follow_links {
        // Extract and queue new links
        let links = extract_links(&html, &url);
        let mut new_links = vec![];
        
        {
            let v = visited.lock().await;
            for link in links {
                // Check domain constraint
                if args.stay_on_domain && !is_same_domain(&link, base_domain) {
                    continue;
                }

                let link_str = link.to_string();
                if !v.contains(&link_str) {
                    new_links.push((link, depth + 1));
                }
            }
        }

        // Add new links to queue
        if !new_links.is_empty() {
            let mut q = queue.lock().await;
            for task in new_links {
                q.push_back(task);
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
