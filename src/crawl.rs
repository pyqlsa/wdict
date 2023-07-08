use crate::filter::FilterMode;

use async_recursion::async_recursion;
use reqwest::{Client, Url};
use scraper::{node::Node, Html, Selector};
use std::collections::HashMap;

/// Crawls websites, gathering links and words from pages
pub struct Crawler {
    opts: CrawlOptions,
    links: HashMap<String, bool>,
    words: HashMap<String, bool>,
    cur_depth: usize,
}

impl Crawler {
    /// Returns a new Crawler instance
    pub fn new(
        url: Url,
        depth: usize,
        min_word_length: usize,
        filter: FilterMode,
        site: SitePolicy,
    ) -> Crawler {
        let mut c = Crawler {
            opts: CrawlOptions::new(url.clone(), depth, min_word_length, filter, site),
            links: HashMap::new(),
            words: HashMap::new(),
            cur_depth: 0,
        };
        c.links.insert(String::from(url), false);
        c
    }

    /// Returns the crawled links
    pub fn links(&self) -> HashMap<String, bool> {
        self.links.clone()
    }

    /// Returns the gathered words
    pub fn words(&self) -> HashMap<String, bool> {
        self.words.clone()
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
            self.links.insert(url.to_owned(), true);
            let document = Self::doc_from_url(String::from(url)).await;
            self.links_from_doc(&document);
            self.words_from_doc(&document);
        }
        if self.cur_depth < self.opts.depth() {
            println!("going deeper... current depth {}", self.cur_depth);
            self.crawl().await;
        }
    }

    /// Get an html document from the provided url.
    async fn doc_from_url(url: String) -> Html {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        let response = client.get(url).send().await;
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
                let fintext = text.text.trim();
                let fintext = self.opts.filter().filter_str(fintext);
                let fintext = fintext.to_lowercase();
                // ignore these characters since we're looking for words
                let fintext = fintext.replace(|c: char| !c.is_alphanumeric(), " ");
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
    /// Filter strategy for words.
    filter: FilterMode,
    /// Strategy for link crawling.
    site: SitePolicy,
}

impl CrawlOptions {
    /// Returns a new CrawlOptions instance
    fn new(
        url: Url,
        depth: usize,
        min_word_length: usize,
        filter: FilterMode,
        site: SitePolicy,
    ) -> CrawlOptions {
        CrawlOptions {
            url,
            depth,
            min_word_length,
            filter,
            site,
        }
    }

    /// Returns the url where crawling initiated from
    fn url(&self) -> Url {
        self.url.clone()
    }

    /// Returns the link search depth used for crawling
    fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the minimum word length for saving words to the wordslist
    fn min_word_length(&self) -> usize {
        self.min_word_length
    }

    /// Returns the configured filter mode for discovered words
    fn filter(&self) -> FilterMode {
        self.filter
    }

    /// Returns the configured site policy for visiting discovered URLs
    fn site(&self) -> SitePolicy {
        self.site
    }
}

/// Defines options for crawling sites.
#[derive(Copy, Debug, Clone)]
pub enum SitePolicy {
    /// Allow crawling links, only if the domain exactly matches
    Same,
    /// Allow crawling links if they are the same domain or subdomains
    Subdomain,
    /// Allow crawling all links, regardless of domain
    All,
}

/// Display implementation
impl std::fmt::Display for SitePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SitePolicy::Same => write!(f, "Same"),
            SitePolicy::Subdomain => write!(f, "Subdomain"),
            SitePolicy::All => write!(f, "All"),
        }
    }
}

impl SitePolicy {
    /// Returns if the given url matches the site visiting policy.
    pub fn matches_policy(&self, source_url: Url, target_url: Url) -> bool {
        if target_url.host_str() == None {
            return false;
        }
        match self {
            SitePolicy::Same => {
                if target_url.host_str().unwrap_or("fail.___")
                    == source_url.host_str().unwrap_or("nope.___")
                {
                    return true;
                }
                return false;
            }
            SitePolicy::Subdomain => {
                let tu = target_url.host_str().unwrap_or("fail.___");
                let su = source_url.host_str().unwrap_or("nope.___");

                if tu == su || tu.ends_with(format!(".{}", su).as_str()) {
                    return true;
                }
                return false;
            }
            SitePolicy::All => return true,
        }
    }
}
