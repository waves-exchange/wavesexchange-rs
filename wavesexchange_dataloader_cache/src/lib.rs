pub mod typemap;

use cached::async_mutex::Mutex;
pub use cached::{SizedCache, TimedCache, TimedSizedCache, UnboundCache};
use dataloader::{
    cached::{Cache as DlCache, Loader},
    BatchFn,
};
use once_cell::sync::Lazy;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use std::{collections::HashMap, hash::Hash};
use typemap::{ShareCloneMap, TypeMap};

static CACHES: Lazy<Mutex<ShareCloneMap>> = Lazy::new(|| Mutex::new(TypeMap::custom()));

#[macro_use]
extern crate async_trait;

impl<K, V, C, SF> typemap::Key for (K, V, C, SF)
where
    K: Eq + Hash + Clone + Debug + Send + Sync + 'static,
    V: Clone + Send + 'static,
    C: cached::Cached<K, V> + Send + 'static,
    SF: Fn(&K, &V) -> bool + 'static,
{
    type Value = Arc<Mutex<Cacher<K, V, C, SF>>>;
}

pub struct Cacher<K, V, C, SF>
where
    K: Eq + Hash + Clone + Debug + Send + Sync,
    V: Clone + Send,
    C: cached::Cached<K, V>,
    SF: Fn(&K, &V) -> bool,
{
    cache: C,
    cache_strategy: SF,
    cache_strategy_filtered_keys: Vec<K>,
    _pd: (PhantomData<K>, PhantomData<V>),
}

