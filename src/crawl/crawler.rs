use bytes::Bytes;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{debug, info, trace, warn};
use ratelimit::Ratelimiter;
use reqwest::{Client, Url};
use scraper::{node::Element, node::Node, Html};
use std::collections::VecDeque;
use std::sync::Arc;
use std::{fs, io::Read};
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};

use crate::collections::UrlDb;
use crate::error::Error;
use crate::extract::Extractor;
use crate::shutdown::Shutdown;
use crate::utils;

use super::SitePolicy;

/// Crawls websites, gathering urls from pages.
pub struct Crawler {
    client: Client,
    opts: CrawlOptions,
    urldb: UrlDb,
    cur_depth: usize,
    limiter: Ratelimiter,
    extractor: Extractor,
    multiprog: MultiProgress,
    /// Listen for shutdown notifications.
    ///
    /// A wrapper around the `broadcast::Receiver` to be paired with a sender.
    shutdown: Shutdown,
}

impl Crawler {
    /// Returns a new Crawler instance with the default `reqwest::Client`.
    pub fn new(
        opts: CrawlOptions,
        urldb: UrlDb,
        extractor: Extractor,
        shutdown: Shutdown,
        multiprog: MultiProgress,
    ) -> Result<Self, Error> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()?;
        Self::new_with_client(client, opts, urldb, extractor, shutdown, multiprog)
    }

    /// Returns a new Crawler instance with the provided `reqwest::Client`.
    pub fn new_with_client(
        client: Client,
        opts: CrawlOptions,
        urldb: UrlDb,
        extractor: Extractor,
        shutdown: Shutdown,
        multiprog: MultiProgress,
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
            urldb,
            cur_depth: 0,
            client,
            limiter,
            extractor,
            shutdown,
            multiprog,
        };
        crawler.urldb.cond_mark_unvisited(String::from(start_url));
        Ok(crawler)
    }

    /// Crawl urls up to a given limit, extracting words from documents;
    /// in web mode, links are extracted from web pages to spider;
    /// in local mode, the directory structure is traversed to find documents;
    /// returns the maximum depth reached upon success.
    pub async fn crawl(&mut self) -> Result<usize, Error> {
        let semaphore = Arc::new(Semaphore::new(self.opts.limit_concurrent()));
        while self.cur_depth < self.opts.depth() {
            // if staged urls are exhausted, populate stage and ratchet up depth
            if self.urldb.num_staged_urls() < 1 {
                self.urldb.stage_unvisited_urls();
            }
            if self.urldb.num_staged_urls() < 1 {
                info!("candidate urls exhausted...");
                break;
            }
            info!("crawling at depth {}", self.cur_depth);
            let pb = self.new_staged_progress();
            let mut jhs = VecDeque::new();
            for url_str in self.urldb.staged_urls_iter() {
                if self.observe_limit().await {
                    break;
                }

                let mut spider = self.build_spider();
                let mut e = self.extractor.clone();
                let sem = semaphore.clone();
                let pbc = pb.clone();
                let jh = tokio::spawn(async move {
                    let permit = sem.acquire().await.unwrap();
                    if let Some(doc) = spider.crawl_url(&url_str).await {
                        e.words_from_doc(&doc);
                    }
                    pbc.inc(1);
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
                pb.abandon_with_message("shutdown early...");
                self.multiprog.remove(&pb);
                break;
            }
            self.cur_depth += 1;
            pb.finish();
            self.multiprog.remove(&pb);
        }

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
            self.urldb.clone(),
            self.shutdown.clone(),
        )
    }

    /// Force the current crawl depth to be of the given value.
    pub fn set_depth(&mut self, d: usize) {
        self.cur_depth = d;
    }

    /// Returns a new progress bar to track urls in the next stage;
    /// if we have too many urls for some reason, returns a spinner.
    fn new_staged_progress(&self) -> ProgressBar {
        let pb = if let Ok(size) = self.urldb.num_staged_urls().try_into() {
            let bar = styled_progress(size);
            self.multiprog.add(bar)
        } else {
            let bar = styled_spinner();
            self.multiprog.add(bar)
        };
        pb.set_prefix(format!("crawl depth: {}", self.cur_depth));
        pb
    }
}

fn styled_progress(size: u64) -> ProgressBar {
    // light - ░
    // medium - ▒
    // dark - ▓
    // solid - █
    let bar = ProgressBar::new(size);
    // deliberate 2 spaces at the end of the template string
    bar.set_style(
        ProgressStyle::with_template("[{prefix:.cyan}] |{wide_bar}| {pos:.blue}/{len:.blue}  ")
            .unwrap()
            .progress_chars("▓░"), // dark/light
    );
    bar
}

