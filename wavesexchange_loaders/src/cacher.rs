use anymap::{any::Any, Map};
use cached::async_sync::Mutex;
use dataloader::cached::Cache as DlCache;
use once_cell::sync::Lazy;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

static CACHES: Lazy<Mutex<Map<dyn Any + Send + Sync>>> = Lazy::new(|| Mutex::new(Map::new()));

pub trait SharedObj: Send + Sync + 'static {}
pub trait CacheKey: Eq + Hash + Clone + Debug + SharedObj {}
pub trait CacheVal: Clone + Debug + SharedObj {}
pub trait CacheBounds<K: CacheKey, V: CacheVal>: cached::Cached<K, V> + SharedObj {}
pub trait ErrBounds: Debug + Send {}

impl<T> SharedObj for T where T: Send + Sync + 'static {}
impl<T> CacheKey for T where T: Eq + Hash + Clone + Debug + SharedObj {}
impl<T> CacheVal for T where T: Clone + Debug + SharedObj {}
impl<K: CacheKey, V: CacheVal, T> CacheBounds<K, V> for T where T: cached::Cached<K, V> + SharedObj {}
impl<T> ErrBounds for T where T: Debug + Send {}

pub struct Cacher<K: CacheKey, V: CacheVal, C: CacheBounds<K, V>> {
    cache: C,
    cache_strategy: Box<dyn Fn(&K, &V) -> bool + Send + 'static>,
    keys_to_drop: Vec<K>,
}

impl<K: CacheKey, V: CacheVal, C: CacheBounds<K, V>> DlCache for &mut Cacher<K, V, C> {
    type Key = K;
    type Val = V;

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Val> {
        self.cache.cache_get(key)
    }

    fn insert(&mut self, key: Self::Key, val: Self::Val) {
        if !(self.cache_strategy)(&key, &val) {
            self.add_key_to_drop(&key)
        }
        self.cache.cache_set(key, val);
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Val> {
        self.cache.cache_remove(key)
    }

    fn clear(&mut self) {
        self.cache.cache_clear()
    }
}

impl<K: CacheKey, V: CacheVal, C: CacheBounds<K, V>> Cacher<K, V, C> {
    fn new(cache: C, strategy_fn: impl Fn(&K, &V) -> bool + SharedObj) -> Cacher<K, V, C> {
        Cacher {
            cache,
            cache_strategy: Box::new(strategy_fn),
            keys_to_drop: Vec::new(),
        }
    }

    pub async fn get_or_init(
        inner_cache_fn: impl FnOnce() -> C,
        strategy_fn: impl Fn(&K, &V) -> bool + SharedObj,
    ) -> Arc<Mutex<Cacher<K, V, C>>> {
        let mut caches = CACHES.lock().await;
        let entry = caches
            .entry::<Arc<Mutex<Cacher<K, V, C>>>>()
            .or_insert(Arc::new(Mutex::new(Self::new(
                inner_cache_fn(),
                strategy_fn,
            ))));
        entry.clone()
    }

    pub fn add_key_to_drop(&mut self, key: &K) {
        self.keys_to_drop.push(key.clone())
    }

    pub fn cleanup(&mut self) {
        let keys_to_remove = self.keys_to_drop.drain(..).collect::<Vec<K>>();
        for key in keys_to_remove {
            (&mut *self).remove(&key);
        }
    }
}
