use crate::error::Error;
use crate::filter::FilterMode;
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
            min_word_length,
            req_per_sec,
            filters,
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
        min_word_length: usize,
        req_per_sec: u64,
        filters: Vec<FilterMode>,
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
            opts: CrawlOptions::new(
                url.clone(),
                depth,
                min_word_length,
                filters,
                include_js,
                include_css,
                site,
            ),
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
                .matches_policy(self.opts.url(), result.clone().unwrap())
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
            self.links_from_doc(&result.unwrap(), &document);
            self.words_from_doc(&document);
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

    /// Extract words from an html document.
    fn words_from_doc(&mut self, document: &Html) -> () {
        // note: alternatives to getting all text nodes (regardless if script/styel/etc. or not)
        //for text in document.clone().root_element().text() { ...do something... }
        for d in document.clone().root_element().descendants() {
            if let Node::Text(text) = d.value() {
                let parent_tag = d
                    .parent()
                    .unwrap()
                    .value()
                    .as_element()
                    .unwrap()
                    .name()
                    .to_lowercase();
                match parent_tag.as_str() {
                    // if parent node is a script tag, means it should be js
                    "script" => {
                        if self.opts.include_js() {
                            self.filter_text(&text.text);
                        }
                    }
                    // if parent node is a style tag, means it should be css
                    "style" => {
                        if self.opts.include_css() {
                            self.filter_text(&text.text);
                        }
                    }
                    // if not ignored, send it
                    _ => self.filter_text(&text.text),
                }
            }
        }
    }

    fn filter_text(&mut self, text: &str) -> () {
        let mut tmp = text.to_string();
        tmp = tmp.to_lowercase();
        // ignore these characters since we're looking for words
        tmp = tmp.replace(|c: char| !c.is_alphanumeric(), " ");
        if tmp.len() > 0 {
            for w in tmp.split_whitespace() {
                let mut fintext = w.to_string();
                for filter in self.opts.filters() {
                    fintext = filter.filter_str(&fintext);
                }
                if fintext.len() >= self.opts.min_word_length() {
                    self.words.entry(String::from(fintext)).or_insert(true);
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
    /// Include javascript from html pages.
    include_js: bool,
    /// Include css from html pages.
    include_css: bool,
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
        include_js: bool,
        include_css: bool,
        site: SitePolicy,
    ) -> Self {
        Self {
            url,
            depth,
            min_word_length,
            filters,
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

    /// Returns the minimum word length for saving words to the wordslist.
    fn min_word_length(&self) -> usize {
        self.min_word_length
    }

    /// Returns the configured filter mode for discovered words.
    fn filters(&self) -> Vec<FilterMode> {
        self.filters.clone()
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
