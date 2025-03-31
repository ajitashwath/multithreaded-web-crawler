use clap::Parser;
use log::{info, LevelFilter};

use multithreaded_web_crawler::config::CrawlerConfig;
use multithreaded_web_crawler::crawler::Crawler;
use multithreaded_web_crawler::storage::memory::MemoryStore;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]

struct Args {
    #[arg(required = true)]
    seed_urls: Vec<String>,
    #[args(short, long, default_value_t = 2)]
    depth: usize,
    #[args(short, long, default_value_t = 8)]
    depth: usize,
    #[args(short, long, default_value_t = 200)]
    depth: u64,
    #[args(long, default_value_t = true)] 
    respect_robots: bool,
}

#[tokio::main] 
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder().filter_level(LevelFilter::Info).init();
    let args = Args::parse();
    let config = CrawlerConfig {
        max_depth: args.depth,
        max_pages: args.max_pages,
        concurrent_requests: args.concurrent,
        delay_ms: args.delay,
        user_agent: "RustWebCrawler/1.0 (https://example.com/bot)".to_string(),
        respect_robots_txt: args.respect_robots,
    };

    let store = MemoryStore::new();
    let crawler = Crawler::new(config, Box::new(store)).await?;
    let result = crawler.crawl(args.seed_urls).await?;

    info!("Crawling completed. Found {} pages.", result.paper_processed);
    info!("Crawled {} unique URLs.", result.unique_urls);

    Ok(())
}