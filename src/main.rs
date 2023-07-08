//! Simple cli tool to extract words from webpages to build a dictionary.
//!
use clap::{Parser, ValueEnum};
use reqwest::Url;
use std::fs::File;
use std::io::Write;

use wdict::{Crawler, FilterMode, SitePolicy};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URL to start crawling from.
    #[arg(
        short,
        long,
        default_value = "https://www.quicksilver899.com/Tolkien/Tolkien_Dictionary.html"
    )]
    url: String,
    /// Limit the depth of crawling links.
    #[arg(short, long, default_value_t = 1)]
    depth: usize,
    /// Only save words greater than or equal to this value.
    #[arg(short, long, default_value_t = 3)]
    min_word_length: usize,
    /// File to write dictionary to (will be overwritten if it already exists).
    #[arg(short, long, default_value = "wdict.txt")]
    file: String,
    /// Filter strategy for words.
    #[arg(long, default_value = "none", value_enum)]
    filter: FilterArg,
    /// Site policy for discovered links.
    #[arg(long, default_value = "same", value_enum)]
    site: SitePolicyArg,
}

#[derive(ValueEnum, Copy, Debug, Clone)]
enum FilterArg {
    /// Transform unicode according to https://github.com/kornelski/deunicode
    Deunicode,
    /// Leave the string as-is
    None,
}

impl FilterArg {
    /// Get filter mode from arg; exists just to de-couple lib from clap
    fn to_mode(&self) -> FilterMode {
        match self {
            FilterArg::Deunicode => FilterMode::Deunicode,
            FilterArg::None => FilterMode::None,
        }
    }
}

/// Defines options for crawling sites.
#[derive(ValueEnum, Copy, Debug, Clone)]
enum SitePolicyArg {
    /// Allow crawling links, only if the domain exactly matches
    Same,
    /// Allow crawling links if they are the same domain or subdomains
    Subdomain,
    /// Allow crawling all links, regardless of domain
    All,
}

impl SitePolicyArg {
    /// Get site policy from arg; exists just to de-couple lib from clap
    fn to_mode(&self) -> SitePolicy {
        match self {
            SitePolicyArg::Same => SitePolicy::Same,
            SitePolicyArg::Subdomain => SitePolicy::Subdomain,
            SitePolicyArg::All => SitePolicy::All,
        }
    }
}

/// Main function.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let u = Url::parse(&args.url)?;
    let mut crawler = Crawler::new(
        u,
        args.depth,
        args.min_word_length,
        args.filter.to_mode(),
        args.site.to_mode(),
    );

    crawler.crawl().await;

    let len_links = crawler.links().len();
    println!("visited links:");
    crawler.links().into_iter().for_each(|(k, v)| {
        if v {
            println!("- {}", k)
        }
    });
    println!("links discovered but not visited:");
    crawler.links().into_iter().for_each(|(k, v)| {
        if !v {
            println!("- {}", k)
        }
    });
    println!("total unique links discovered: {}", len_links);
    println!();

    let len_words = crawler.words().len();
    println!("unique words: {}", len_words);

    println!("writing dictionary to file: {}", args.file);
    let mut file = File::create(args.file).expect("Error creating dictionary file");
    crawler.words().into_iter().for_each(|(k, _v)| {
        let line = format!("{}\n", k);
        file.write_all(line.as_bytes())
            .expect("Error writing to dictionary");
    });
    Ok(())
}
