use ratelimit::Ratelimiter;
use reqwest::{Client, Url};
use scraper::{node::Element, node::Node, Html};
use std::collections::VecDeque;
use std::sync::Arc;
use std::{fs, io::Read};
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};

use crate::collections::{DocQueue, UrlDb};
use crate::error::Error;
use crate::extract::Extractor;
use crate::shutdown::Shutdown;
use crate::utils;

use super::SitePolicy;

/// Crawls websites, gathering urls from pages.
pub struct Crawler {
    client: Client,
    opts: CrawlOptions,
    docs: DocQueue,
    urldb: UrlDb,
    cur_depth: usize,
    limiter: Ratelimiter,
    extractor: Extractor,
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
        extractor: Extractor,
        shutdown: Shutdown,
    ) -> Result<Self, Error> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()?;
        Self::new_with_client(client, opts, docs, urldb, extractor, shutdown)
    }

    /// Returns a new Crawler instance with the provided `reqwest::Client`.
    pub fn new_with_client(
        client: Client,
        opts: CrawlOptions,
        docs: DocQueue,
        urldb: UrlDb,
        extractor: Extractor,
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
            extractor,
            shutdown,
        };
        crawler.urldb.cond_mark_unvisited(String::from(start_url));
        Ok(crawler)
    }

    /// Crawl urls up to a given limit, scraping urls and words from pages;
    /// returns the maximum depth reached upon success.
    pub async fn crawl(&mut self) -> Result<usize, Error> {
        let semaphore = Arc::new(Semaphore::new(self.opts.limit_concurrent()));
        while self.cur_depth < self.opts.depth() {
            // if staged urls are exhausted, populate stage and ratchet up depth
            if self.urldb.num_staged_urls() < 1 {
                self.urldb.stage_unvisited_urls();
            }
            if self.urldb.num_staged_urls() < 1 {
                println!("candidate urls exhausted...");
                break;
            }
            println!("crawling at depth {}", self.cur_depth);
            let mut jhs = VecDeque::new();
            for url_str in self.urldb.staged_urls_iter() {
                if self.observe_limit().await {
                    break;
                }

                let mut spider = self.build_spider();
                let mut e = self.extractor.clone();
                let sem = semaphore.clone();
                let jh = tokio::spawn(async move {
                    let permit = sem.acquire().await.unwrap();
                    spider.crawl_url(&url_str).await;
                    e.poll_queue().await;
                    drop(permit);
                });
                jhs.push_back(jh);
            }

            while !jhs.is_empty() {
                if let Some(jh) = jhs.pop_front() {
                    //let _ = tokio::join!(jh);
                    let _ = jh.await;
                }
            }
            // outer shutdown check before depth step, as we may have been
            // shutdown before completing current depth
            if self.shutdown.is_shutdown() {
                break;
            }
            self.cur_depth += 1;
        }

        self.docs.push(None);
        Ok(self.cur_depth)
    }

    // Observes configured rate limit and returns whether or not we've been
    // shutdown.
    async fn observe_limit(&mut self) -> bool {
        tokio::select! {
          _ = async {
            if let Err(dur) = self.limiter.try_wait() {
                sleep(dur).await;
            }
            // soften how we hit the limiter
            let millis = 1000 / self.opts.req_per_sec;
            sleep(Duration::from_millis(millis)).await;
          } => {}
          _ = self.shutdown.recv() => {}
        }

        self.shutdown.is_shutdown()
    }

    // Builds and returns a new spider.
    fn build_spider(&self) -> Spider {
        Spider::new(
            self.client.clone(),
            self.opts.clone(),
            self.docs.clone(),
            self.urldb.clone(),
            self.shutdown.clone(),
        )
    }

    /// Force the current crawl depth to be of the given value.
    pub fn set_depth(&mut self, d: usize) {
        self.cur_depth = d;
    }
}

/// Crawls websites, gathering urls from pages.
#[derive(Debug, Clone)]
struct Spider {
    client: Client,
    opts: CrawlOptions,
    docs: DocQueue,
    urldb: UrlDb,
    /// Listen for shutdown notifications.
    ///
    /// A wrapper around the `broadcast::Receiver` to be paired with a sender.
    shutdown: Shutdown,
}

impl Spider {
    /// Returns a new Spider instance with the provided `reqwest::Client`.
    pub fn new(
        client: Client,
        opts: CrawlOptions,
        docs: DocQueue,
        urldb: UrlDb,
        shutdown: Shutdown,
    ) -> Self {
        Self {
            opts,
            docs,
            urldb,
            client,
            shutdown,
        }
    }

    async fn crawl_url(&mut self, url_str: &String) {
        // give us a chance to receive graceful shutdown signal
        tokio::select! {
          _ = sleep(Duration::from_millis(u64::from(utils::num_between(20, 120)))) => {}
          _ = self.shutdown.recv() => {}
        }
        if self.shutdown.is_shutdown() {
            return;
        }

        let result = Url::parse(url_str.as_str());
        if let Err(e) = result {
            eprintln!("not a url: {}", e);
            self.urldb.mark_errored(url_str.clone());
            return;
        }
        let url = result.unwrap();

        match self.opts.mode {
            CrawlMode::Web => self.crawl_web(&url).await,
            CrawlMode::Local => self.crawl_local(&url).await,
        }
    }

