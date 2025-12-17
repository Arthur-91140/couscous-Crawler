mod cli;
mod crawler;
mod database;
mod extractor;
mod image_processor;

use cli::parse_args;
use crawler::Crawler;
use database::Database;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args();
    
    println!("Couscous Crawler v0.1.0");
    println!("==========================");
    println!("Starting URL: {}", args.url);
    println!("Depth limit: {}", if args.depth == 0 { "unlimited".to_string() } else { args.depth.to_string() });
    println!("Stay on domain: {}", args.stay_on_domain);
    println!("Workers: {}", args.workers);
    println!("Database: {}", args.db);
    println!("Resume: {}", args.resume);
    println!();

    // Initialize database
    let db = Arc::new(Database::new(&args.db)?);
    
    // Create crawler
    let crawler = Crawler::new(args.clone(), db.clone())?;
    
    // Initialize (queue start URL or resume)
    crawler.init()?;
    
    let start_time = Instant::now();
    
    println!("Starting crawl...\n");
    crawler.run().await?;
    
    let elapsed = start_time.elapsed();
    
    // Print statistics
    let (unique_emails, total_entries) = db.get_stats()?;
    let unique_phones = db.get_phone_count()?;
    let images_saved = db.get_image_count()?;
    
    println!();
    println!("==========================");
    println!("Crawl complete!");
    println!("Time elapsed: {:.2}s", elapsed.as_secs_f64());
    println!("Unique emails found: {}", unique_emails);
    println!("Unique phones found: {}", unique_phones);
    println!("Images with faces: {}", images_saved);
    println!("Total email entries: {}", total_entries);
    println!("Results saved to: {}", args.db);
    
    Ok(())
}
