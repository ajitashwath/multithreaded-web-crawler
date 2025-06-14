# Web Crawler

A concurrent web crawler written in Rust that can efficiently crawl websites while respecting robots.txt and implementing proper rate limiting.

## Features

- **Concurrent Crawling**: Configurable number of simultaneous requests for optimal performance
- **Depth Control**: Limit crawling depth to prevent infinite loops
- **Rate Limiting**: Built-in delays between requests to be respectful to target servers
- **Robots.txt Support**: Respects website crawling policies (configurable)
- **URL Normalization**: Automatic URL cleaning and deduplication
- **HTML Parsing**: Extracts titles, descriptions, and links from web pages
- **Error Handling**: Robust error handling for network issues and parsing errors
- **Memory Efficient**: Uses efficient data structures for large-scale crawling

## Installation

### Prerequisites

- Rust 1.70+ (2021 edition)
- Cargo package manager

### Clone and Build

```bash
git clone <repository-url>
cd web-crawler
cargo build --release
```

## Configuration

The crawler can be configured with the following parameters:

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `max_depth` | usize | 3 | Maximum crawling depth from seed URLs |
| `max_pages` | usize | 100 | Maximum number of pages to crawl |
| `concurrent_requests` | usize | 5 | Number of simultaneous HTTP requests |
| `delay_ms` | u64 | 1000 | Delay between requests in milliseconds |
| `user_agent` | String | "RustWebCrawler/1.0" | User agent string for HTTP requests |
| `respect_robots_txt` | bool | true | Whether to respect robots.txt files |

## Usage

### Basic Usage

```rust
use web_crawler::Crawler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crawler = Crawler::new(Crawler::default()).await?;

    let seed_urls = vec![
        "https://example.com".to_string(),
        "https://example.org".to_string(),
    ];

    let results = crawler.run(seed_urls).await?;
    println!("Crawled {} pages", results.len());
    
    Ok(())
}
```

### Custom Configuration

```rust
let config = Crawler {
    max_depth: 2,
    max_pages: 50,
    concurrent_requests: 8,
    delay_ms: 200,
    user_agent: "MyBot/1.0 (https://mysite.com/bot)".to_string(),
    respect_robots_txt: true,
};

let crawler = Crawler::new(config).await?;
```

### Running the Example

The included example crawls Rust documentation sites:

```bash
cargo run --release
```

This will crawl:
- https://www.rust-lang.org/
- https://doc.rust-lang.org/book/

## Architecture

### Core Components

#### `Crawler`
The main crawler struct that orchestrates the crawling process:
- Manages worker tasks for concurrent crawling
- Handles URL queue and visited URL tracking
- Implements rate limiting and depth control

#### `Page`
Represents a crawled web page containing:
- URL and title
- Meta description
- Full HTML content
- Extracted links

#### `RobotsTxt`
Handles robots.txt parsing and URL permission checking:
- Fetches and parses robots.txt files
- Determines if URLs are allowed to be crawled

#### `ContentStore`
Trait for storing crawled content with implementations:
- `MemoryStore`: In-memory storage for small crawls

### Data Flow

1. **Initialization**: Crawler starts with seed URLs in the queue
2. **Worker Tasks**: Multiple async workers process URLs concurrently
3. **Page Fetching**: Each worker fetches and parses HTML content
4. **Link Extraction**: New links are extracted and added to the queue
5. **Storage**: Parsed page data is stored using the ContentStore
6. **Termination**: Process continues until max pages/depth reached

## Performance Considerations

### Memory Usage
- Uses `DashSet` for thread-safe visited URL tracking
- HTML content is stored in memory (consider disk storage for large crawls)
- URL queue uses `VecDeque` for efficient FIFO operations

### Network Efficiency
- Configurable concurrent request limits
- Built-in request delays to avoid overwhelming servers
- HTTP client with connection pooling and timeouts

### Scalability
- Async/await for efficient I/O handling
- Lock-free data structures where possible
- Minimal memory allocations in hot paths

## Error Handling

The crawler handles various error conditions:

- **Network Errors**: Connection timeouts, DNS failures
- **HTTP Errors**: 4xx/5xx status codes
- **Parse Errors**: Invalid HTML or URL formats
- **Robots.txt Violations**: Blocked by robots.txt rules

All errors are logged with context for debugging.

## Dependencies

### Runtime Dependencies
- `tokio`: Async runtime and utilities
- `reqwest`: HTTP client
- `scraper`: HTML parsing and CSS selectors
- `url`: URL parsing and manipulation
- `dashmap`: Concurrent hash map
- `futures`: Async utilities

### Development Dependencies
- `tokio-test`: Testing utilities for async code
- `wiremock`: HTTP mocking for tests

## Testing

Run the test suite:

```bash
cargo test
```

For integration tests with network mocking:

```bash
cargo test --features integration-tests
```

## Known Issues

1. **Struct Definition Mismatch**: The `Page` struct definition is inconsistent between usage and declaration
2. **Syntax Errors**: Several compilation errors need fixing:
   - Missing iterator in `for url in seed_urls`
   - Incorrect field name `concurrent_request` vs `concurrent_requests`
   - Invalid syntax in config initialization (semicolon instead of comma)
3. **Missing Fields**: Some struct fields are referenced but not defined

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Examples

### Crawl a Single Domain

```rust
let config = Crawler {
    max_depth: 3,
    max_pages: 1000,
    concurrent_requests: 10,
    delay_ms: 500,
    user_agent: "MyCrawler/1.0".to_string(),
    respect_robots_txt: true,
};

let crawler = Crawler::new(config).await?;
let results = crawler.run(vec!["https://example.com".to_string()]).await?;
```

### Export Results

```rust
// After crawling
for page in results {
    println!("Title: {:?}", page.title);
    println!("URL: {}", page.url);
    println!("Links found: {}", page.links.len());
    println!("---");
}
```

## Support

For questions, issues, or contributions, please:
- Open an issue on GitHub
- Check existing documentation
- Review the code examples above

---
