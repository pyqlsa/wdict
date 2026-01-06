use clap::builder::ValueParser;
use log::{info, warn};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Url;
use std::fs;
use std::io::{self, BufRead};
use std::str::FromStr;

use crate::collections::{UrlDb, WordDb};
use crate::error::Error;
use crate::utils;

use super::{Cli, FilterArg, SitePolicyArg};

/// Helper for json output URL file.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct State {
    pub starting_url: Url,
    pub depth_reached: usize,
    pub visited: Vec<String>,
    pub staged: Vec<String>,
    pub unvisited: Vec<String>,
    pub skipped: Vec<String>,
    pub errored: Vec<String>,
    pub site_policy: SitePolicyArg,
    pub user_agent: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
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
    /// Returns a new State instance.
    pub fn new(url: Url) -> Self {
        Self {
            starting_url: url.clone(),
            depth_reached: 0,
            visited: Vec::new(),
            staged: Vec::new(),
            unvisited: Vec::new(),
            skipped: Vec::new(),
            errored: Vec::new(),
            site_policy: SitePolicyArg::Same,
            user_agent: None,
            headers: None,
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

    /// Returns a new State instance constructed from the given file path.
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
        Err(Error::GeneralError(
            "value cannot have leading/trailing whitespace, nor consist of only whitespace"
                .to_string(),
        ))
    } else {
        Ok(value.trim().to_string())
    }
}

pub fn headers_parser() -> ValueParser {
    ValueParser::new(parse_key_val)
}

pub fn parse_key_val(s: &str) -> Result<(String, String), Error> {
    s.split_once('=')
        .map(|(key, val)| (key.to_owned(), val.to_owned()))
        .ok_or(Error::GeneralError(
            "invalid header format; use 'key=value'".to_string(),
        ))
}

/// Helper for url parsing, predominantly to wrap errors.
pub fn parse_url(url_str: &str) -> Result<Url, Error> {
    let res = Url::parse(url_str);
    match res {
        Err(e) => Err(Error::UrlParseError(e)),
        Ok(u) => Ok(u),
    }
}

/// Helper for header parsing from HashMap to HeaderMap.
pub fn parse_headers(hashmap: &Option<Vec<(String, String)>>) -> Result<Option<HeaderMap>, Error> {
    let mut headermap = HeaderMap::new();
    if let Some(hm) = hashmap {
        for (key, val) in hm {
            let hk_res = HeaderName::from_str(key.to_lowercase().as_ref());
            if let Err(e) = hk_res {
                return Err(Error::HeaderNameError(e));
            }

            let hv_res = HeaderValue::from_str(val.as_ref());
            if let Err(e) = hv_res {
                return Err(Error::HeaderValueError(e));
            }

            let hk = hk_res.unwrap();
            let hv = hv_res.unwrap();
            headermap.insert(hk, hv);
        }
    }
    if headermap.len() < 1 {
        Ok(None)
    } else {
        Ok(Some(headermap))
    }
}

/// Helper for parsing a Target from cli args into a Url.
pub fn parse_target(args: &Cli) -> Result<Url, Error> {
    let t = &args.target;
    if let Some(url_str) = t.url.as_deref() {
        let res = parse_url(url_str);
        match res {
            Err(e) => {
                warn!("error parsing target url {}", url_str);
                return Err(e);
            }
            Ok(u) => return Ok(u),
        }
    }

    if let Some(t) = t.theme {
        let res = parse_url(t.as_str());
        match res {
            Err(e) => {
                warn!("error parsing target theme url {}", t.as_str());
                return Err(e);
            }
            Ok(u) => return Ok(u),
        }
    }

    if let Some(p) = t.path.as_deref() {
        let res = utils::url_from_path_str(&p);
        match res {
            Err(e) => {
                warn!("error parsing target path {} as url", p);
                Err(e)
            }
            Ok(u) => parse_url(&u.as_str()),
        }
    } else {
        Err(Error::GeneralError(
            "no valid crawl target detected".to_string(),
        ))
    }
}

/// Build an initial State based on cli args.
pub fn build_initial_state(args: &mut Cli) -> Result<State, Error> {
    let state = if args.target.resume || args.target.resume_strict {
        info!(
            "resuming from state '{}' and dictionary '{}'",
            args.state_file, args.output
        );
        State::new_from_file(args.state_file.as_str())?
    } else {
        let url = parse_target(&args)?;
        State::new(url)
    };

    if args.target.resume_strict {
        args.depth = state.depth;
        args.include_js = state.include_js;
        args.include_css = state.include_css;
        args.site_policy = state.site_policy;
        args.user_agent = state.user_agent.clone();
        args.header = state.headers.clone();
        args.req_per_sec = state.req_per_sec;
        args.limit_concurrent = state.limit_concurrent;
        args.min_word_length = state.min_word_length;
        args.max_word_length = state.max_word_length;
        args.filters = state.filters.clone();
    }

    Ok(state)
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
