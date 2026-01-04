use clap::builder::ValueParser;
use log::warn;
use reqwest::Url;
use std::fs;
use std::io::{self, BufRead};

use crate::collections::{UrlDb, WordDb};
use crate::error::Error;
use crate::utils;

use super::{Cli, FilterArg, SitePolicyArg};

/// Helper for json output URL file.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct State {
    pub starting_url: String,
    pub depth_reached: usize,
    pub visited: Vec<String>,
    pub staged: Vec<String>,
    pub unvisited: Vec<String>,
    pub skipped: Vec<String>,
    pub errored: Vec<String>,
    pub site_policy: SitePolicyArg,
    pub filters: Vec<FilterArg>,
    pub depth: usize,
    pub include_js: bool,
    pub include_css: bool,
    pub min_word_length: usize,
    pub max_word_length: usize,
    pub req_per_sec: u64,
    pub limit_concurrent: usize,
}

impl State {
    /// Returns a new UrlDb instance.
    pub fn new(url: &str) -> Self {
        Self {
            starting_url: url.to_string(),
            depth_reached: 0,
            visited: Vec::new(),
            staged: Vec::new(),
            unvisited: Vec::new(),
            skipped: Vec::new(),
            errored: Vec::new(),
            site_policy: SitePolicyArg::Same,
            filters: Vec::new(),
            depth: 1,
            include_js: false,
            include_css: false,
            min_word_length: 3,
            max_word_length: usize::MAX,
            req_per_sec: 5,
            limit_concurrent: 5,
        }
    }
    pub fn new_from_file(file: &str) -> Result<State, Error> {
        let contents = fs::read_to_string(file)?;
        let state: State = serde_json::from_str(contents.as_str())?;

        Ok(state)
    }
}

pub fn str_not_whitespace_parser() -> ValueParser {
    ValueParser::new(str_not_whitespace)
}

pub fn str_not_whitespace(value: &str) -> Result<String, Error> {
    if value.len() < 1 || value.trim().len() != value.len() {
        Err(Error::StrWhitespaceError)
    } else {
        Ok(value.trim().to_string())
    }
}

/// Helper for url parsing, predominantly to squash errors.
pub fn parse_url(url_str: &str) -> Result<Url, ()> {
    let res = Url::parse(url_str);
    match res {
        Err(e) => {
            warn!("error parsing url {}: {}", url_str, e);
            Err(())
        }
        Ok(u) => Ok(u),
    }
}

/// Helper for parsing a Target from cli args into a Url.
pub fn parse_target(args: &Cli) -> Result<Url, ()> {
    let t = &args.target;
    if let Some(url_str) = t.url.as_deref() {
        return parse_url(url_str);
    }

    if let Some(t) = t.theme {
        return parse_url(t.as_str());
    }

    if let Some(p) = t.path.as_deref() {
        let res = utils::url_from_path_str(&p);
        match res {
            Err(e) => {
                warn!("error parsing path {} as url: {}", p, e);
                return Err(());
            }
            Ok(u) => {
                return parse_url(&u.as_str());
            }
        }
    }

    return Err(());
}
/// Helper for parsing a State from cli args.
pub fn parse_state(args: &Cli) -> Result<State, ()> {
    if args.target.resume || args.target.resume_strict {
        let state_res = State::new_from_file(args.state_file.as_str());
        match state_res {
            Err(e) => {
                warn!("error extracting state from {}: {}", args.state_file, e);
                return Err(());
            }
            Ok(s) => {
                return Ok(s);
            }
        }
    }
    let url_res = parse_target(&args);
    if let Err(_) = url_res {
        return Err(());
    }
    let url = url_res.unwrap();

    return Ok(State::new(url.as_str()));
}

// Popuplate urldb from state; state urls are consumed.
pub fn fill_urldb_from_state(db: &mut UrlDb, s: &mut State) {
    s.visited
        .drain(0..)
        .into_iter()
        .for_each(|u| db.mark_visited(&u));
    s.staged
        .drain(0..)
        .into_iter()
        .for_each(|u| db.mark_staged(&u));
    s.unvisited
        .drain(0..)
        .into_iter()
        .for_each(|u| db.mark_unvisited(&u));
    s.skipped
        .drain(0..)
        .into_iter()
        .for_each(|u| db.mark_skipped(&u));
    s.errored
        .drain(0..)
        .into_iter()
        .for_each(|u| db.mark_errored(&u));
}

// Popuplate worddb from existing dictionary.
pub fn fill_worddb_from_file(db: &mut WordDb, file: &str) {
    let file_res = fs::File::open(file);
    match file_res {
        Err(e) => {
            warn!("failed opening dictionary {}: {}", file, e);
            warn!("...continuing without previous words");
            return;
        }
        Ok(f) => {
            let lines = io::BufReader::new(f).lines().flatten();
            for line in lines {
                db.insert(line);
            }
        }
    }
}
