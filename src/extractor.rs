use crate::doc_queue::DocQueue;
use crate::error::Error;
use crate::filter::FilterMode;
use crate::helpers;
use crate::worddb::WordDb;

use scraper::{node::Node, Html};
use tokio::time::{sleep, Duration};

/// Extracts words from html documents.
pub struct Extractor {
    opts: ExtractOptions,
    docs: DocQueue,
    words: WordDb,
}

impl Extractor {
    /// Returns a new Extractor instance.
    pub fn new(opts: ExtractOptions, docs: DocQueue, words: WordDb) -> Self {
        Self { opts, docs, words }
    }

    /// Parse documents from the provided queue and extract words.
    pub async fn parse_from_queue(&mut self) -> Result<(), Error> {
        sleep(Duration::from_millis(500)).await;
        loop {
            sleep(Duration::from_millis(u64::from(helpers::num_between(
                10, 30,
            ))))
            .await;
            if self.docs.is_empty() {
                continue;
            }
            if let Some(doc) = self.docs.pop() {
                self.words_from_doc(&doc);
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Extract words from the provided document.
    pub fn words_from_doc(&mut self, document: &String) -> () {
        let doc = Html::parse_document(&document);
        // note: alternatives to getting all text nodes (regardless if script/styel/etc. or not)
        //for text in document.clone().root_element().text() { ...do something... }
        for d in doc.root_element().descendants() {
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

    /// Filter text based on configured filters and capture resulting words.
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
                    self.words.insert(fintext);
                }
            }
        }
    }
}

/// Options used when building wordlists.
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    /// Only save words greater than or equal to this value.
    min_word_length: usize,
    /// Include javascript from html pages.
    include_js: bool,
    /// Include css from html pages.
    include_css: bool,
    /// Filter strategy for words; multiple can be specified.
    filters: Vec<FilterMode>,
}

impl ExtractOptions {
    /// Returns a new ExtractOptions instance.
    pub fn new(
        min_word_length: usize,
        include_js: bool,
        include_css: bool,
        filters: Vec<FilterMode>,
    ) -> Self {
        Self {
            min_word_length,
            include_js,
            include_css,
            filters,
        }
    }

    /// Returns the minimum word length for saving words to the wordslist.
    pub fn min_word_length(&self) -> usize {
        self.min_word_length
    }

    /// Returns whether or not configuration dictates  to include js.
    pub fn include_js(&self) -> bool {
        self.include_js
    }

    /// Returns whether or not configuration dictates  to include css.
    pub fn include_css(&self) -> bool {
        self.include_css
    }

    /// Returns the configured filter mode for discovered words.
    pub fn filters(&self) -> impl Iterator<Item = &FilterMode> {
        self.filters.iter()
    }
}
