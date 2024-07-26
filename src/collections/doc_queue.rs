use bytes::Bytes;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

/// Provides a FIFO queue for documents.
#[derive(Debug)]
pub struct DocQueue(Arc<Mutex<VecDeque<Option<Bytes>>>>);

impl Clone for DocQueue {
    /// Returns a clone/handle of the given DocQueue.
    fn clone(&self) -> Self {
        DocQueue(Arc::clone(&self.0))
    }
}

impl DocQueue {
    /// Returns a new DocQueue instance.
    pub fn new() -> Self {
        DocQueue(Arc::new(Mutex::new(VecDeque::new())))
    }

    /// Push a document into the queue.
    pub fn push(&mut self, doc: Option<Bytes>) {
        let mut queue: MutexGuard<VecDeque<Option<Bytes>>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        queue.push_back(doc);
    }

    /// Pop a document from the queue.
    pub fn pop(&mut self) -> Option<Bytes> {
        let mut queue: MutexGuard<VecDeque<Option<Bytes>>> = match self.0.lock() {
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
        let queue: MutexGuard<VecDeque<Option<Bytes>>> = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        queue.is_empty()
    }
}
