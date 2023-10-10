//! Simple cli tool to extract words from webpages to build a dictionary.
//!
use clap::{Args, Parser, ValueEnum};
use reqwest::Url;
use std::fs::File;
use std::io::Write;

use wdict::{Crawler, Error, FilterMode, SitePolicy};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// URL to start crawling from.
    #[command(flatten)]
    url: Site,
    /// Limit the depth of crawling links.
    #[arg(short, long, default_value_t = 1)]
    depth: usize,
    /// Only save words greater than or equal to this value.
    #[arg(short, long, default_value_t = 3)]
    min_word_length: usize,
    /// Number of requests to make per second.
    #[arg(short, long, default_value_t = 20)]
    req_per_sec: u64,
    /// File to write dictionary to (will be overwritten if it already exists).
    #[arg(short, long, default_value = "wdict.txt")]
    file: String,
    /// Filter strategy for words; multiple can be specified.
    #[arg(
        long,
        default_value = "none",
        value_enum,
        num_args = 1..,
        value_delimiter = ',',
    )]
    filters: Vec<FilterArg>,
    /// Include javascript from <script> tags and links.
    #[arg(short = 'j', long, default_value_t = false)]
    inclue_js: bool,
    /// Include CSS from <style> tags links.
    #[arg(short = 'c', long, default_value_t = false)]
    inclue_css: bool,
    /// Site policy for discovered links.
    #[arg(long, default_value = "same", value_enum)]
    site: SitePolicyArg,
}

#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
struct Site {
    /// URL to start crawling from.
    #[arg(short, long)]
    url: Option<String>,
    /// Pre-canned theme URLs to start crawling from (for fun, demoing features, and sparking new
    /// ideas).
    #[arg(long, value_enum)]
    theme: Theme,
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

#[derive(ValueEnum, Copy, Debug, Clone)]
enum Theme {
    /// Star Wars themed URL <https://www.starwars.com/databank>.
    StarWars,
    /// Tolkien themed URL <https://www.quicksilver899.com/Tolkien/Tolkien_Dictionary.html>.
    Tolkien,
    /// Witcher themed URL <https://witcher.fandom.com/wiki/Elder_Speech>.
    Witcher,
}

impl Theme {
    /// Get url string for the theme.
    fn as_str(&self) -> &str {
        match self {
            Self::StarWars => starwars_url!(),
            Self::Tolkien => tolkien_url!(),
            Self::Witcher => witcher_url!(),
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
    /// Allow crawling links, only if the domain exactly matches.
    Same,
    /// Allow crawling links if they are the same domain or subdomains.
    Subdomain,
    /// Allow crawling all links, regardless of domain.
    All,
}

impl SitePolicyArg {
    /// Get site policy from arg; exists just to de-couple lib from clap.
    fn to_mode(&self) -> SitePolicy {
        match self {
            Self::Same => SitePolicy::Same,
            Self::Subdomain => SitePolicy::Subdomain,
            Self::All => SitePolicy::All,
        }
    }
}

/// Main function.
#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Cli::parse();
    let site = &args.url;
    let url = if let Some(url_str) = site.url.as_deref() {
        Url::parse(url_str)?
    } else {
        Url::parse(site.theme.as_str())?
    };
    let mut crawler = Crawler::new(
        url,
        args.depth,
        args.min_word_length,
        args.req_per_sec,
        to_modes(args.filters),
        args.inclue_js,
        args.inclue_css,
        args.site.to_mode(),
    )?;

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
