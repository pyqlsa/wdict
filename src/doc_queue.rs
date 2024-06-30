use std::collections::VecDeque;
use std::string::String;
use std::sync::{Arc, Mutex, MutexGuard};

/// Provides a FIFO queue for documents.
pub struct DocQueue(Arc<Mutex<VecDeque<Option<String>>>>);

impl DocQueue {
    /// Returns a new DocQueue instance.
    pub fn new() -> Self {
        DocQueue(Arc::new(Mutex::new(VecDeque::new())))
    }

    /// Returns a clone/handle of the given DocQueue.
    pub fn clone(q: &Self) -> Self {
        DocQueue(Arc::clone(&q.0))
    }

    /// Push a document into the queue.
    pub fn push(&mut self, doc: Option<String>) {
        let mut queue: MutexGuard<VecDeque<Option<String>>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        queue.push_back(doc);
    }

    /// Pop a document from the queue.
    pub fn pop(&mut self) -> Option<String> {
        let mut queue: MutexGuard<VecDeque<Option<String>>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let doc = if let Some(d) = queue.pop_front() {
            d
        } else {
            None
        };
        doc
    }

    /// Return if the queue is empty or not.
    pub fn is_empty(&self) -> bool {
        let queue: MutexGuard<VecDeque<Option<String>>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        queue.is_empty()
    }
}
