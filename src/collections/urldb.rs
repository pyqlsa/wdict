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
    pub fn visited_urls_iter(&self) -> impl Iterator<Item = String> {
        let hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.clone()
            .into_iter()
            .filter(|(_k, v)| *v == Status::Visited)
            .map(|(k, _v)| k)
    }

    /// Returns an iterator over the urls that are currently staged.
    pub fn staged_urls_iter(&self) -> impl Iterator<Item = String> {
        let hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.clone()
            .into_iter()
            .filter(|(_k, v)| *v == Status::Staged)
            .map(|(k, _v)| k)
    }

    /// Returns an iterator over the urls that were discovered, but unvisited.
    pub fn unvisited_urls_iter(&self) -> impl Iterator<Item = String> {
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
    pub fn skipped_urls_iter(&self) -> impl Iterator<Item = String> {
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
    pub fn errored_urls_iter(&self) -> impl Iterator<Item = String> {
        let hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.clone()
            .into_iter()
            .filter(|(_k, v)| *v == Status::Error)
            .map(|(k, _v)| k)
    }

    /// Returns the number of urls that were visited.
    pub fn num_visited_urls(&self) -> usize {
        self.visited_urls_iter().count()
    }

    /// Returns the number of urls that were staged.
    pub fn num_staged_urls(&self) -> usize {
        self.staged_urls_iter().count()
    }

    /// Returns the number of urls that were unvisited.
    pub fn num_unvisited_urls(&self) -> usize {
        self.unvisited_urls_iter().count()
    }

    /// Returns the number of urls that were skipped.
    pub fn num_skipped_urls(&self) -> usize {
        self.skipped_urls_iter().count()
    }

    /// Returns the number of urls that encountered an error.
    pub fn num_errored_urls(&self) -> usize {
        self.errored_urls_iter().count()
    }

    /// Inserts and marks a url as visited.
    pub fn mark_visited(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.insert(url.to_owned(), Status::Visited);
    }

    /// Inserts and marks a url as staged.
    pub fn mark_staged(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.insert(url.to_owned(), Status::Staged);
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

    /// Inserts and marks a url as staged, only if the url is new.
    pub fn cond_mark_staged(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.entry(url.clone()).or_insert(Status::Staged);
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

    /// Move all unvisited urls onto the stage.
    pub fn stage_unvisited_urls(&mut self) {
        let mut hm: MutexGuard<HashMap<String, Status>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        for (k, _v) in hm
            .clone()
            .into_iter()
            .filter(|(_k, v)| *v == Status::Unvisited)
        {
            hm.insert(k, Status::Staged);
        }
    }
}

#[derive(Copy, Debug, Clone)]
enum Status {
    /// A Url that was already visited successfully.
    Visited,
    /// A Url that has not yet been visited, but is about to be.
    Staged,
    /// A newly discovered Url that has not yet been processed or visited.
    Unvisited,
    /// A Url that has been determined to be skipped; ultimately will not be visted.
    Skip,
    /// A Url of which an error was encountered while attempting to visit.
    Error,
}

impl PartialEq for Status {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Visited, Self::Visited)
            | (Self::Staged, Self::Staged)
            | (Self::Unvisited, Self::Unvisited)
            | (Self::Skip, Self::Skip)
            | (Self::Error, Self::Error) => true,
            _ => false,
        }
    }
}
