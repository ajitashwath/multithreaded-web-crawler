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