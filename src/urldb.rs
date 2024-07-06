use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

/// Stores urls, tracking whether or not they have been visited.
pub struct UrlDb(Arc<Mutex<HashMap<String, Status>>>);

impl Clone for UrlDb {
    /// Returns a clone/handle of the given UrlDb.
    fn clone(&self) -> Self {
        UrlDb(Arc::clone(&self.0))
    }
}

impl UrlDb {
    /// Returns a new UrlDb instance.
    pub fn new() -> Self {
        UrlDb(Arc::new(Mutex::new(HashMap::new())))
    }

    /// Returns an iterator over the urls that were discovered and visited.
    fn filter_visited_urls(&self) -> impl Iterator<Item = String> {
        let hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.clone()
            .into_iter()
            .filter(|(_k, v)| *v == Status::Visited)
            .map(|(k, _v)| k)
    }

    /// Returns an iterator over the urls that were discovered, but unvisited.
    fn filter_unvisited_urls(&self) -> impl Iterator<Item = String> {
        let hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.clone()
            .into_iter()
            .filter(|(_k, v)| *v == Status::Unvisited)
            .map(|(k, _v)| k)
    }

    /// Returns an iterator over the urls that were discovered, but skipped.
    fn filter_skipped_urls(&self) -> impl Iterator<Item = String> {
        let hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.clone()
            .into_iter()
            .filter(|(_k, v)| *v == Status::Skip)
            .map(|(k, _v)| k)
    }

    /// Returns an iterator over the urls that were discovered, but encountered and error while
    /// visiting.
    fn filter_errored_urls(&self) -> impl Iterator<Item = String> {
        let hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.clone()
            .into_iter()
            .filter(|(_k, v)| *v == Status::Error)
            .map(|(k, _v)| k)
    }

    /// Returns the urls that were visited.
    pub fn visited_urls(&self) -> Vec<String> {
        self.filter_visited_urls().collect()
    }

    /// Returns the urls that were unvisited.
    pub fn unvisited_urls(&self) -> Vec<String> {
        self.filter_unvisited_urls().collect()
    }

    /// Returns the urls that were skipped.
    pub fn skipped_urls(&self) -> Vec<String> {
        self.filter_skipped_urls().collect()
    }

    /// Returns the urls that encountred errors.
    pub fn errored_urls(&self) -> Vec<String> {
        self.filter_errored_urls().collect()
    }

    /// Returns the number of urls that were visited.
    pub fn num_visited_urls(&self) -> usize {
        self.filter_visited_urls().count()
    }

    /// Returns the number of urls that were unvisited.
    pub fn num_unvisited_urls(&self) -> usize {
        self.filter_unvisited_urls().count()
    }

    /// Returns the number of urls that were skipped.
    pub fn num_skipped_urls(&self) -> usize {
        self.filter_skipped_urls().count()
    }

    /// Returns the number of urls that encountered an error.
    pub fn num_errored_urls(&self) -> usize {
        self.filter_errored_urls().count()
    }

    /// Inserts and marks a url as visited.
    pub fn mark_visited(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.insert(url.to_owned(), Status::Visited);
    }

    /// Inserts and marks a url as unvisited.
    pub fn mark_unvisited(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.insert(url.to_owned(), Status::Unvisited);
    }

    /// Inserts and marks a url as skipped.
    pub fn mark_skipped(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.insert(url.to_owned(), Status::Skip);
    }

    /// Inserts and marks a url as errored.
    pub fn mark_errored(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.insert(url.to_owned(), Status::Error);
    }

    /// Inserts and marks a url as visited, only if the url is new.
    pub fn cond_mark_visited(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.entry(url.clone()).or_insert(Status::Visited);
    }

    /// Inserts and marks a url as unvisited, only if the url is new.
    pub fn cond_mark_unvisited(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.entry(url.clone()).or_insert(Status::Unvisited);
    }

    /// Inserts and marks a url as skipped, only if the url is new.
    pub fn cond_mark_skipped(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.entry(url.clone()).or_insert(Status::Skip);
    }

    /// Inserts and marks a url as errored, only if the url is new.
    pub fn cond_mark_errored(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.entry(url.clone()).or_insert(Status::Error);
    }
}

#[derive(Copy, Debug, Clone)]
enum Status {
    Visited,
    Unvisited,
    Skip,
    Error,
}

impl PartialEq for Status {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Visited, Self::Visited)
            | (Self::Unvisited, Self::Unvisited)
            | (Self::Skip, Self::Skip)
            | (Self::Error, Self::Error) => true,
            _ => false,
        }
    }
}
