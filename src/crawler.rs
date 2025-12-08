use crate::cli::Args;
use crate::database::Database;
use crate::extractor::{extract_emails, extract_links, is_same_domain};
use reqwest::Client;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
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
        let (tx, rx) = mpsc::channel::<CrawlTask>(1000);
        let rx = Arc::new(Mutex::new(rx));

        // Send initial task
        tx.send(CrawlTask {
            url: start_url,
            depth: 1,
        }).await?;

        // Spawn workers
        let mut handles = vec![];
        for worker_id in 0..self.args.workers {
            let worker = CrawlerWorker {
                id: worker_id,
                client: self.client.clone(),
                db: self.db.clone(),
                visited: self.visited.clone(),
                args: self.args.clone(),
                base_domain: self.base_domain.clone(),
                tx: tx.clone(),
                rx: rx.clone(),
            };
            
            handles.push(tokio::spawn(async move {
                worker.run().await
            }));
        }

        // Drop the original sender so the channel closes when all workers are done
        drop(tx);

        // Wait for all workers to complete
        for handle in handles {
            if let Err(e) = handle.await {
                if self.args.verbose {
                    eprintln!("[ERROR] Worker panicked: {:?}", e);
                }
            }
        }

        Ok(())
    }
}

/// Individual crawler worker
struct CrawlerWorker {
    id: usize,
    client: Client,
    db: Arc<Database>,
    visited: Arc<Mutex<HashSet<String>>>,
    args: Args,
    base_domain: String,
    tx: mpsc::Sender<CrawlTask>,
    rx: Arc<Mutex<mpsc::Receiver<CrawlTask>>>,
}

impl CrawlerWorker {
    async fn run(&self) {
        loop {
            // Try to get a task
            let task = {
                let mut rx = self.rx.lock().await;
                rx.recv().await
            };

            match task {
                Some(task) => {
                    self.process_task(task).await;
                }
                None => {
                    // Channel closed, exit
                    break;
                }
            }
        }
    }

    async fn process_task(&self, task: CrawlTask) {
        let url_str = task.url.to_string();

        // Check if already visited
        {
            let mut visited = self.visited.lock().await;
            if visited.contains(&url_str) {
                return;
            }
            visited.insert(url_str.clone());
        }

        if self.args.verbose {
            println!("[Worker {}] Crawling: {} (depth: {})", self.id, url_str, task.depth);
        }

        // Fetch the page
        let html = match self.fetch_page(&task.url).await {
            Ok(content) => content,
            Err(e) => {
                if self.args.verbose {
                    eprintln!("[Worker {}] Error fetching {}: {}", self.id, url_str, e);
                }
                return;
            }
        };

        // Extract emails
        let emails = extract_emails(&html);
        let mut new_emails = 0;
        for email in &emails {
            match self.db.insert_email(email, &url_str) {
                Ok(true) => new_emails += 1,
                Ok(false) => {}
                Err(e) => {
                    if self.args.verbose {
                        eprintln!("[Worker {}] DB error: {}", self.id, e);
                    }
                }
            }
        }

        if !emails.is_empty() && self.args.verbose {
            println!("[Worker {}] Found {} emails ({} new) on {}", 
                self.id, emails.len(), new_emails, url_str);
        }

        // Check depth limit
        let should_follow_links = self.args.depth == 0 || task.depth < self.args.depth;
        
        if should_follow_links {
            // Extract and queue new links
            let links = extract_links(&html, &task.url);
            
            for link in links {
                // Check domain constraint
                if self.args.stay_on_domain && !is_same_domain(&link, &self.base_domain) {
                    continue;
                }

                // Check if already visited
                let link_str = link.to_string();
                {
                    let visited = self.visited.lock().await;
                    if visited.contains(&link_str) {
                        continue;
                    }
                }

                // Queue the new task
                let new_task = CrawlTask {
                    url: link,
                    depth: task.depth + 1,
                };

                if self.tx.send(new_task).await.is_err() {
                    // Channel closed
                    break;
                }
            }
        }
    }

    async fn fetch_page(&self, url: &Url) -> Result<String, reqwest::Error> {
        let response = self.client.get(url.as_str()).send().await?;
        
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
}
