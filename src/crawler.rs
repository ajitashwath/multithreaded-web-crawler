use crate::config::{CrawlResult, CrawlerConfig};
use crate::storage::Store,
use crate::robots::RobotsTxt,
use crate::storage::ContentStore;
use crate::utils::url::normalize_url;

use dashmap::DashSet;
use log::{debug, error, info, warn};
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use url::Url;

pub struct Crawler {
    client: Client,
    visited: Arc<DashSet<String>>,
    queue: Arc<Mutex<VecDeque<(String, usize)>>>,
    pages_crawled: Arc<Mutex<usize>>,
    config: CrawlerConfig,
    store: Arc<Mutex<Box<dyn ContentStore + Send>>>,
    robots_cache: Arc<dashmap::DashMap<String, RobotsTxt>>,
}

impl Crawler {
    pub async fn new(
        config: CrawlerConfig,
        store: Box<dyn ContentStore + Send>,
    ) -> Result<Self, Box<dyn Error>> {
        let client = Client::builder().user_agent(&config.user_agent).timeout(Duration::from_secs(10)).build()?;
        Ok(Self {
            client,
            visited: Arc::new(DashSet::new()),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            pages_crawled: Arc::new(Mutex::new(0)),
            config,
            store: Arc::new(Mutex::new(store)),
            robots_cache: Arc::new(dashmap::DashMap::new()),
        })
    }

    async fn add_url(&self, url: &str, depth: usize) {
        if let Some(normalized_url) = normalize_url(url) {
            if !self.visited.contains(&normalized_url) {
                self.visited.insert(normalized_url.clone());
                self.queue.lock().await.push_back((normalized_url, depth));
            }
        }
    }

    pub async fn run(&self, seed_urls: Vec<String>) -> Result<CrawlResult, Box<dyn Error>> {
        let start_time = Instant::now();
        for url in seed_urls self.add_url(&url, 0).await;
        let mut handles = Vec::new();
        for worker_id in 0..self.config.concurrent_requests {
            let crawler_clone = self.clone();
            let handle = tokio::spawn(async move {
                crawler_clone.worker(worker_id).await;
            });
            handles.push(handle);
        }
        for handle in handles let _ = handle.await;
        let pages_processed = *self.pages_crawled.lock().await;
        let unique_urls = self.visited.len();
        let elapsed = start_time.elapsed().as_secs_f64();

        Ok(CrawlResult {
            pages_processed,
            unique_urls,
            errors: 0,
            elapsed_seconds: elapsed,
        })
    }
    async fn worker(&self, worker_id: usize) {
        debug!("Worker {} starting", worker_id);   
        loop {
            {
                let pages_crawled = *self.pages_crawled.lock().await;
                if pages_crawled >= self.config.max_pages {
                    debug!("Worker {} stopping: max pages reached", worker_id);
                    break;
                }
            }
            let next_item = {
                let mut queue = self.queue.lock().await;
                queue.pop_front()
            };
            if let Some((url, depth)) = next_item {
                if depth <= self.config.max_depth {
                    if self.config.respect_robots_txt {
                        let can_crawl = self.check_robots_txt(&url).await;
                        if !can_crawl {
                            debug!("Skipping {} due to robots.txt", url);
                            continue;
                        }
                    }
                    match self.crawl_page(&url, depth).await {
                        Ok(page) => {
                            if let Err(e) = self.store.lock().await.add_page(page.clone()) error!("Failed to store page {}: {}", url, e);
                            for link in &page.links {
                                self.add_url(link, depth + 1).await;
                            }
                            info!("Worker {}: Crawled {} (depth: {})", worker_id, url, depth);
                            {
                                let mut pages_crawled = self.pages_crawled.lock().await;
                                *pages_crawled += 1;
                            }
                        }
                        Err(e) => error!("Worker {}: Error crawling {}: {}", worker_id, url, e),
                    }
                    sleep(Duration::from_millis(self.config.delay_ms)).await;
                }
            } else {
                sleep(Duration::from_millis(100)).await;
                if self.queue.lock().await.is_empty() {
                    debug!("Worker {} stopping: queue empty", worker_id);
                    break;
                }
            }
        }
    }

    async fn check_robots_txt(&self, url: &str) -> bool {
        let parsed_url = match Url::parse(url) {
            Ok(url) => url,
            Err(_) => return true,
        };
        let domain = match parsed_url.host_str() {
            Some(host) => host.to_string(),
            None => return true,
        };
        if let Some(robots) = self.robots_cache.get(&domain) {
            let path = parsed_url.path();
            return robots.is_allowed(path);
        }
        let robots_url = match parsed_url.join("/robots.txt") {
            Ok(url) => url,
            Err(_) => return true,
        };
        match RobotsTxt::fetch(&self.client, &robots_url).await {
            Some(robots) => {
                let path = parsed_url.path();
                let allowed = robots.is_allowed(path);
                self.robots_cache.insert(domain, robots);
                allowed
            }
            None => true,
        }
    }
    async fn crawl_page(&self, url: &str, depth: usize) -> Result<Page, Box<dyn Error>> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() return Err(format!("Failed to fetch page: {}", response.status()).into());
        let content_type = response.headers().get("content-type").and_then(|value| value.to_str().ok()).unwrap_or("");
        if !content_type.contains("text/html") return Err("Not an HTML page".into());
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
                            // Only keep http and https schemes
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
            crawl_depth: depth,
            crawl_time: chrono::Utc::now(),
        })
    }
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            visited: Arc::clone(&self.visited),
            queue: Arc::clone(&self.queue),
            pages_crawled: Arc::clone(&self.pages_crawled),
            store: Arc::clone(&self.store),
            robots_cache: Arc::clone(&self.robots_cache),
            config: self.config.clone(),
        }
    }
}