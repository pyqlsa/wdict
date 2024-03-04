use crate::error::Error;
use crate::shutdown::Shutdown;
use crate::site::SitePolicy;

use async_recursion::async_recursion;
use ratelimit::Ratelimiter;
use reqwest::{Client, Url};
use scraper::{node::Element, node::Node, Html};
use std::collections::HashMap;
use std::time::Duration;

/// Crawls websites, gathering urls from pages.
pub struct Crawler {
    opts: CrawlOptions,
    urls: HashMap<String, bool>,
    docs: Vec<Html>,
    cur_depth: usize,
    client: Client,
    limiter: Ratelimiter,
    /// Listen for shutdown notifications.
    ///
    /// A wrapper around the `broadcast::Receiver` to be paired with a sender.
    shutdown: Shutdown,
}

impl Crawler {
    /// Returns a new Crawler instance with the default `reqwest::Client`.
    pub fn new(opts: CrawlOptions, shutdown: Shutdown) -> Result<Self, Error> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Self::new_with_client(opts, client, shutdown)
    }

    /// Returns a new Crawler instance with the provided `reqwest::Client`.
    pub fn new_with_client(
        opts: CrawlOptions,
        client: Client,
        shutdown: Shutdown,
    ) -> Result<Self, Error> {
        let tokens = if opts.requests_per_second() < 1 {
            1
        } else {
            opts.requests_per_second()
        };
        let limiter = Ratelimiter::builder(tokens, Duration::from_secs(1))
            .max_tokens(tokens)
            .initial_available(tokens / 2)
            .build()?;
        let start_url = opts.url.clone();
        let mut crawler = Self {
            opts,
            urls: HashMap::new(),
            docs: Vec::new(),
            cur_depth: 0,
            client,
            limiter,
            shutdown,
        };
        crawler.mark_to_be_visited(String::from(start_url));
        Ok(crawler)
    }

    /// Returns the crawled urls.
    pub fn urls(&self) -> HashMap<String, bool> {
        self.urls.clone()
    }

    /// Returns the urls that were visited.
    pub fn visited_urls(&self) -> Vec<String> {
        self.urls()
            .into_iter()
            .filter(|(_k, v)| *v)
            .map(|(k, _v)| k)
            .collect()
    }

    /// Returns the urls that were discovered, but not visited.
    pub fn not_visited_urls(&self) -> Vec<String> {
        self.urls()
            .into_iter()
            .filter(|(_k, v)| !*v)
            .map(|(k, _v)| k)
            .collect()
    }

    /// Returns the gathered documents.
    pub fn docs(&self) -> Vec<Html> {
        self.docs.clone()
    }

    /// Inserts and marks a url as visited.
    pub fn mark_visited(&mut self, url: String) -> () {
        self.urls.insert(url.to_owned(), true);
    }

    /// Inserts and marks a url as not yet visited.
    pub fn mark_to_be_visited(&mut self, url: String) -> () {
        self.urls.insert(url.to_owned(), false);
    }

    /// Crawl urls up to a given limit, scraping urls and words from pages.
    #[async_recursion(?Send)]
    pub async fn crawl(&mut self) -> Result<(), Error> {
        self.cur_depth += 1;
        for (url, visited) in self.urls.clone().iter() {
            if self.shutdown.is_shutdown() {
                break;
            }
            let result = Url::parse(url.as_str());
            if let Err(e) = result {
                eprintln!("not a url: {}", e);
                continue;
            }
            if !self
                .opts
                .site()
                .matches_policy(self.opts.url(), result.clone().unwrap())
            {
                println!(
                    "site policy '{}' violated for url: '{}', skipping...",
                    self.opts.site(),
                    url
                );
                continue;
            }

            if *visited {
                println!("already visited '{}', skipping", url);
                continue;
            }

            println!("visiting {}", url);
            self.mark_visited(url.to_owned());
            let document = self.doc_from_url(String::from(url)).await;
            match document {
                Ok(doc) => {
                    self.urls_from_doc(&result.unwrap(), &doc);
                    self.docs.push(doc);
                }
                Err(e) => eprintln!("{}", e),
            }
        }
        if self.cur_depth < self.opts.depth() {
            println!("going deeper... current depth {}", self.cur_depth);
            let _ = self.crawl().await; // safe to ignore result/error
        }
        Ok(())
    }

    /// Get an html document from the provided url.
    async fn doc_from_url(&self, url: String) -> Result<Html, Error> {
        // TODO: do this smarter?
        for _ in 0..10 {
            if self.shutdown.is_shutdown() {
                return Err(Error::EarlyTerminationError);
            }
            if let Err(sleep) = self.limiter.try_wait() {
                std::thread::sleep(sleep);
                continue;
            } else {
                break;
            }
        }

        let response = self.client.get(url).send().await;
        match response {
            Err(e) => {
                if e.is_status() {
                    if let Some(status_code) = e.status() {
                        match status_code.as_u16() {
                            429 => {
                                // breadcrumbs? https://github.com/rust-lang-nursery/rust-cookbook/pull/395/files
                                println!("wait and retry on 429 not implemented, skipping...");
                            }
                            _ => {
                                println!("unexpected status code: {}", status_code);
                            }
                        }
                    } else {
                        eprintln!("expected error with status code, but found none: {}", e);
                    }
                } else {
                    eprintln!("unexpected error: {}", e);
                }
                Err(Error::Request { why: e })
            }
            Ok(res) => {
                let doc_text = res.text().await.unwrap();

                Ok(Html::parse_document(&doc_text))
            }
        }
    }

    /// Extract urls from an html document.
    fn urls_from_doc(&mut self, url: &Url, document: &Html) -> () {
        // breadcrumbs for using selector to extract urls from elements...
        //let link_selector = Selector::parse(r#"a[href^="http"]"#).unwrap();
        //
        //for elem in document.clone().select(&link_selector) {
        //    self.urls
        //        .entry(String::from(elem.value().attr("href").unwrap()))
        //        .or_insert(false);
        //}
        for d in document.clone().root_element().descendants() {
            if let Node::Element(elem) = d.value() {
                if let Some(href) = elem.attr("href") {
                    let final_url = Self::url_from_href(url, href);
                    match final_url {
                        Err(_e) => continue, // just skip href;
                        Ok(u) => self.conditional_insert_url(u, elem),
                    }
                }
            }
        }
    }

    /// Based on a page's original url, return a url based on the given href extracted from the
    /// page.
    fn url_from_href(url: &Url, href: &str) -> Result<String, Error> {
        if href == "" || href.starts_with('#') {
            // ignore whitespace and anchors; probably need a better error for this, but it's
            // generally ignored anyways
            return Err(Error::StrWhitespaceError);
        }
        let result = Url::parse(href);
        match result {
            Ok(u) => {
                // must be a valid full url
                return Ok(String::from(u.as_str()));
            }
            // TODO: this section could be tidied for clarity, but errors are generally ignored
            // from this function anyways
            Err(e) => {
                // likely a relative url
                if let Ok(joined_url) = url.join(href) {
                    return Ok(String::from(joined_url.as_str()));
                } else {
                    return Err(Error::UrlParsing { why: e });
                }
            }
        }
    }

    /// Conditionally save the given url if it adheres to configured options,
    /// determined by the given element; given element is intended to be the
    /// element from which  the url was extracted from.
    fn conditional_insert_url(&mut self, url: String, elem: &Element) -> () {
        if url.as_str() != "" {
            match elem.name() {
                "link" => {
                    if self.opts.include_css() && elem.attr("rel").unwrap() == "stylesheet" {
                        self.urls.entry(url.clone()).or_insert(false);
                    }
                    if self.opts.include_js() && elem.attr("as").unwrap() == "script" {
                        self.urls.entry(url.clone()).or_insert(false);
                    }
                }
                "a" => {
                    self.urls.entry(url.clone()).or_insert(false);
                }
                _ => {}
            }
        }
    }
}

