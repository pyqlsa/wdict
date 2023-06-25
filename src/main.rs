//! Simple cli tool to extract words from webpages to build a dictionary.
//!
use async_recursion::async_recursion;
use clap::{Parser, ValueEnum};
use deunicode::deunicode;
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
    limit: i32,
    /// File to write dictionary to (will be overwritten if it already exists).
    #[arg(short, long, default_value = "wdict.txt")]
    file: String,
    /// Munge strategy for words.
    #[arg(short, long, default_value = "none", value_enum)]
    munge: MungeMode,
}

#[derive(ValueEnum, Copy, Debug, Clone)]
enum MungeMode {
    Deunicode,
    None,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let depth = 0;
    let mut links = HashMap::new();
    let mut words = HashMap::new();
    links.insert(String::from(args.url), false);

    crawl(&mut links, &mut words, depth, args.limit, args.munge).await;

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
    let file = File::create(args.file);
    match file {
        Ok(mut f) => {
            words
                .into_iter()
                .for_each(|(k, _v)| writeln!(f, "{}", k).unwrap());
        }
        Err(e) => {
            eprintln!("Error writing out dictionary: {}", e)
        }
    }
}

/// Crawl links up to a given limit, scraping links and words from pages.
#[async_recursion(?Send)]
async fn crawl(
    links: &mut HashMap<String, bool>,
    words: &mut HashMap<String, bool>,
    depth: i32,
    limit: i32,
    munge: MungeMode,
) -> () {
    let deeper = depth + 1;
    for (url, visited) in links.clone().into_iter() {
        if visited {
            println!("skipping.... already visited {}", url);
        } else {
            println!("visiting {}", url);
            links.insert(url.to_owned(), true);
            let document = doc_from_url(String::from(url)).await;
            links_from_doc(links, &document);
            words_from_doc(words, &document, munge);
        }
    }
    if deeper < limit {
        println!("going deeper... current depth {}", deeper);
        crawl(links, words, deeper, limit, munge).await;
    }
}

/// Get an html document from the provided url.
async fn doc_from_url(url: String) -> Html {
    let client = reqwest::Client::builder()
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
///
/// TODO: better filtering with things like:
/// - https://github.com/unicode-rs/unicode-normalization
/// - https://github.com/unicode-rs/unicode-security
/// - https://github.com/null8626/decancer
/// - https://github.com/kornelski/deunicode
fn words_from_doc(words: &mut HashMap<String, bool>, document: &Html, munge: MungeMode) -> () {
    for node in document.clone().tree {
        if let Node::Text(text) = node {
            let fintext = text.text.trim();
            let fintext = munge_str(fintext, munge);
            let fintext = fintext.to_lowercase();
            // ignore these characters since we're looking for words
            let fintext = fintext.replace(|c: char| !c.is_alphanumeric(), " ");
            if fintext.len() > 0 {
                for w in fintext.split_whitespace() {
                    words.entry(String::from(w)).or_insert(true);
                }
            }
        }
    }
}

fn munge_str(s: &str, munge: MungeMode) -> String {
    match munge {
        MungeMode::Deunicode => deunicode(s),
        _ => s.to_string(), // Munge::None
    }
}
