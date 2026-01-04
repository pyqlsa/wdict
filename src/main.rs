//! Simple cli tool to extract words from webpages to build a dictionary.
//!
use clap::Parser;
use indicatif::MultiProgress;
use indicatif_log_bridge::LogWrapper;
use log::{error, info, LevelFilter};
use serde_json;
use std::io::Write;
use std::{fs, process::exit};
use tokio::signal;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use wdict::cli::{self, Cli, FilterArg, State};
use wdict::collections::{UrlDb, WordDb};
use wdict::crawl::{CrawlMode, CrawlOptions, Crawler};
use wdict::extract::ExtractOptions;
use wdict::{Error, Shutdown};

/// Main function.
#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut args = Cli::parse();

    let filter = args.verbose.log_level_filter();
    let multi = MultiProgress::new();
    setup_logger(multi.clone(), &filter);

    if args.target.resume || args.target.resume_strict {
        info!(
            "resuming from state '{}' and dictionary '{}'",
            args.state_file, args.output
        );
    }

    let state_res = cli::parse_state(&args);
    if let Err(_) = state_res {
        exit(1);
    }
    let mut in_state = state_res.unwrap();

    let url_res = cli::parse_url(in_state.starting_url.as_str());
    if let Err(_) = url_res {
        exit(1);
    }
    let url = url_res.unwrap();

    let crawl_mode = if url.scheme() == "file" {
        CrawlMode::Local
    } else {
        CrawlMode::Web
    };

    if args.target.resume_strict {
        args.depth = in_state.depth;
        args.include_js = in_state.include_js;
        args.include_css = in_state.include_css;
        args.site_policy = in_state.site_policy;
        args.req_per_sec = in_state.req_per_sec;
        args.limit_concurrent = in_state.limit_concurrent;
        args.min_word_length = in_state.min_word_length;
        args.max_word_length = in_state.max_word_length;
        args.filters = in_state.filters.drain(0..).collect();
    }

    info!(
        "using '{}' as target with crawl mode: {}",
        &url.as_str(),
        crawl_mode
    );

    let (notify_shutdown, _) = broadcast::channel(1);
    let copts = CrawlOptions::new(
        &url,
        args.depth,
        args.include_js,
        args.include_css,
        args.site_policy.to_mode(),
        args.req_per_sec,
        args.limit_concurrent,
        crawl_mode,
    );
    let eopts = ExtractOptions::new(
        args.min_word_length,
        args.max_word_length,
        args.include_js,
        args.include_css,
        FilterArg::to_modes(&args.filters),
    );

    let urldb: UrlDb = UrlDb::new();
    let mut uc = urldb.clone();
    cli::fill_urldb_from_state(&mut uc, &mut in_state); // resume

    let worddb: WordDb = WordDb::new();
    let mut wc = worddb.clone();
    if args.target.resume || args.target.resume_strict || args.append {
        cli::fill_worddb_from_file(&mut wc, &args.output);
    }

    let mut crawler = Crawler::new(
        copts,
        eopts,
        uc,
        wc,
        Shutdown::new(notify_shutdown.subscribe()),
        multi.clone(),
    )?;
    crawler.set_depth(in_state.depth_reached); // resume

    let crawl_handle = tokio::spawn(async move { crawler.crawl().await });
    let sig_handle = tokio::spawn(async move {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("shutting down...");
                // When `notify_shutdown` is dropped, all tasks which have `subscribe`d will
                // receive the shutdown signal and can exit
                drop(notify_shutdown);
            }
        }
    });

    // wait for all the crawling to complete
    let depth_reached = wait_for_crawl(crawl_handle).await;

    // cleanup if we weren't interrupted
    if !sig_handle.is_finished() {
        sig_handle.abort();
    }

    let len_urls = urldb.num_visited_urls();
    let len_words = worddb.len();
    info!(
        "reached depth: {}; unique words {}; visited urls: {}",
        depth_reached, len_words, len_urls
    );

    if args.no_write {
        info!("Skipping dictionary creation");
    } else {
        let mut file =
            fs::File::create(args.output.clone()).expect("Error creating dictionary file");
        let mut contents = String::new();
        worddb.iter().for_each(|word| {
            let line = format!("{}\n", word);
            contents.push_str(&line);
        });
        file.write_all(contents.as_bytes())
            .expect("Error writing to dictionary");
        info!("dictionary written to: {}", args.output);
    }

    if args.output_state {
        let out_state = State {
            starting_url: url.to_string(),
            depth_reached,
            visited: urldb.visited_urls_iter().collect(),
            staged: urldb.staged_urls_iter().collect(),
            unvisited: urldb.unvisited_urls_iter().collect(),
            skipped: urldb.skipped_urls_iter().collect(),
            errored: urldb.errored_urls_iter().collect(),
            depth: args.depth,
            filters: args.filters,
            include_css: args.include_css,
            include_js: args.include_js,
            req_per_sec: args.req_per_sec,
            limit_concurrent: args.limit_concurrent,
            min_word_length: args.min_word_length,
            max_word_length: args.max_word_length,
            site_policy: args.site_policy,
        };
        let url_file = args.state_file;
        if let Ok(j) = serde_json::to_string_pretty(&out_state) {
            let mut file = fs::File::create(url_file.clone()).expect("Error creating state file");
            file.write_all(j.as_bytes())
                .expect("Error writing state to file");
            info!("state written to file: {}", url_file);
        } else {
            error!("Error serializing output state json")
        }
    }

    Ok(())
}

fn setup_logger(m: MultiProgress, f: &LevelFilter) -> () {
    let filter_str = format!("none,wdict={}", f.as_str());
    let logenv = env_logger::Env::default().default_filter_or(filter_str);
    let logger = env_logger::Builder::from_env(logenv).build();
    let level = logger.filter();

    if let Err(e) = LogWrapper::new(m, logger).try_init() {
        eprintln!("failed setting up logger: {}", e);
        exit(1);
    }
    log::set_max_level(level);
}

/// Wait for crawl thread handle to complete; retrned the max depth_reached
/// while crawling.
async fn wait_for_crawl(h: JoinHandle<Result<usize, Error>>) -> usize {
    let mut depth_reached = 0;
    //let (r1,) = tokio::join!(h1);
    let r1 = h.await;
    match r1 {
        Ok(res) => match res {
            Err(err) => {
                error!("unexpected error while crawling {}", err);
            }
            Ok(i) => depth_reached = i,
        },
        Err(err) => {
            error!("unexpected error while joining threads {}", err);
        }
    }
    depth_reached
}
