use crate::error::Error;
use crate::site::SitePolicy;

use async_recursion::async_recursion;
use ratelimit::Ratelimiter;
use reqwest::{Client, Url};
use scraper::{node::Element, node::Node, Html};
use std::collections::HashMap;
use std::time::Duration;

/// Crawls websites, gathering links and words from pages.
pub struct Crawler {
    opts: CrawlOptions,
    links: HashMap<String, bool>,
    docs: Vec<Html>,
    cur_depth: usize,
    client: Client,
    limiter: Ratelimiter,
}

impl Crawler {
    /// Returns a new Crawler instance with the default `reqwest::Client`.
    pub fn new(
        url: Url,
        depth: usize,
        req_per_sec: u64,
        include_js: bool,
        include_css: bool,
        site: SitePolicy,
    ) -> Result<Self, Error> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Self::new_with_client(
            url,
            depth,
            req_per_sec,
            include_js,
            include_css,
            site,
            client,
        )
    }

    /// Returns a new Crawler instance with the provided `reqwest::Client`.
    pub fn new_with_client(
        url: Url,
        depth: usize,
        req_per_sec: u64,
        include_js: bool,
        include_css: bool,
        site: SitePolicy,
        client: Client,
    ) -> Result<Self, Error> {
        let tokens = if req_per_sec < 1 { 1 } else { req_per_sec };
        let limiter = Ratelimiter::builder(tokens, Duration::from_secs(1))
            .max_tokens(tokens)
            .initial_available(tokens / 2)
            .build()?;
        let mut crawler = Self {
            opts: CrawlOptions::new(url.clone(), depth, include_js, include_css, site),
            links: HashMap::new(),
            docs: Vec::new(),
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

    /// Returns the gathered documents.
    pub fn docs(&self) -> Vec<Html> {
        self.docs.clone()
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
        for (url, visited) in self.links.clone().iter() {
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
            self.visited(url.to_owned());
            let document = self.doc_from_url(String::from(url)).await;
            self.links_from_doc(&result.unwrap(), &document);
            self.docs.push(document);
        }
        if self.cur_depth < self.opts.depth() {
            println!("going deeper... current depth {}", self.cur_depth);
            self.crawl().await;
        }
    }

    /// Get an html document from the provided url.
    async fn doc_from_url(&self, url: String) -> Html {
        // TODO: do this smarter?
        for _ in 0..10 {
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

                // Return empty doc, as there's nothing to parse.
                Html::new_document()
            }
            Ok(res) => {
                let doc_text = res.text().await.unwrap();

                Html::parse_document(&doc_text)
            }
        }
    }

    /// Extract links from an html document.
    fn links_from_doc(&mut self, url: &Url, document: &Html) -> () {
        // breadcrumbs for using selector to extract links from elements...
        //let link_selector = Selector::parse(r#"a[href^="http"]"#).unwrap();
        //
        //for elem in document.clone().select(&link_selector) {
        //    self.links
        //        .entry(String::from(elem.value().attr("href").unwrap()))
        //        .or_insert(false);
        //}
        for d in document.clone().root_element().descendants() {
            if let Node::Element(elem) = d.value() {
                if let Some(href) = elem.attr("href") {
                    if href != "" {
                        let result = Url::parse(href);
                        match result {
                            Err(_e) => {
                                // likely a relative url
                                if href.starts_with('#') {
                                    // ignore anchor links
                                    continue;
                                }
                                if let Ok(base_url) =
                                    Url::parse(url.origin().unicode_serialization().as_str())
                                {
                                    if let Ok(joined_url) = base_url.join(href) {
                                        let final_url = String::from(joined_url.as_str());
                                        self.conditional_insert_link(final_url, elem)
                                    } else {
                                        // errors along the way, skipping href
                                        continue;
                                    }
                                } else {
                                    // errors along the way, skipping href
                                    continue;
                                }
                            }
                            Ok(u) => {
                                // must be a valid full url
                                let final_url = String::from(u.as_str());
                                self.conditional_insert_link(final_url, elem)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Conditionally save the given url if it adheres to configured options,
    /// determined by the given element; given element is intended to be the
    /// element from which  the url was extracted from.
    fn conditional_insert_link(&mut self, url: String, elem: &Element) -> () {
        if url.as_str() != "" {
            match elem.name() {
                "link" => {
                    if self.opts.include_css() && elem.attr("rel").unwrap() == "stylesheet" {
                        self.links.entry(url.clone()).or_insert(false);
                    }
                    if self.opts.include_js() && elem.attr("as").unwrap() == "script" {
                        self.links.entry(url.clone()).or_insert(false);
                    }
                }
                "a" => {
                    self.links.entry(url.clone()).or_insert(false);
                }
                _ => {}
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
    /// Include javascript from html pages.
    include_js: bool,
    /// Include css from html pages.
    include_css: bool,
    /// Strategy for link crawling.
    site: SitePolicy,
}

impl CrawlOptions {
    /// Returns a new CrawlOptions instance.
    fn new(url: Url, depth: usize, include_js: bool, include_css: bool, site: SitePolicy) -> Self {
        Self {
            url,
            depth,
            include_js,
            include_css,
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

    /// Returns whether or not configuration dictates  to include js.
    fn include_js(&self) -> bool {
        self.include_js
    }

    /// Returns whether or not configuration dictates  to include css.
    fn include_css(&self) -> bool {
        self.include_css
    }

    /// Returns the configured site policy for visiting discovered URLs.
    fn site(&self) -> SitePolicy {
        self.site
    }
}
