use crate::error::Error;
use crate::filter::FilterMode;
use crate::site::SitePolicy;

use async_recursion::async_recursion;
use ratelimit::Ratelimiter;
use reqwest::{Client, Url};
use scraper::{node::Node, Html, Selector};
use std::collections::HashMap;
use std::time::Duration;

/// Crawls websites, gathering links and words from pages.
pub struct Crawler {
    opts: CrawlOptions,
    links: HashMap<String, bool>,
    words: HashMap<String, bool>,
    cur_depth: usize,
    client: Client,
    limiter: Ratelimiter,
}

impl Crawler {
    /// Returns a new Crawler instance with the default `reqwest::Client`.
    pub fn new(
        url: Url,
        depth: usize,
        min_word_length: usize,
        req_per_sec: u64,
        filters: Vec<FilterMode>,
        site: SitePolicy,
    ) -> Result<Self, Error> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Self::new_with_client(
            url,
            depth,
            min_word_length,
            req_per_sec,
            filters,
            site,
            client,
        )
    }

    /// Returns a new Crawler instance with the provided `reqwest::Client`.
    pub fn new_with_client(
        url: Url,
        depth: usize,
        min_word_length: usize,
        req_per_sec: u64,
        filters: Vec<FilterMode>,
        site: SitePolicy,
        client: Client,
    ) -> Result<Self, Error> {
        let tokens = if req_per_sec < 1 { 1 } else { req_per_sec };
        let limiter = Ratelimiter::builder(tokens, Duration::from_secs(1))
            .max_tokens(tokens)
            .initial_available(tokens / 2)
            .build()?;
        let mut crawler = Self {
            opts: CrawlOptions::new(url.clone(), depth, min_word_length, filters, site),
            links: HashMap::new(),
            words: HashMap::new(),
            cur_depth: 0,
            client,
            limiter,
        };
        crawler.to_be_visited(String::from(url));
        Ok(crawler)
    }

    /// Returns the crawled links.
    pub fn links(&self) -> HashMap<String, bool> {
        self.links.clone()
    }

    /// Returns the gathered words.
    pub fn words(&self) -> HashMap<String, bool> {
        self.words.clone()
    }

    /// Inserts and marks a link as visited.
    pub fn visited(&mut self, url: String) -> () {
        self.links.insert(url.to_owned(), true);
    }

    /// Inserts and marks a link as not yet visited.
    pub fn to_be_visited(&mut self, url: String) -> () {
        self.links.insert(url.to_owned(), false);
    }

    /// Crawl links up to a given limit, scraping links and words from pages.
    #[async_recursion(?Send)]
    pub async fn crawl(&mut self) -> () {
        self.cur_depth += 1;
        for (url, visited) in self.links.clone().into_iter() {
            let result = Url::parse(url.as_str());
            if let Err(e) = result {
                eprintln!("not a url: {}", e);
                continue;
            }
            if !self
                .opts
                .site()
                .matches_policy(self.opts.url(), result.unwrap())
            {
                println!(
                    "site policy '{}' violated for url: '{}', skipping...",
                    self.opts.site(),
                    url
                );
                continue;
            }

            if visited {
                println!("already visited '{}', skipping", url);
                continue;
            }

            println!("visiting {}", url);
            self.visited(url.to_owned());
            let document = self.doc_from_url(String::from(url)).await;
            self.links_from_doc(&document);
            self.words_from_doc(&document);
        }
        if self.cur_depth < self.opts.depth() {
            println!("going deeper... current depth {}", self.cur_depth);
            self.crawl().await;
        }
    }

    /// Get an html document from the provided url.
    async fn doc_from_url(&self, url: String) -> Html {
        // TODO: do this smarter
        for _ in 0..10 {
            if let Err(sleep) = self.limiter.try_wait() {
                std::thread::sleep(sleep);
                continue;
            }
        }

        let response = self.client.get(url).send().await;
        match response {
            Err(e) => {
                if e.is_status() {
                    let status_code = e.status().expect("derp").as_u16();
                    match status_code {
                        429 => {
                            println!("wait and retry on 429 not implemented, skipping...")
                        }
                        _ => {
                            println!("Unexpected status code: {}", status_code);
                        }
                    }
                } else {
                    eprintln!("unknown error: {}", e);
                }
                Html::new_document()
            }
            Ok(res) => {
                let doc_text = res.text().await.unwrap();

                Html::parse_document(&doc_text)
            }
        }
    }

    /// Extract links from an html document.
    fn links_from_doc(&mut self, document: &Html) -> () {
        let link_selector = Selector::parse(r#"a[href^="http"]"#).unwrap();

        for elem in document.clone().select(&link_selector) {
            self.links
                .entry(String::from(elem.value().attr("href").unwrap()))
                .or_insert(false);
        }
    }

    /// Extract words from an html document.
    fn words_from_doc(&mut self, document: &Html) -> () {
        for node in document.clone().tree {
            if let Node::Text(text) = node {
                let mut fintext = text.text.trim().to_string();
                for filter in self.opts.filters() {
                    fintext = filter.filter_str(&fintext);
                }
                fintext = fintext.to_lowercase();
                // ignore these characters since we're looking for words
                fintext = fintext.replace(|c: char| !c.is_alphanumeric(), " ");
                if fintext.len() > 0 {
                    for w in fintext.split_whitespace() {
                        if w.len() >= self.opts.min_word_length() {
                            self.words.entry(String::from(w)).or_insert(true);
                        }
                    }
                }
            }
        }
    }
}

/// Options used when crawling and building wordlists.
#[derive(Debug, Clone)]
struct CrawlOptions {
    /// Url to start crawling from.
    url: Url,
    /// Limit the depth of crawling links.
    depth: usize,
    /// Only save words greater than or equal to this value.
    min_word_length: usize,
    /// Filter strategy for words; multiple can be specified.
    filters: Vec<FilterMode>,
    /// Strategy for link crawling.
    site: SitePolicy,
}

impl CrawlOptions {
    /// Returns a new CrawlOptions instance.
    fn new(
        url: Url,
        depth: usize,
        min_word_length: usize,
        filters: Vec<FilterMode>,
        site: SitePolicy,
    ) -> Self {
        Self {
            url,
            depth,
            min_word_length,
            filters,
            site,
        }
    }

    /// Returns the url where crawling initiated from.
    fn url(&self) -> Url {
        self.url.clone()
    }

    /// Returns the link search depth used for crawling.
    fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the minimum word length for saving words to the wordslist.
    fn min_word_length(&self) -> usize {
        self.min_word_length
    }

    /// Returns the configured filter mode for discovered words.
    fn filters(&self) -> Vec<FilterMode> {
        self.filters.clone()
    }

    /// Returns the configured site policy for visiting discovered URLs.
    fn site(&self) -> SitePolicy {
        self.site
    }
}