impl<K, V, C, SF> DlCache for &mut Cacher<K, V, C, SF>
where
    K: Eq + Hash + Clone + Debug + Send + Sync,
    V: Clone + Send,
    C: cached::Cached<K, V>,
    SF: Fn(&K, &V) -> bool,
{
    type Key = K;
    type Val = V;

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Val> {
        self.cache.cache_get(key)
    }

    fn insert(&mut self, key: Self::Key, val: Self::Val) {
        if !(self.cache_strategy)(&key, &val) {
            self.cache_strategy_filtered_keys.push(key.clone());
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

impl<K, V, C, SF> Cacher<K, V, C, SF>
where
    K: Eq + Hash + Clone + Debug + Send + Sync + 'static,
    V: Clone + Send + 'static,
    C: cached::Cached<K, V> + Send + 'static,
    SF: Fn(&K, &V) -> bool + Send + 'static,
{
    fn new(cache: C, strategy_fn: SF) -> Cacher<K, V, C, SF> {
        Cacher {
            cache,
            cache_strategy: strategy_fn,
            cache_strategy_filtered_keys: Vec::new(),
            _pd: (PhantomData, PhantomData),
        }
    }

    async fn get_or_init<F: FnOnce() -> C>(
        inner_cache_fn: F,
        strategy_fn: SF,
    ) -> Arc<Mutex<Cacher<K, V, C, SF>>> {
        let mut caches = CACHES.lock().await;
        let entry = caches
            .entry::<(K, V, C, SF)>()
            .or_insert(Arc::new(Mutex::new(Self::new(
                inner_cache_fn(),
                strategy_fn,
            ))));
        entry.clone()
    }

    fn cleanup(&mut self) {
        let keys_to_remove = self
            .cache_strategy_filtered_keys
            .drain(..)
            .collect::<Vec<K>>();
        for key in keys_to_remove {
            (&mut *self).remove(&key).expect("unreachable");
        }
    }
}

type LocalLoader<'c, K, V, CL, SF> =
    Loader<K, V, BatchFnWrapper<CL>, &'c mut Cacher<K, V, <CL as CachedLoader<K, V>>::Cache, SF>>;

#[async_trait]
pub trait CachedLoader<K, V>: Send + Clone + 'static
where
    K: Eq + Hash + Clone + Debug + Send + Sync + 'static,
    V: Clone + Send + 'static,
{
    type Cache: cached::Cached<K, V> + Send + 'static;

    /// Setup cache type
    fn init_cache() -> Self::Cache;

    /// Modify loader params
    fn init_loader<SF: Fn(&K, &V) -> bool>(
        loader: LocalLoader<K, V, Self, SF>,
    ) -> LocalLoader<K, V, Self, SF> {
        loader
    }

    /// I.e. cache only Ok(...), but not Err
    #[inline]
    fn cache_strategy(_: &K, _: &V) -> bool {
        true
    }

    /// Like a method in BatchFn
    async fn load_fn(&mut self, keys: &[K]) -> HashMap<K, V>;

    /// Don't override this
    async fn load(&self, key: K) -> V {
        let wrapper = BatchFnWrapper(self.clone());
        let cache = Cacher::get_or_init(Self::init_cache, Self::cache_strategy).await;
        let mut cache_lock = cache.lock().await;
        let loader = Loader::with_cache(wrapper, &mut *cache_lock);
        let result = Self::init_loader(loader).load(key).await;
        cache_lock.cleanup();
        result
    }
}

pub struct BatchFnWrapper<C>(C);

#[async_trait]
impl<K, V, C> BatchFn<K, V> for BatchFnWrapper<C>
where
    K: Eq + Hash + Clone + Debug + Send + Sync + 'static,
    V: Clone + Send + 'static,
    C: CachedLoader<K, V>,
{
    async fn load(&mut self, keys: &[K]) -> HashMap<K, V> {
        self.0.load_fn(keys).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};
    use tokio::time::sleep;

    //upper border, cached values are usually extracted faster
    const CACHED_DUR: Duration = Duration::from_millis(10);
    const SLEEP_DUR: Duration = Duration::from_secs(1);

    #[tokio::test]
    async fn test_timed() {
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<u64, String> for Loadable {
            type Cache = TimedCache<u64, String>;

            async fn load_fn(&mut self, keys: &[u64]) -> HashMap<u64, String> {
                sleep(SLEEP_DUR).await;
                HashMap::from_iter(
                    keys.into_iter()
                        .map(|k| (*k, format!("your number is {}", k))),
                )
            }

            fn init_cache() -> Self::Cache {
                TimedCache::with_lifespan(3) //seconds to persist in cache
            }
        }

        let loader = Loadable {};
        let now = SystemTime::now();
        let result = loader.load(4u64).await;
        assert!(now.elapsed().unwrap() >= SLEEP_DUR);
        assert_eq!(result, "your number is 4");

        //value is cached
        let now = SystemTime::now();
        loader.load(4u64).await;
        assert!(now.elapsed().unwrap() < CACHED_DUR);

        sleep(Duration::from_secs(3)).await;

        //value is dropped due to ttl
        let now = SystemTime::now();
        loader.load(4u64).await;
        assert!(now.elapsed().unwrap() >= SLEEP_DUR);
    }

    #[tokio::test]
    async fn test_sized() {
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<isize, String> for Loadable {
            type Cache = SizedCache<isize, String>;

            async fn load_fn(&mut self, keys: &[isize]) -> HashMap<isize, String> {
                sleep(SLEEP_DUR).await;
                HashMap::from_iter(
                    keys.into_iter()
                        .map(|k| (*k, format!("your signed number is {}", k))),
                )
            }

            fn init_cache() -> Self::Cache {
                SizedCache::with_size(1)
            }
        }

        let loader = Loadable {};
        let now = SystemTime::now();
        let result = loader.load(-65535isize).await;
        assert!(now.elapsed().unwrap() >= SLEEP_DUR);
        assert_eq!(result, "your signed number is -65535");

        //value is cached
        let now = SystemTime::now();
        loader.load(-65535isize).await;
        assert!(now.elapsed().unwrap() < CACHED_DUR);

        //rewriting only available cache entry
        let now = SystemTime::now();
        loader.load(-4isize).await;
        assert!(now.elapsed().unwrap() >= SLEEP_DUR);

        //value is dropped because cache size is exceeded
        let now = SystemTime::now();
        loader.load(-65535isize).await;
        assert!(now.elapsed().unwrap() >= SLEEP_DUR);
    }

    #[tokio::test]
    async fn test_cache_strategy() {
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<isize, Option<String>> for Loadable {
            type Cache = SizedCache<isize, Option<String>>;

            async fn load_fn(&mut self, keys: &[isize]) -> HashMap<isize, Option<String>> {
                sleep(SLEEP_DUR).await;
                HashMap::from_iter(keys.into_iter().map(|k| {
                    (
                        *k,
                        if k % 2 == 0 {
                            // returns only even numbers
                            Some(format!("your signed number is {}", k))
                        } else {
                            None
                        },
                    )
                }))
            }

            fn init_cache() -> Self::Cache {
                SizedCache::with_size(50)
            }

            fn cache_strategy(_: &isize, v: &Option<String>) -> bool {
                v.is_some()
            }
        }

        //even number
        let loader = Loadable {};
        let now = SystemTime::now();
        loader.load(28isize).await;
        assert!(now.elapsed().unwrap() >= SLEEP_DUR);

        //cached
        let now = SystemTime::now();
        loader.load(28isize).await;
        assert!(now.elapsed().unwrap() < CACHED_DUR);

        //odd number
        let now = SystemTime::now();
        loader.load(5isize).await;
        assert!(now.elapsed().unwrap() >= SLEEP_DUR);

        //not cached
        let now = SystemTime::now();
        loader.load(5isize).await;
        assert!(now.elapsed().unwrap() >= SLEEP_DUR);
    }
}