fn styled_spinner() -> ProgressBar {
    let bar = ProgressBar::new_spinner();
    bar.set_style(
        ProgressStyle::with_template("[{prefix:.cyan}] |{spinner}|")
            .unwrap()
            .tick_strings(&[
                "░░░░░░░░░░░░░░░░░░░░",
                "░░░░░░░░░░░░░░░░░░░░",
                "▓░░░░░░░░░░░░░░░░░░░",
                "▓▓░░░░░░░░░░░░░░░░░░",
                "▓▓▓░░░░░░░░░░░░░░░░░",
                "▓▓▓▓░░░░░░░░░░░░░░░░",
                "▓▓▓▓▓░░░░░░░░░░░░░░░",
                "▓▓▓▓▓▓░░░░░░░░░░░░░░",
                "▓▓▓▓▓▓▓░░░░░░░░░░░░░",
                "▓▓▓▓▓▓▓▓░░░░░░░░░░░░",
                "▓▓▓▓▓▓▓▓▓░░░░░░░░░░░",
                "▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░",
                "░▓▓▓▓▓▓▓▓▓▓░░░░░░░░░",
                "░░▓▓▓▓▓▓▓▓▓▓░░░░░░░░",
                "░░░▓▓▓▓▓▓▓▓▓▓░░░░░░░",
                "░░░░▓▓▓▓▓▓▓▓▓▓░░░░░░",
                "░░░░░▓▓▓▓▓▓▓▓▓▓░░░░░",
                "░░░░░░▓▓▓▓▓▓▓▓▓▓░░░░",
                "░░░░░░░▓▓▓▓▓▓▓▓▓▓░░░",
                "░░░░░░░░▓▓▓▓▓▓▓▓▓▓░░",
                "░░░░░░░░░▓▓▓▓▓▓▓▓▓▓░",
                "░░░░░░░░░░▓▓▓▓▓▓▓▓▓▓",
                "░░░░░░░░░░░▓▓▓▓▓▓▓▓▓",
                "░░░░░░░░░░░░▓▓▓▓▓▓▓▓",
                "░░░░░░░░░░░░░▓▓▓▓▓▓▓",
                "░░░░░░░░░░░░░░▓▓▓▓▓▓",
                "░░░░░░░░░░░░░░░▓▓▓▓▓",
                "░░░░░░░░░░░░░░░░▓▓▓▓",
                "░░░░░░░░░░░░░░░░░▓▓▓",
                "░░░░░░░░░░░░░░░░░░▓▓",
                "░░░░░░░░░░░░░░░░░░░▓",
                "░░░░░░░░░░░░░░░░░░░░",
                "░░░░░░░░░░░░░░░░░░░░",
                "░░░░░░░░░░░░░░░░░░░░",
            ]),
    );
    bar.enable_steady_tick(Duration::from_millis(100));
    bar
}

/// Crawls websites, gathering urls from pages.
#[derive(Debug, Clone)]
struct Spider {
    client: Client,
    opts: CrawlOptions,
    urldb: UrlDb,
    /// Listen for shutdown notifications.
    ///
    /// A wrapper around the `broadcast::Receiver` to be paired with a sender.
    shutdown: Shutdown,
}

impl Spider {
    /// Returns a new Spider instance with the provided `reqwest::Client`.
    pub fn new(client: Client, opts: CrawlOptions, urldb: UrlDb, shutdown: Shutdown) -> Self {
        Self {
            opts,
            urldb,
            client,
            shutdown,
        }
    }

    async fn crawl_url(&mut self, url_str: &String) -> Option<Bytes> {
        // give us a chance to receive graceful shutdown signal
        tokio::select! {
          _ = sleep(Duration::from_millis(u64::from(utils::num_between(20, 120)))) => {}
          _ = self.shutdown.recv() => {}
        }
        if self.shutdown.is_shutdown() {
            return None;
        }

        let result = Url::parse(url_str.as_str());
        if let Err(e) = result {
            debug!("not a url: {}", e);
            self.urldb.mark_errored(url_str.clone());
            return None;
        }
        let url = result.unwrap();

        match self.opts.mode {
            CrawlMode::Web => self.crawl_web(&url).await,
            CrawlMode::Local => self.crawl_local(&url).await,
        }
    }

