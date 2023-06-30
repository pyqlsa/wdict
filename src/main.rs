//! Simple cli tool to extract words from webpages to build a dictionary.
//!
use async_recursion::async_recursion;
use clap::{Parser, ValueEnum};
use deunicode::deunicode;
use reqwest::{Client, Url};
use scraper::{node::Node, Html, Selector};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

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
    /// Munge strategy for words.
    #[arg(long, default_value = "none", value_enum)]
    munge: MungeMode,
    /// Site policy for discovered links.
    #[arg(long, default_value = "same", value_enum)]
    site: SitePolicy,
}

/// Defines a way to munge strings when building wordlists.
///
/// TODO: more options with things like:
/// - https://github.com/unicode-rs/unicode-normalization
/// - https://github.com/unicode-rs/unicode-security
/// - https://github.com/null8626/decancer
#[derive(ValueEnum, Copy, Debug, Clone)]
enum MungeMode {
    /// Munge unicode according to https://github.com/kornelski/deunicode
    Deunicode,
    /// Leave the string as-is
    None,
}

/// Defines options for crawling sites.
#[derive(ValueEnum, Copy, Debug, Clone)]
enum SitePolicy {
    /// Allow crawling links, only if the domain exactly matches
    Same,
    /// Allow crawling links if they are the same domain or subdomains
    Subdomain,
    /// Allow crawling all links, regardless of domain
    All,
}

impl std::fmt::Display for SitePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SitePolicy::Same => write!(f, "Same"),
            SitePolicy::Subdomain => write!(f, "Subdomain"),
            SitePolicy::All => write!(f, "All"),
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
    /// Munge strategy for words.
    munge: MungeMode,
    /// Strategy for link crawling.
    site: SitePolicy,
}

/// Main function.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let u = Url::parse(&args.url)?;
    let opts = CrawlOptions {
        url: u,
        depth: args.depth,
        min_word_length: args.min_word_length,
        munge: args.munge,
        site: args.site,
    };

    let cur_depth = 0;
    let mut links = HashMap::new();
    let mut words = HashMap::new();
    links.insert(String::from(args.url), false);

    crawl(&opts, &mut links, &mut words, cur_depth).await;

    let len_links = links.len();
    println!("visited links:");
    links.clone().into_iter().for_each(|(k, v)| {
        if v {
            println!("- {}", k)
        }
    });
    println!("links discovered but not visited:");
    links.clone().into_iter().for_each(|(k, v)| {
        if !v {
            println!("- {}", k)
        }
    });
    println!("total unique links discovered: {}", len_links);
    println!();

    let len_words = words.len();
    println!("unique words: {}", len_words);

    println!("writing dictionary to file: {}", args.file);
    let mut file = File::create(args.file).expect("Error creating dictionary file");
    words.into_iter().for_each(|(k, _v)| {
        let line = format!("{}\n", k);
        file.write_all(line.as_bytes())
            .expect("Error writing to dictionary");
    });
    Ok(())
}

/// Crawl links up to a given limit, scraping links and words from pages.
#[async_recursion(?Send)]
async fn crawl(
    opts: &CrawlOptions,
    links: &mut HashMap<String, bool>,
    words: &mut HashMap<String, bool>,
    cur_depth: usize,
) -> () {
    let deeper = cur_depth + 1;
    for (url, visited) in links.clone().into_iter() {
        let result = Url::parse(url.as_str());
        match result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("not a url: {}", e);
                continue;
            }
        }
        if !matches_site_policy(result.unwrap(), opts.clone()) {
            println!(
                "site policy '{}' violated for url: '{}', skipping...",
                opts.site, url
            );
            continue;
        }

        if visited {
            println!("already visited '{}', skipping", url);
            continue;
        }

        println!("visiting {}", url);
        links.insert(url.to_owned(), true);
        let document = doc_from_url(String::from(url)).await;
        links_from_doc(links, &document);
        words_from_doc(opts, words, &document);
    }
    if deeper < opts.depth {
        println!("going deeper... current depth {}", deeper);
        crawl(opts, links, words, deeper).await;
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
fn links_from_doc(links: &mut HashMap<String, bool>, document: &Html) -> () {
    let link_selector = Selector::parse(r#"a[href^="http"]"#).unwrap();

    for elem in document.clone().select(&link_selector) {
        links
            .entry(String::from(elem.value().attr("href").unwrap()))
            .or_insert(false);
    }
}

/// Extract words from an html document.
fn words_from_doc(opts: &CrawlOptions, words: &mut HashMap<String, bool>, document: &Html) -> () {
    for node in document.clone().tree {
        if let Node::Text(text) = node {
            let fintext = text.text.trim();
            let fintext = munge_str(fintext, opts.munge);
            let fintext = fintext.to_lowercase();
            // ignore these characters since we're looking for words
            let fintext = fintext.replace(|c: char| !c.is_alphanumeric(), " ");
            if fintext.len() > 0 {
                for w in fintext.split_whitespace() {
                    if w.len() >= opts.min_word_length {
                        words.entry(String::from(w)).or_insert(true);
                    }
                }
            }
        }
    }
}

/// Munge the input string with the given mode.
fn munge_str(s: &str, munge: MungeMode) -> String {
    match munge {
        MungeMode::Deunicode => deunicode(s),
        _ => s.to_string(), // Munge::None
    }
}

/// Returns if the given url matches the site visiting policy.
fn matches_site_policy(url: Url, opts: CrawlOptions) -> bool {
    if url.host_str() == None {
        return false;
    }
    match opts.site {
        SitePolicy::Same => {
            if url.host_str().unwrap_or("fail.___") == opts.url.host_str().unwrap_or("nope.___") {
                return true;
            }
            return false;
        }
        SitePolicy::Subdomain => {
            let u = url.host_str().unwrap_or("fail.___");
            let u2 = opts.url.host_str().unwrap_or("nope.___");

            if u == u2 || u.ends_with(format!(".{}", u2).as_str()) {
                return true;
            }
            return false;
        }
        SitePolicy::All => return true,
    }
}