/// Options used when crawling and building wordlists.
#[derive(Debug, Clone)]
pub struct CrawlOptions {
    /// Url to start crawling from.
    url: Url,
    /// Limit the depth of crawling urls.
    depth: usize,
    /// Include javascript from html pages.
    include_js: bool,
    /// Include css from html pages.
    include_css: bool,
    /// Strategy for url crawling.
    site: SitePolicy,
    /// Upper limit of requests per second while crawling.
    req_per_sec: u64,
}

impl CrawlOptions {
    /// Returns a new CrawlOptions instance.
    pub fn new(
        url: Url,
        depth: usize,
        req_per_sec: u64,
        include_js: bool,
        include_css: bool,
        site: SitePolicy,
    ) -> Self {
        Self {
            url,
            depth,
            req_per_sec,
            include_js,
            include_css,
            site,
        }
    }

    /// Returns the url where crawling initiated from.
    pub fn url(&self) -> Url {
        self.url.clone()
    }

    /// Returns the url search depth used for crawling.
    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn requests_per_second(&self) -> u64 {
        self.req_per_sec
    }

    /// Returns whether or not configuration dictates  to include js.
    pub fn include_js(&self) -> bool {
        self.include_js
    }

    /// Returns whether or not configuration dictates  to include css.
    pub fn include_css(&self) -> bool {
        self.include_css
    }

    /// Returns the configured site policy for visiting discovered URLs.
    pub fn site(&self) -> SitePolicy {
        self.site
    }
}
