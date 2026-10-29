//! Opaque `source_id` → info-hash cache (SECURITY.md: validate opaque ids; the
//! client must not be able to forge an arbitrary torrent source).

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct SearchCache {
    entries: Mutex<HashMap<String, (String, Instant)>>,
    ttl: Duration,
    max_entries: usize,
}

impl SearchCache {
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
            max_entries,
        }
    }

    /// Store `id -> info_hash`. Evicts expired entries when over the cap; if
    /// still full, the new entry is dropped (basic, documented eviction).
    pub fn insert(&self, id: String, info_hash: String) {
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= self.max_entries {
            entries.retain(|_, (_, at)| at.elapsed() < self.ttl);
        }
        if entries.len() < self.max_entries {
            entries.insert(id, (info_hash, Instant::now()));
        }
    }

    /// Look up a live info hash by id (lazy TTL cleanup).
    pub fn get(&self, id: &str) -> Option<String> {
        let mut entries = self.entries.lock().unwrap();
        match entries.get(id) {
            Some((hash, at)) if at.elapsed() < self.ttl => Some(hash.clone()),
            _ => {
                entries.remove(id);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get_within_ttl() {
        let cache = SearchCache::new(Duration::from_secs(60), 10);
        cache.insert("abc".into(), "deadbeef".into());
        assert_eq!(cache.get("abc").as_deref(), Some("deadbeef"));
        assert_eq!(cache.get("missing"), None);
    }

    #[test]
    fn expired_entry_is_removed() {
        let cache = SearchCache::new(Duration::from_secs(0), 10);
        cache.insert("abc".into(), "deadbeef".into());
        assert_eq!(cache.get("abc"), None);
    }
}
