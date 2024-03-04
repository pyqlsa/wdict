//! Simple cli tool to extract words from webpages to build a dictionary.
//!
use clap::{Args, Parser, ValueEnum};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::io::Write;
use tokio::signal;
use tokio::sync::broadcast;

use wdict::{CrawlOptions, Crawler, Error, Extractor, FilterMode, Shutdown, SitePolicy};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// URL to start crawling from.
    #[command(flatten)]
    site: Site,
    /// Limit the depth of crawling urls.
    #[arg(short, long, default_value_t = 1)]
    depth: usize,
    /// Only save words greater than or equal to this value.
    #[arg(short, long, default_value_t = 3)]
    min_word_length: usize,
    /// Number of requests to make per second.
    #[arg(short, long, default_value_t = 20)]
    req_per_sec: u64,
    /// File to write dictionary to (will be overwritten if it already exists).
    #[arg(short, long, default_value = "wdict.txt", value_parser = clap::builder::ValueParser::new(str_not_whitespace))]
    output: String,
    /// Write discovered urls to a file.
    #[arg(long, default_value_t = false)]
    output_urls: bool,
    /// File to write urls to, json formatted (will be overwritten if it already exists).
    #[arg(long, default_value = "urls.json", value_parser = clap::builder::ValueParser::new(str_not_whitespace))]
    output_urls_file: String,
    /// Filter strategy for words; multiple can be specified (comma separated).
    #[arg(
        long,
        default_value = "none",
        value_enum,
        num_args = 1..,
        value_delimiter = ',',
    )]
    filters: Vec<FilterArg>,
    /// Include javascript from <script> tags and urls.
    #[arg(short = 'j', long, default_value_t = false)]
    inclue_js: bool,
    /// Include CSS from <style> tags urls.
    #[arg(short = 'c', long, default_value_t = false)]
    inclue_css: bool,
    /// Site policy for discovered urls.
    #[arg(long, default_value = "same", value_enum)]
    site_policy: SitePolicyArg,
}

#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
struct Site {
    /// URL to start crawling from.
    #[arg(short, long, value_parser = clap::builder::ValueParser::new(str_not_whitespace))]
    url: Option<String>,
    /// Pre-canned theme URLs to start crawling from (for fun, demoing features, and sparking new
    /// ideas).
    #[arg(long, value_enum)]
    theme: Option<Theme>,
}

fn str_not_whitespace(value: &str) -> Result<String, Error> {
    if value.len() < 1 || value.trim().len() != value.len() {
        Err(Error::StrWhitespaceError)
    } else {
        Ok(value.to_string())
    }
}

// Need to wait on https://github.com/clap-rs/clap/issues/2639 before using macros in doc comments
// so they show up in help; leaving breadcrumbs here in hopes it will eventually come to fruition.
//
// #[doc = concat!("description <", some_url!(),">")]
macro_rules! starwars_url {
    () => {
        "https://www.starwars.com/databank"
    };
}
macro_rules! tolkien_url {
    () => {
        "https://www.quicksilver899.com/Tolkien/Tolkien_Dictionary.html"
    };
}
macro_rules! witcher_url {
    () => {
        "https://witcher.fandom.com/wiki/Elder_Speech"
    };
}
macro_rules! pokemon_url {
    () => {
        "https://www.smogon.com"
    };
}
macro_rules! bebop_url {
    () => {
        "https://cowboybebop.fandom.com/wiki/Cowboy_Bebop"
    };
}
macro_rules! greek_url {
    () => {
        "https://www.theoi.com"
    };
}
macro_rules! greco_roman_url {
    () => {
        "https://www.gutenberg.org/files/22381/22381-h/22381-h.htm"
    };
}
macro_rules! lovecraft_url {
    () => {
        "https://www.hplovecraft.com"
    };
}

#[derive(ValueEnum, Copy, Debug, Clone)]
enum Theme {
    /// Star Wars themed URL <https://www.starwars.com/databank>.
    StarWars,
    /// Tolkien themed URL <https://www.quicksilver899.com/Tolkien/Tolkien_Dictionary.html>.
    Tolkien,
    /// Witcher themed URL <https://witcher.fandom.com/wiki/Elder_Speech>.
    Witcher,
    /// Pokemon themed URL <https://www.smogon.com>.
    Pokemon,
    /// Cowboy Bebop themed URL <https://cowboybebop.fandom.com/wiki/Cowboy_Bebop>.
    Bebop,
    /// Greek Mythology themed URL <https://www.theoi.com>.
    Greek,
    /// Greek and Roman Mythology themed URL <https://www.gutenberg.org/files/22381/22381-h/22381-h.htm>.
    GrecoRoman,
    /// H.P. Lovecraft themed URL <https://www.hplovecraft.com>.
    Lovecraft,
}

impl Theme {
    /// Get url string for the theme.
    fn as_str(&self) -> &str {
        match self {
            Self::StarWars => starwars_url!(),
            Self::Tolkien => tolkien_url!(),
            Self::Witcher => witcher_url!(),
            Self::Pokemon => pokemon_url!(),
            Self::Bebop => bebop_url!(),
            Self::Greek => greek_url!(),
            Self::GrecoRoman => greco_roman_url!(),
            Self::Lovecraft => lovecraft_url!(),
        }
    }
}

