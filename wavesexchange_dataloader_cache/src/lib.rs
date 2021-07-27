use std::{hash::Hash, time::Duration};
use dataloader::cached::Cache;
use ttl_cache::TtlCache;

pub struct TtlFIFOCache<K: Eq + Hash, V> {
    cache: TtlCache<K, V>,
    ttl: Duration,
}

impl<K: Eq + Hash, V> TtlFIFOCache<K, V> {
    pub fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            cache: TtlCache::new(capacity),
            ttl,
        }
    }
}

impl<K: Eq + Hash, V> Cache for TtlFIFOCache<K, V> {
    type Key = K;
    type Val = V;

    fn get(&self, key: &Self::Key) -> Option<&Self::Val> {
        self.cache.get(key)
    }

    fn insert(&mut self, key: Self::Key, val: Self::Val) {
        self.cache.insert(key, val, self.ttl.to_owned());
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Val> {
        self.cache.remove(key)
    }

    fn clear(&mut self) {
        self.cache.clear();
    }
}
