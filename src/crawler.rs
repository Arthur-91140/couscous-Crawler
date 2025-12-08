use crate::cli::Args;
use crate::database::Database;
use crate::extractor::{extract_emails, extract_links, is_same_domain};
use reqwest::Client;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Semaphore};
use url::Url;

/// Task to be processed by crawler workers
#[derive(Debug, Clone)]
struct CrawlTask {
    url: Url,
    depth: u32,
}

/// Crawler state shared between workers
pub struct Crawler {
    client: Client,
    db: Arc<Database>,
    visited: Arc<Mutex<HashSet<String>>>,
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
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Crawler {
            client,
            db,
            visited: Arc::new(Mutex::new(HashSet::new())),
            args,
            base_domain,
        })
    }

    /// Run the crawler
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let start_url = Url::parse(&self.args.url)?;
        
        // Channel for distributing tasks
        let (tx, mut rx) = mpsc::channel::<CrawlTask>(1000);
        
        // Track active tasks for graceful shutdown
        let active_tasks = Arc::new(AtomicUsize::new(1)); // Start with 1 for the initial URL
        let semaphore = Arc::new(Semaphore::new(self.args.workers));

        // Send initial task
        tx.send(CrawlTask {
            url: start_url,
            depth: 1,
        }).await?;

        // Process tasks until all are done
        while active_tasks.load(Ordering::SeqCst) > 0 || !tx.is_closed() {
            // Try to receive with timeout to check termination condition
            match tokio::time::timeout(
                std::time::Duration::from_millis(100),
                rx.recv()
            ).await {
                Ok(Some(task)) => {
                    // Acquire semaphore permit
                    let permit = semaphore.clone().acquire_owned().await.unwrap();
                    
                    let client = self.client.clone();
                    let db = self.db.clone();
                    let visited = self.visited.clone();
                    let args = self.args.clone();
                    let base_domain = self.base_domain.clone();
                    let tx = tx.clone();
                    let active = active_tasks.clone();

                    tokio::spawn(async move {
                        process_task(
                            task, client, db, visited, args, base_domain, tx, active.clone()
                        ).await;
                        active.fetch_sub(1, Ordering::SeqCst);
                        drop(permit);
                    });
                }
                Ok(None) => {
                    // Channel closed
                    break;
                }
                Err(_) => {
                    // Timeout - check if we should exit
                    if active_tasks.load(Ordering::SeqCst) == 0 {
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}

async fn process_task(
    task: CrawlTask,
    client: Client,
    db: Arc<Database>,
    visited: Arc<Mutex<HashSet<String>>>,
    args: Args,
    base_domain: String,
    tx: mpsc::Sender<CrawlTask>,
    active_tasks: Arc<AtomicUsize>,
) {
    let url_str = task.url.to_string();

    // Check if already visited
    {
        let mut visited_guard = visited.lock().await;
        if visited_guard.contains(&url_str) {
            return;
        }
        visited_guard.insert(url_str.clone());
    }

    if args.verbose {
        println!("[Crawling] {} (depth: {})", url_str, task.depth);
    }

    // Fetch the page
    let html = match fetch_page(&client, &task.url).await {
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
        println!("📧 Found {} emails ({} new) on {}", emails.len(), new_emails, url_str);
    }

    // Check depth limit
    let should_follow_links = args.depth == 0 || task.depth < args.depth;
    
    if should_follow_links {
        // Extract and queue new links
        let links = extract_links(&html, &task.url);
        
        for link in links {
            // Check domain constraint
            if args.stay_on_domain && !is_same_domain(&link, &base_domain) {
                continue;
            }

            // Check if already visited
            let link_str = link.to_string();
            {
                let visited_guard = visited.lock().await;
                if visited_guard.contains(&link_str) {
                    continue;
                }
            }

            // Queue the new task
            let new_task = CrawlTask {
                url: link,
                depth: task.depth + 1,
            };

            // Increment active tasks before sending
            active_tasks.fetch_add(1, Ordering::SeqCst);
            
            if tx.send(new_task).await.is_err() {
                // Channel closed, decrement back
                active_tasks.fetch_sub(1, Ordering::SeqCst);
                break;
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