#[derive(ValueEnum, Copy, Debug, Clone)]
enum FilterArg {
    /// Transform unicode according to <https://github.com/kornelski/deunicode>.
    Deunicode,
    /// Transform unicode according to <https://github.com/null8626/decancer>.
    Decancer,
    /// Ignore words that consist of all numbers.
    AllNumbers,
    /// Ignore words that contain any number.
    AnyNumbers,
    /// Ignore words that contain no numbers.
    NoNumbers,
    /// Keep only words that exclusively contain numbers.
    OnlyNumbers,
    /// Ignore words that consist of all ascii characters.
    AllAscii,
    /// Ignore words that contain any ascii character.
    AnyAscii,
    /// Ignore words that contain no ascii characters.
    NoAscii,
    /// Keep only words that exclusively contain ascii characters.
    OnlyAscii,
    /// Leave the word as-is.
    None,
}

impl FilterArg {
    /// Get filter mode from arg; exists just to de-couple lib from clap.
    fn to_mode(&self) -> FilterMode {
        match self {
            Self::Deunicode => FilterMode::Deunicode,
            Self::Decancer => FilterMode::Decancer,
            Self::AllNumbers => FilterMode::AllNumbers,
            Self::AnyNumbers => FilterMode::AnyNumbers,
            Self::NoNumbers => FilterMode::NoNumbers,
            Self::OnlyNumbers => FilterMode::OnlyNumbers,
            Self::AllAscii => FilterMode::AllAscii,
            Self::AnyAscii => FilterMode::AnyAscii,
            Self::NoAscii => FilterMode::NoAscii,
            Self::OnlyAscii => FilterMode::OnlyAscii,
            Self::None => FilterMode::None,
        }
    }
}

/// Convert a Vector of FilterArg to a Vector of FilterMode.
fn to_modes(v: Vec<FilterArg>) -> Vec<FilterMode> {
    v.clone().into_iter().map(|f| f.to_mode()).collect()
}

/// Defines options for crawling sites.
#[derive(ValueEnum, Copy, Debug, Clone)]
enum SitePolicyArg {
    /// Allow crawling urls, only if the domain exactly matches.
    Same,
    /// Allow crawling urls if they are the same domain or subdomains.
    Subdomain,
    /// Allow crawling urls if they are the same domain or a sibling.
    Sibling,
    /// Allow crawling all urls, regardless of domain.
    All,
}

impl SitePolicyArg {
    /// Get site policy from arg; exists just to de-couple lib from clap.
    fn to_mode(&self) -> SitePolicy {
        match self {
            Self::Same => SitePolicy::Same,
            Self::Subdomain => SitePolicy::Subdomain,
            Self::Sibling => SitePolicy::Sibling,
            Self::All => SitePolicy::All,
        }
    }
}

/// Helper for json output url file.
#[derive(Serialize, Deserialize, Debug)]
struct OutputUrls {
    visited: Vec<String>,
    not_visited: Vec<String>,
}

/// Main function.
#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Cli::parse();
    let site = &args.site;
    let url = if let Some(url_str) = site.url.as_deref() {
        Url::parse(url_str)?
    } else {
        if let Some(t) = site.theme {
            Url::parse(t.as_str())?
        } else {
            Url::parse("")?
        }
    };

    let (notify_shutdown, _) = broadcast::channel(1);
    let copts = CrawlOptions::new(
        url,
        args.depth,
        args.req_per_sec,
        args.inclue_js,
        args.inclue_css,
        args.site_policy.to_mode(),
    );

    let mut crawler = Crawler::new(copts, Shutdown::new(notify_shutdown.subscribe()))?;

    tokio::select! {
        res = crawler.crawl() => {
            if let Err(err) = res {
                eprintln!("unexpected error while crawling {}", err);
            }
        }
        _ = signal::ctrl_c() => {
            println!("shutting down...");
        }
    }
    // When `notify_shutdown` is dropped, all tasks which have `subscribe`d will
    // receive the shutdown signal and can exit
    drop(notify_shutdown);

    let mut extractor = Extractor::new(
        args.min_word_length,
        to_modes(args.filters),
        args.inclue_js,
        args.inclue_css,
    )?;

    crawler.docs().iter().for_each(|doc| {
        extractor.words_from_doc(&doc);
    });

    let len_words = extractor.words().len();
    println!("unique words: {}", len_words);
    println!(
        "visited pages: {}",
        crawler.urls().values().filter(|v| **v).count()
    );

    let mut file = File::create(args.output.clone()).expect("Error creating dictionary file");
    extractor.words().iter().for_each(|(word, _v)| {
        let line = format!("{}\n", word);
        file.write_all(line.as_bytes())
            .expect("Error writing to dictionary");
    });
    println!("dictionary written to: {}", args.output);

    if args.output_urls {
        let out_urls = OutputUrls {
            visited: crawler.visited_urls(),
            not_visited: crawler.not_visited_urls(),
        };
        let url_file = args.output_urls_file;
        if let Ok(j) = serde_json::to_string_pretty(&out_urls) {
            let mut file = File::create(url_file.clone()).expect("Error creating urls file");
            file.write_all(j.as_bytes())
                .expect("Error writing urls to file");
            println!("urls written to file: {}", url_file);
        } else {
            eprintln!("Error serializing output urls json")
        }
    }
    println!();

    Ok(())
}
