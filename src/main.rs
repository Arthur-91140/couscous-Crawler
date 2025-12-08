mod cli;
mod crawler;
mod database;
mod extractor;

use cli::parse_args;
use crawler::Crawler;
use database::Database;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args();
    
    println!("🥘 Couscous Crawler v0.1.0");
    println!("==========================");
    println!("Starting URL: {}", args.url);
    println!("Depth limit: {}", if args.depth == 0 { "unlimited".to_string() } else { args.depth.to_string() });
    println!("Stay on domain: {}", args.stay_on_domain);
    println!("Workers: {}", args.workers);
    println!("Database: {}", args.db);
    println!();

    // Initialize database
    let db = Arc::new(Database::new(&args.db)?);
    
    // Create and run crawler
    let crawler = Crawler::new(args.clone(), db.clone())?;
    
    let start_time = Instant::now();
    
    println!("🚀 Starting crawl...\n");
    crawler.run().await?;
    
    let elapsed = start_time.elapsed();
    
    // Print statistics
    let (unique_emails, total_entries) = db.get_stats()?;
    
    println!();
    println!("==========================");
    println!("✅ Crawl complete!");
    println!("⏱️  Time elapsed: {:.2}s", elapsed.as_secs_f64());
    println!("📧 Unique emails found: {}", unique_emails);
    println!("📝 Total entries: {}", total_entries);
    println!("💾 Results saved to: {}", args.db);
    
    Ok(())
}
