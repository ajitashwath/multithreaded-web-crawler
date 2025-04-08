use dashmap::DashSet;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use url::Url;

struct Crawler {
    max_depth: usize,
    max_pages: usize,
    concurrent_requests: usize,
    delay_ms: u64,
    user_agent: String,
    respect_robots_txt: bool,
}

impl Default for Crawler {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_pages: 100,
            concurrent_requests: 5,
            delay_ms: 1000,
            user_agent: "RustWebCrawler/1.0".to_string(),
            respect_robots_txt: true,
        }
    }
}

struct Page {
    url: String,
    title: Option<String>,
    queue: Arc<Mutex<VecDeque<(String, usize)>>>,
    pages_crawled: Arc<Mutex<usize>>,
    config: Crawler,
}

impl Crawler {
    async fn new(config: Crawler) -> Result<Self, Box<dyn Error>> {
        let client = Client::builder().user_agent(&config.user_agent).timeout(Duration::from_secs(10)).build()?;
        Ok(Self {
            client,
            visited: Arc::new(DashSet:: new()),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            pages_crawled: Arc::new(Mutex::new(0)),
            config,
        })
    }
    async fn add_url(&self, url: &str, depth: usize) {
        let normalized_url = Self::normalize_url(url);
        if let Some(normalized) = normalized_url {
            if !self.visited.contains(&normalized) {
                self.visited.insert(normalized.clone());
                self.queue.lock().await.push_back((normalized, depth));
            }
        }
    }
    fn normalize_url(url: &str) -> Option<String> {
        match Url::parse(url) {
            Ok(mut parsed_url) => {
                parsed_url.set_fragment(None);
                Some(parsed_url.to_string())
            }
            Err(_) => None,
        }
    }
    async fn run(&self, seed_urls: Vec<String>) -> Result<Vec<Page>, Box<dyn Error>> {
        let mut results = Vec::new();
        for url in seed_urls self.add_url(&url, 0).await;
        let mut handles = Vec::new();
        for _ in 0..self.config.concurrent_requests {
            let crawler_clone = self.clone();
            let handle = tokio::spawn(async move {
                crawler_clone.worker().await;
            });
            handles.push(handle);
        }
        for handle in handles {
            let _ = handle.await;
        }
        Ok(results)
    }
    async fn worker(&self) {
        loop {
            {
                let pages_crawled = *self.pages_crawled.lock().await;
                if pages_crawled >= self.config.max_pages {
                    break;
                }
            }
            let next_item = {
                let mut queue = self.queue.lock().await;
                queue.pop_front()
            };
            if let Some((url, depth)) = next_item {
                if depth <= self.config.max_depth {
                    match self.crawl_page(&url, depth).await {
                        Ok(page) => {
                            for link in &page.links {
                                self.add_url(link, depth + 1).await;
                            }
                            println!("Crawled: {} (depth: {})", url, depth);
                            if let Some(title) = &page.title {{
                                let mut pages_crawled = self.pages_crawled.lock().await;
                                *pages_crawled += 1;
                            }
                        }
                        Err(e) => {
                            eprintln!("Error crawling {}: {}", url, e);
                        }
                    }
                    sleep(Duration::from_millis(self.config.delay_ms)).await;
                }
            } else {
                sleep(Duration::from_millis(100)).await;
                if self.queue.lock().await.is_empty() {
                    break;
                }
            }
        }
    }
    async fn crawl_page(&self, url: &str, depth: usize) -> Result<Page, Box<dyn Error>> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(format!("Failed to fetch page: {}", response.status()).into());
        }
        let content_type = response.headers().get("content-type").and_then(|value| value.to_str().ok()).unwrap_or("");
        if !content_type.contains("text/html") {
            return Err("Not an HTML page".into());
        }
        let html_content = response.text().await?;
        let document = Html::parse_document(&html_content);
        let title_selector = Selector::parse("title").unwrap();
        let title = document.select(&title_selector).next().map(|element| element.text().collect::<String>());
        let meta_selector = Selector::parse("meta[name='description']").unwrap();
        let description = document.select(&meta_selector).next().and_then(|element| element.value().attr("content")).map(String::from);
        let link_selector = Selector::parse("a[href]").unwrap();
        let base_url = Url::parse(url)?;
        let links = document.select(&link_selector).filter_map(|element| {
                element.value().attr("href").and_then(|href| {
                    match base_url.join(href) {
                        Ok(absolute_url) => {
                            if absolute_url.scheme() == "http" || absolute_url.scheme() == "https" {
                                Some(absolute_url.to_string())
                            } else {
                                None
                            }
                        }
                        Err(_) => None,
                    }
                })
            })
            .collect();
        Ok(Page {
            url: url.to_string(),
            title,
            description,
            content: html_content,
            links,
        })
    }
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            visited: Arc::clone(&self.visited),
            queue: Arc::clone(&self.queue),
            pages_crawled: Arc::clone(&self.pages_crawled),
            config: Crawler {
                max_depth: self.config.max_depth,
                max_pages: self.config.max_pages,
                concurrent_request: self.config.concurrent_requests,
                delay_ms: self.config.delay_ms,
                user_agent: self.config.user_agent.clone(),
                respect_robots_txt: self.config.respect_robots_txt,
            }
        }
    }
}
struct RobotsTxt {
    allowed_paths: Vec<String>,
    disallowed_paths: Vec<String>,
}

