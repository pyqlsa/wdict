use crate::doc_queue::DocQueue;
use crate::error::Error;
use crate::shutdown::Shutdown;
use crate::site::SitePolicy;
use crate::urldb::UrlDb;

use ratelimit::Ratelimiter;
use reqwest::{Client, Url};
use scraper::{node::Element, node::Node, Html};
use std::time::Duration;

/// Crawls websites, gathering urls from pages.
pub struct Crawler {
    client: Client,
    opts: CrawlOptions,
    docs: DocQueue,
    urldb: UrlDb,
    cur_depth: usize,
    limiter: Ratelimiter,
    /// Listen for shutdown notifications.
    ///
    /// A wrapper around the `broadcast::Receiver` to be paired with a sender.
    shutdown: Shutdown,
}

impl Crawler {
    /// Returns a new Crawler instance with the default `reqwest::Client`.
    pub fn new(
        opts: CrawlOptions,
        docs: DocQueue,
        urldb: UrlDb,
        shutdown: Shutdown,
    ) -> Result<Self, Error> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Self::new_with_client(client, opts, docs, urldb, shutdown)
    }

    /// Returns a new Crawler instance with the provided `reqwest::Client`.
    pub fn new_with_client(
        client: Client,
        opts: CrawlOptions,
        docs: DocQueue,
        urldb: UrlDb,
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
            docs,
            urldb,
            cur_depth: 0,
            client,
            limiter,
            shutdown,
        };
        crawler.urldb.mark_unvisited(String::from(start_url));
        Ok(crawler)
    }

    /// Crawl urls up to a given limit, scraping urls and words from pages.
    pub async fn crawl(&mut self) -> Result<(), Error> {
        while self.cur_depth < self.opts.depth() {
            self.cur_depth += 1;
            println!("crawling at depth {}", self.cur_depth);
            for url in self.urldb.unvisited_urls() {
                // give us a chance to receive graceful shutdown signal
                tokio::select! {
                  _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
                  _ = self.shutdown.recv() => {}
                }
                if self.shutdown.is_shutdown() {
                    break;
                }

                let result = Url::parse(url.as_str());
                if let Err(e) = result {
                    eprintln!("not a url: {}", e);
                    self.urldb.mark_errored(url.to_owned());
                    continue;
                }
                if !self.matches_site_policy(&result.clone().unwrap()) {
                    println!(
                        "site policy '{}' violated for url: '{}', skipping...",
                        self.opts.site(),
                        url
                    );
                    self.urldb.mark_skipped(url.to_owned());
                    continue;
                }

                println!("visiting {}", url);
                let document = self.doc_from_url(String::from(url.to_owned())).await;
                match document {
                    Ok(doc) => {
                        self.urldb.mark_visited(url.to_owned());
                        self.urls_from_doc(&result.unwrap(), &doc);
                        self.docs.push(Some(doc));
                    }
                    Err(e) => {
                        self.urldb.mark_errored(url.to_owned());
                        eprintln!("error fetching page: {}", e);
                    }
                }
            }
            if self.shutdown.is_shutdown() {
                break;
            }
        }

        self.docs.push(None);
        Ok(())
    }

    /// Return whether or not the provided url matches the configured site policy.
    fn matches_site_policy(&self, url: &Url) -> bool {
        return self.opts.site().matches_policy(&self.opts.url(), &url);
    }

    /// Get an html document from the provided url.
    async fn doc_from_url(&self, url: String) -> Result<String, Error> {
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
                //Ok(Html::parse_document(&doc_text))
                Ok(res.text().await.unwrap())
            }
        }
    }

    /// Extract urls from an html document.
    fn urls_from_doc(&mut self, url: &Url, document: &String) -> () {
        let doc = Html::parse_document(&document);
        // breadcrumbs for using selector to extract urls from elements...
        //let link_selector = Selector::parse(r#"a[href^="http"]"#).unwrap();
        //
        //for elem in document.clone().select(&link_selector) {
        //    self.urls
        //        .entry(String::from(elem.value().attr("href").unwrap()))
        //        .or_insert(false);
        //}
        for d in doc.root_element().descendants() {
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
                        self.urldb.cond_mark_unvisited(url.clone());
                    }
                    if self.opts.include_js() && elem.attr("as").unwrap() == "script" {
                        self.urldb.cond_mark_unvisited(url.clone());
                    }
                }
                "a" => {
                    self.urldb.cond_mark_unvisited(url.clone());
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
