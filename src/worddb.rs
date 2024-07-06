use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

/// Stores unique words.
pub struct WordDb(Arc<Mutex<HashMap<String, bool>>>);

impl Clone for WordDb {
    /// Returns a clone/handle of the given WordDb.
    fn clone(&self) -> Self {
        WordDb(Arc::clone(&self.0))
    }
}

impl WordDb {
    /// Returns a new WordDb instance.
    pub fn new() -> Self {
        WordDb(Arc::new(Mutex::new(HashMap::new())))
    }

    /// Returns the length of the word database.
    pub fn len(&self) -> usize {
        let hm: MutexGuard<HashMap<String, bool>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.len()
    }

    /// Inserts word into db.
    pub fn insert(&mut self, url: String) -> () {
        let mut hm: MutexGuard<HashMap<String, bool>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.insert(url.to_owned(), true);
    }

    /// Returns an iterator over the discovered words.
    pub fn iter(&self) -> impl Iterator<Item = String> {
        let hm: MutexGuard<HashMap<String, bool>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        hm.clone().into_iter().map(|(k, _v)| k)
    }
}