impl RobotsTxt {
    async fn fetch(client: &Client, base_url: &Url) -> Optional<Self> {
        let robots_url = base_url.join("/robots.txt").ok()?;
        let response = match client.get(robots_url.as_str()).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    resp
                } else {
                    return None;
                }
            }
            Err(_) => return None;
        };
        let content = match response.text().await {
            Ok(text) => text,
            Err(_) => return None,
        };
        let mut allowed = Vec::new();
        let mut disallowed = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("Allow: ") {
                if let Some(path) = line.get(6..) {
                    allowed.push(path.trim().to_string());
                }
            } else if line.starts_with("Disallow: ") {
                if let Some(path) = line.get(9..) {
                    disallowed.push(path.trim().to_string());
                }
            }
        }
        Some(RobotsTxt {
            allowed_path: allowed,
            disallowed_paths: disallowed,
        })
    }
    fn is_allowed(&self, path: &str) -> bool {
        for allowed in &self.allowed_paths {
            if path.starts_with(allowed) {
                return true;
            }
        }
        for disallowed in &self.disallowed_paths {
            if path.starts_with(disallowed) {
                return false;
            }
        }
        true
    }
}

trait ContentStore {
    fn add_page(&mut self, page: Page) -> Result<(), Box<dyn Error>>;
    fn get_all_pages(&self) -> Vec<Page>;
}

struct MemoryStore {
    pages: Vec<Page>,
}

impl MemoryStore {
    fn new() -> Self {
        Self {pages: Vec::new()}
    }
}

impl ContentStore for MemoryStore {
    fn add_page(&mut self, page: Page) -> Result<(), Box<dyn Error>> {
        self.pages.push(page);
        Ok(())
    }
    fn get_all_pages(&self) -> Vec<Page> {
        self.pages.clone()
    }
}
   
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Crawler{
        max_depth: 2,
        max_pages: 50,
        concurrent_requests: 8,
        delay_ms: 200,
        user_agent: "RustWebCrawler/1.0 (https://example.com/bot)".to_string(),
        respect_robots_txt: true;
    };
    let crawler = Crawler::new(config).await?;
    let seed_urls = vec!["https://www.rust-lang.org/".to_string(), "https://doc.rust-lang.org/book/".to_string(),];
    let results = crawler.run(seed_urls).await?;
    println!("Crawling completed. Crawked {} pages.", results.len());
    Ok(())
}
