use bytes::{Bytes, BytesMut};
use infer;
use log::debug;
use scraper::{node::Node, Html};
use unicode_segmentation::UnicodeSegmentation;

use crate::collections::WordDb;

use super::FilterMode;

/// Extracts words from html documents.
#[derive(Debug, Clone)]
pub struct Extractor {
    opts: ExtractOptions,
    words: WordDb,
}

impl Extractor {
    /// Returns a new Extractor instance.
    pub fn new(opts: ExtractOptions, words: WordDb) -> Self {
        Self { opts, words }
    }

    pub fn words_from_doc(&mut self, buf: &Bytes) -> () {
        let res = infer::get(buf);
        match res {
            Some(kind) => match kind.mime_type() {
                "text/html" => self.words_from_html(&buf),
                _ => debug!("unsupported mime type: {}", kind),
            },
            None => {
                //warn!("failure infering mime type");
                // attempt as plain text file
                self.words_from_text(&buf);
            }
        }
    }

    /// Extract words from the provided document, treating bytes buffer as html.
    fn words_from_html(&mut self, buf: &Bytes) -> () {
        let s = String::from_utf8_lossy(&buf).to_string();
        //let mut fin: String = String::new();
        let mut fin = BytesMut::new();
        {
            // scope protects the html object from async
            let doc = Html::parse_document(&s);
            // note: alternatives to getting all text nodes (regardless if script/styel/etc. or not)
            //for text in document.clone().root_element().text() { ...do something... }
            for d in doc.root_element().descendants() {
                if let Node::Text(text) = d.value() {
                    //let parent_tag = d.parent().unwrap().value().as_element().unwrap().name();
                    if let Some(p) = d.parent() {
                        if let Some(e) = p.value().as_element() {
                            match e.name().to_lowercase().as_str() {
                                // if parent node is a script tag, means it should be js
                                "script" => {
                                    if self.opts.include_js() {
                                        fin.extend(text.text.clone().into_bytes().iter());
                                    }
                                }
                                // if parent node is a style tag, means it should be css
                                "style" => {
                                    if self.opts.include_css() {
                                        fin.extend(text.text.clone().into_bytes().iter());
                                    }
                                }
                                // if not otherwise ignorable, send it
                                _ => {
                                    fin.extend(text.text.clone().into_bytes().iter());
                                }
                            }
                        }
                    }
                }
            }
        }
        self.filter_text(&fin.freeze());
    }

    /// Extract words from the provided document, treating bytes buffer as plain text.
    fn words_from_text(&mut self, buf: &Bytes) -> () {
        //let s = String::from_utf8_lossy(&buf).to_string();
        self.filter_text(&buf);
    }

    /// Filter text based on configured filters and capture resulting words.
    fn filter_text(&mut self, buf: &Bytes) -> () {
        for w in String::from_utf8_lossy(&buf).unicode_words() {
            if w.len() < self.opts.min_word_length() || w.len() > self.opts.max_word_length() {
                continue;
            }

            let mut fintext = w.to_string();
            for filter in self.opts.filters() {
                filter.filter_str(&mut fintext);
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
