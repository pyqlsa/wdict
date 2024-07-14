use reqwest::Url;
use std::fs;
use std::io::{self, BufRead};

use crate::collections::{UrlDb, WordDb};
use crate::utils;

use crate::cli::args::{Cli, State};

/// Helper for url parsing, predominantly to squash errors.
pub fn parse_url(url_str: &str) -> Result<Url, ()> {
    let res = Url::parse(url_str);
    match res {
        Err(e) => {
            eprintln!("error parsing url {}: {}", url_str, e);
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
                eprintln!("error parsing path {} as url: {}", p, e);
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
                eprintln!("error extracting state from {}: {}", args.state_file, e);
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
        .for_each(|u| db.mark_visited(u));
    s.staged
        .drain(0..)
        .into_iter()
        .for_each(|u| db.mark_staged(u));
    s.unvisited
        .drain(0..)
        .into_iter()
        .for_each(|u| db.mark_unvisited(u));
    s.skipped
        .drain(0..)
        .into_iter()
        .for_each(|u| db.mark_skipped(u));
    s.errored
        .drain(0..)
        .into_iter()
        .for_each(|u| db.mark_errored(u));
}

// Popuplate worddb from existing dictionary.
pub fn fill_worddb_from_file(db: &mut WordDb, file: &str) {
    let file_res = fs::File::open(file);
    match file_res {
        Err(e) => {
            eprintln!("failed opening dictionary {}: {}", file, e);
            eprintln!("...continuing without previous words");
            eprintln!();
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
