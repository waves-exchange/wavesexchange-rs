use std::{hash::Hash, time::Duration};
use ttl_cache::TtlCache;

pub struct SizedTtlCache<K: Eq + Hash, V> {
    cache: TtlCache<K, V>,
    ttl: Duration,
}

impl<K: Eq + Hash, V> SizedTtlCache<K, V> {
    pub fn new(size: usize, ttl: Duration) -> Self {
        Self {
            cache: TtlCache::new(size),
            ttl,
        }
    }
}

impl<K: Eq + Hash, V> dataloader::cached::Cache for SizedTtlCache<K, V> {
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