    async fn crawl_local(&mut self, url: &Url) {
        let path = url.to_file_path().unwrap();
        let display = path.display();

        println!("visiting {}", display);
        let meta_res = fs::metadata(&path);
        if let Err(e) = meta_res {
            self.urldb.mark_errored(url.to_string());
            eprintln!("error getting path metadata {}: {}", display, e);
            return;
        }

        let meta = meta_res.unwrap();
        if meta.is_file() {
            self.handle_local_file(&url);
        } else if meta.is_dir() {
            self.handle_local_dir(&url);
        } else {
            // ¯\_(ツ)_/¯
            return;
        }
    }

    fn handle_local_file(&mut self, url: &Url) {
        let path = url.to_file_path().unwrap();
        let display = path.display();

        let file_res = fs::File::open(&path);
        if let Err(e) = file_res {
            self.urldb.mark_errored(url.to_string());
            eprintln!("error opening file {}: {}", display, e);
            return;
        }

        let mut file = file_res.unwrap();
        let mut s = String::new();
        match file.read_to_string(&mut s) {
            Err(e) => {
                self.urldb.mark_errored(url.to_string());
                eprintln!("error reading file {}: {}", display, e);
            }
            Ok(_) => {
                self.urldb.mark_visited(url.to_string());
                self.docs.push(Some(s));
            }
        }
    }

    fn handle_local_dir(&mut self, url: &Url) {
        let path = url.to_file_path().unwrap();
        let display = path.display();

        let paths_res = fs::read_dir(&path);
        if let Err(e) = paths_res {
            self.urldb.mark_errored(url.to_string());
            eprintln!("error reading directory {}: {}", display, e);
            return;
        }
        let paths = paths_res.unwrap();
        for pr in paths {
            match pr {
                Err(e) => {
                    //self.urldb.mark_errored(url.to_string());
                    eprintln!("error reading directory contents {}: {}", display, e);
                    continue;
                }
                Ok(p) => {
                    let child = p.path().display().to_string();
                    let res = utils::url_from_path_str(child.as_str());
                    match res {
                        Err(_) => {
                            eprintln!("error parsing path as url: {}", child);
                            continue;
                        }
                        Ok(u) => {
                            self.urldb.mark_unvisited(u.to_string());
                        }
                    }
                }
            }
            self.urldb.mark_visited(url.to_string());
        }
    }

    async fn crawl_web(&mut self, url: &Url) {
        if !self.matches_site_policy(&url) {
            println!(
                "site policy '{}' violated for url: '{}', skipping...",
                self.opts.site(),
                url.as_str()
            );
            self.urldb.mark_skipped(url.as_str().to_string());
            return;
        }

        println!("visiting {}", url.as_str());
        let document = self.doc_from_url(&url.as_str().to_string()).await;
        match document {
            Ok(doc) => {
                self.urldb.mark_visited(url.to_string());
                self.urls_from_doc(&url, &doc);
                self.docs.push(Some(doc));
            }
            Err(e) => match e {
                Error::EarlyTerminationError => {
                    //self.urldb.mark_unvisited(url.to_string());
                    //eprintln!("terminated while fetching: {}", url.as_str());
                }
                _ => {
                    self.urldb.mark_errored(url.to_string());
                    eprintln!("error fetching page {}: {}", url.as_str(), e);
                }
            },
        }
    }

    /// Return whether or not the provided url matches the configured site policy.
    fn matches_site_policy(&self, url: &Url) -> bool {
        return self.opts.site().matches_policy(&self.opts.url(), &url);
    }

    /// Get an html document from the provided url.
    async fn doc_from_url(&mut self, url: &String) -> Result<String, Error> {
        tokio::select! {
            response = self.client.get(url).send() => { Self::handle_response(response).await }
            _ = self.shutdown.recv() => { Err(Error::EarlyTerminationError) }
        }
    }

    async fn handle_response(
        response: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<String, Error> {
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
                Err(Error::RequestError { why: e })
            }
            Ok(res) => Ok(res.text().await.unwrap()),
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
                    return Err(Error::UrlParseError { why: e });
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
    /// Maximum number of concurrent requests.
    limit_concurrent: usize,
    /// Crawl mode.
    mode: CrawlMode,
}

impl CrawlOptions {
    /// Returns a new CrawlOptions instance.
    pub fn new(
        url: &Url,
        depth: usize,
        include_js: bool,
        include_css: bool,
        site: SitePolicy,
        req_per_sec: u64,
        limit_concurrent: usize,
        mode: CrawlMode,
    ) -> Self {
        Self {
            url: url.clone(),
            depth,
            include_js,
            include_css,
            site,
            req_per_sec,
            limit_concurrent,
            mode,
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

    /// Returns whether or not configuration dictates  to include js.
    pub fn include_js(&self) -> bool {
        self.include_js
    }

    /// Returns whether or not configuration dictates  to include css.
    pub fn include_css(&self) -> bool {
        self.include_css
    }

    /// Returns number of configured requests per second.
    pub fn requests_per_second(&self) -> u64 {
        self.req_per_sec
    }

    /// Returns maximum number of concurrent requests allowed.
    pub fn limit_concurrent(&self) -> usize {
        self.limit_concurrent
    }

    /// Returns the configured site policy for visiting discovered URLs.
    pub fn site(&self) -> SitePolicy {
        self.site
    }
}

#[derive(Copy, Debug, Clone)]
pub enum CrawlMode {
    Web,
    Local,
}

/// Display implementation.
impl std::fmt::Display for CrawlMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Web => write!(f, "Web"),
            Self::Local => write!(f, "Local"),
        }
    }
}

impl PartialEq for CrawlMode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Web, Self::Web) | (Self::Local, Self::Local) => true,
            _ => false,
        }
    }
}