    async fn crawl_local(&mut self, url: &Url) -> Option<Bytes> {
        let path = url.to_file_path().unwrap();
        let display = path.display();

        trace!("visiting {}", display);
        let meta_res = fs::metadata(&path);
        if let Err(e) = meta_res {
            self.urldb.mark_errored(url.to_string());
            warn!("error getting path metadata {}: {}", display, e);
            return None;
        }

        let meta = meta_res.unwrap();
        if meta.is_file() {
            self.handle_local_file(&url)
        } else if meta.is_dir() {
            self.handle_local_dir(&url);
            None
        } else {
            // ¯\_(ツ)_/¯
            None
        }
    }

    fn handle_local_file(&mut self, url: &Url) -> Option<Bytes> {
        let path = url.to_file_path().unwrap();
        let display = path.display();

        let file_res = fs::File::open(&path);
        if let Err(e) = file_res {
            self.urldb.mark_errored(url.to_string());
            warn!("error opening file {}: {}", display, e);
            return None;
        }

        let mut file = file_res.unwrap();
        let mut buf = Vec::new();
        match file.read_to_end(&mut buf) {
            Err(e) => {
                self.urldb.mark_errored(url.to_string());
                warn!("error reading file {}: {}", display, e);
                None
            }
            Ok(_) => {
                self.urldb.mark_visited(url.to_string());
                Some(Bytes::from(buf))
            }
        }
    }

    fn handle_local_dir(&mut self, url: &Url) -> () {
        let path = url.to_file_path().unwrap();
        let display = path.display();

        let paths_res = fs::read_dir(&path);
        if let Err(e) = paths_res {
            self.urldb.mark_errored(url.to_string());
            warn!("error reading directory {}: {}", display, e);
            return;
        }
        let paths = paths_res.unwrap();
        for pr in paths {
            match pr {
                Err(e) => {
                    //self.urldb.mark_errored(url.to_string());
                    warn!("error reading directory contents {}: {}", display, e);
                    continue;
                }
                Ok(p) => {
                    let child = p.path().display().to_string();
                    let res = utils::url_from_path_str(child.as_str());
                    match res {
                        Err(_) => {
                            warn!("error parsing path as url: {}", child);
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

    async fn crawl_web(&mut self, url: &Url) -> Option<Bytes> {
        if !self.matches_site_policy(&url) {
            debug!(
                "site policy '{}' violated for url: '{}', skipping...",
                self.opts.site(),
                url.as_str()
            );
            self.urldb.mark_skipped(url.as_str().to_string());
            return None;
        }

        trace!("visiting {}", url.as_str());
        let document = self.doc_from_url(&url.as_str().to_string()).await;
        match document {
            Ok(doc) => {
                let doc_string = String::from_utf8_lossy(&doc).to_string();
                self.urldb.mark_visited(url.to_string());
                self.urls_from_doc(&url, &doc_string);
                Some(doc)
            }
            Err(e) => match e {
                Error::EarlyTerminationError => {
                    // empty on purpose for now;
                    //
                    //self.urldb.mark_unvisited(url.to_string());
                    //debug!("terminated while fetching: {}", url.as_str());
                    None
                }
                _ => {
                    self.urldb.mark_errored(url.to_string());
                    warn!("error fetching page {}: {}", url.as_str(), e);
                    None
                }
            },
        }
    }

    /// Return whether or not the provided url matches the configured site policy.
    fn matches_site_policy(&self, url: &Url) -> bool {
        return self.opts.site().matches_policy(&self.opts.url(), &url);
    }

    /// Get an html document from the provided url.
    async fn doc_from_url(&mut self, url: &String) -> Result<Bytes, Error> {
        tokio::select! {
            response = self.client.get(url).send() => { Self::handle_response(response).await }
            _ = self.shutdown.recv() => { Err(Error::EarlyTerminationError) }
        }
    }

    async fn handle_response(
        response: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<Bytes, Error> {
        match response {
            Err(e) => {
                if e.is_status() {
                    if let Some(status_code) = e.status() {
                        match status_code.as_u16() {
                            429 => {
                                // breadcrumbs? https://github.com/rust-lang-nursery/rust-cookbook/pull/395/files
                                debug!("wait and retry on 429 not implemented, skipping...");
                            }
                            _ => {
                                debug!("unexpected status code: {}", status_code);
                            }
                        }
                    } else {
                        debug!("expected error with status code, but found none: {}", e);
                    }
                } else {
                    debug!("unexpected error: {}", e);
                }
                Err(Error::RequestError { why: e })
            }
            Ok(res) => {
                let r = res.bytes().await;
                match r {
                    Err(e) => {
                        debug!("error reading request response: {}", e);
                        Err(Error::RequestError { why: e })
                    }
                    Ok(ress) => Ok(ress),
                }
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
            (Self::Web, Self::Web) => true,
            (Self::Local, Self::Local) => true,
            _ => false,
        }
    }
}
