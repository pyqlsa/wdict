use bytes::Bytes;
use infer;
use scraper::{node::Node, Html};
use unicode_segmentation::UnicodeSegmentation;

use crate::collections::{DocQueue, WordDb};

use super::FilterMode;

/// Extracts words from html documents.
#[derive(Debug, Clone)]
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

    /// Parse documents from the internal queue and extract words.
    pub async fn process_doc_from_queue(&mut self) -> () {
        //sleep(Duration::from_millis(u64::from(utils::num_between(10, 30)))).await;
        if self.docs.is_empty() {
            return;
        }
        if let Some(buf) = self.docs.pop() {
            self.words_from_doc(&buf);
        }
    }

    fn words_from_doc(&mut self, buf: &Bytes) -> () {
        let res = infer::get(buf);
        match res {
            Some(kind) => match kind.mime_type() {
                "text/html" => self.words_from_html(&buf),
                _ => eprintln!("unsupported mime type: {}", kind),
            },
            None => {
                //eprintln!("failure infering mime type");
                // attempt as plain text file
                self.words_from_text(&buf);
            }
        }
    }

    /// Extract words from the provided document, treating bytes buffer as html.
    fn words_from_html(&mut self, buf: &Bytes) -> () {
        let s = String::from_utf8_lossy(&buf).to_string();
        let doc = Html::parse_document(&s);
        let mut s: String = String::new();
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
                            s.push_str(&text.text);
                        }
                    }
                    // if parent node is a style tag, means it should be css
                    "style" => {
                        if self.opts.include_css() {
                            s.push_str(&text.text);
                        }
                    }
                    // if not ignored, send it
                    _ => {
                        s.push_str(&text.text);
                    }
                }
            }
        }
        self.filter_text(&s);
    }

    /// Extract words from the provided document, treating bytes buffer as plain text.
    fn words_from_text(&mut self, buf: &Bytes) -> () {
        let s = String::from_utf8_lossy(&buf).to_string();
        self.filter_text(&s);
    }

    /// Filter text based on configured filters and capture resulting words.
    fn filter_text(&mut self, text: &str) -> () {
        for w in text.unicode_words() {
            let mut fintext = w.to_lowercase();
            for filter in self.opts.filters() {
                filter.filter_str(&mut fintext);
            }
            if fintext.len() < self.opts.min_word_length()
                || fintext.len() > self.opts.max_word_length()
            {
                continue;
            }
            self.words.insert(fintext);
        }
    }
}

/// Options used when building wordlists.
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    /// Only save words greater than or equal to this value.
    min_word_length: usize,
    /// Only save words less than or equal to this value.
    max_word_length: usize,
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
        max_word_length: usize,
        include_js: bool,
        include_css: bool,
        filters: Vec<FilterMode>,
    ) -> Self {
        Self {
            min_word_length,
            max_word_length,
            include_js,
            include_css,
            filters,
        }
    }

    /// Returns the minimum word length for saving words to the wordslist.
    pub fn min_word_length(&self) -> usize {
        self.min_word_length
    }

    /// Returns the maximum word length for saving words to the wordslist.
    pub fn max_word_length(&self) -> usize {
        self.max_word_length
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
