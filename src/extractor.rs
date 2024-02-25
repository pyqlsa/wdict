use crate::error::Error;
use crate::filter::FilterMode;

use scraper::{node::Node, Html};
use std::collections::HashMap;

/// Crawls websites, gathering links and words from pages.
pub struct Extractor {
    opts: ExtractOptions,
    words: HashMap<String, bool>,
}

impl Extractor {
    /// Returns a new Crawler instance with the default `reqwest::Client`.
    pub fn new(
        min_word_length: usize,
        filters: Vec<FilterMode>,
        include_js: bool,
        include_css: bool,
    ) -> Result<Self, Error> {
        let extractor = Self {
            opts: ExtractOptions::new(min_word_length, filters, include_js, include_css),
            words: HashMap::new(),
        };
        Ok(extractor)
    }

    /// Returns the gathered words.
    pub fn words(&self) -> HashMap<String, bool> {
        self.words.clone()
    }

    /// Extract words from an html document.
    pub fn words_from_doc(&mut self, document: &Html) -> () {
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

/// Options used when building wordlists.
#[derive(Debug, Clone)]
struct ExtractOptions {
    /// Only save words greater than or equal to this value.
    min_word_length: usize,
    /// Filter strategy for words; multiple can be specified.
    filters: Vec<FilterMode>,
    /// Include javascript from html pages.
    include_js: bool,
    /// Include css from html pages.
    include_css: bool,
}

impl ExtractOptions {
    /// Returns a new CrawlOptions instance.
    fn new(
        min_word_length: usize,
        filters: Vec<FilterMode>,
        include_js: bool,
        include_css: bool,
    ) -> Self {
        Self {
            min_word_length,
            filters,
            include_js,
            include_css,
        }
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
}
