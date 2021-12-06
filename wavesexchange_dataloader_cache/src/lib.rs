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
use wavesexchange_log::error;

static CACHES: Lazy<Mutex<ShareCloneMap>> = Lazy::new(|| Mutex::new(TypeMap::custom()));

pub trait CacheKey: Eq + Hash + Clone + Debug + Send + Sync + 'static {}
pub trait CacheVal: Clone + Debug + Send + 'static {}
pub trait CacheBounds<K: CacheKey, V: CacheVal>: cached::Cached<K, V> + Send + 'static {}

impl<T> CacheKey for T where T: Eq + Hash + Clone + Debug + Send + Sync + 'static {}
impl<T> CacheVal for T where T: Clone + Debug + Send + 'static {}
impl<K: CacheKey, V: CacheVal, T> CacheBounds<K, V> for T where
    T: cached::Cached<K, V> + Send + 'static
{
}

#[macro_use]
extern crate async_trait;

struct TyMapKey<T>(PhantomData<T>);

impl<K: CacheKey, V: CacheVal, C: CacheBounds<K, V>> typemap::Key for TyMapKey<(K, V, C)> {
    type Value = Arc<Mutex<Cacher<K, V, C>>>;
}

pub struct Cacher<K: CacheKey, V: CacheVal, C: CacheBounds<K, V>> {
    cache: C,
    cache_strategy: Box<dyn Fn(&K, &V) -> bool + Send + 'static>,
    cache_strategy_filtered_keys: Vec<K>,
    _pd: (PhantomData<K>, PhantomData<V>),
}

impl<K: CacheKey, V: CacheVal, C: CacheBounds<K, V>> DlCache for &mut Cacher<K, V, C> {
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

impl<K: CacheKey, V: CacheVal, C: CacheBounds<K, V>> Cacher<K, V, C> {
    fn new(cache: C, strategy_fn: impl Fn(&K, &V) -> bool + Send + 'static) -> Cacher<K, V, C> {
        Cacher {
            cache,
            cache_strategy: Box::new(strategy_fn),
            cache_strategy_filtered_keys: Vec::new(),
            _pd: (PhantomData, PhantomData),
        }
    }

    async fn get_or_init(
        inner_cache_fn: impl FnOnce() -> C,
        strategy_fn: impl Fn(&K, &V) -> bool + Send + 'static,
    ) -> Arc<Mutex<Cacher<K, V, C>>> {
        let mut caches = CACHES.lock().await;
        let entry = caches
            .entry::<TyMapKey<(K, V, C)>>()
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

type LocalLoader<'c, K, V, CL> =
    Loader<K, V, BatchFnWrapper<CL>, &'c mut Cacher<K, V, <CL as CachedLoader<K, V>>::Cache>>;

#[async_trait]
pub trait CachedLoader<K: CacheKey, V: CacheVal>: Send + Sync + Clone + 'static {
    type Cache: CacheBounds<K, V>;

    /// Setup cache type
    fn init_cache() -> Self::Cache;

    /// Modify loader params
    fn init_loader(loader: LocalLoader<K, V, Self>) -> LocalLoader<K, V, Self> {
        loader
    }

    /// I.e. cache only Ok(...), but not Err
    #[inline]
    fn cache_strategy(_: &K, _: &V) -> bool {
        true
    }

    /// Like a method in BatchFn
    async fn load_fn(&mut self, keys: &[K]) -> Vec<V>;

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
impl<K: CacheKey, V: CacheVal, C: CachedLoader<K, V>> BatchFn<K, V> for BatchFnWrapper<C> {
    async fn load(&mut self, keys: &[K]) -> HashMap<K, V> {
        let values = self.0.load_fn(keys).await;
        if keys.len() != values.len() {
            error!(
                "Keys and values vectors aren't length-equal! keys: {:?} ;;; values: {:?}",
                &keys, &values
            );
        }
        keys.iter().cloned().zip(values).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};
    use tokio::time::sleep;

    //upper border, cached values are usually extracted faster
    const CACHED_DUR: Duration = Duration::from_millis(1);
    //we assume that loader functions are slow, so we just sleep in them
    const SLEEP_DUR: Duration = Duration::from_secs(1);

    fn is_cached(load_time: Duration) -> bool {
        load_time < CACHED_DUR
    }

    fn is_not_cached(load_time: Duration) -> bool {
        load_time >= SLEEP_DUR
    }

    async fn measure_load<K: CacheKey, V: CacheVal + Eq, C: CachedLoader<K, V>>(
        loader: &C,
        key: K,
        expected_val: V,
        measure_fn: impl Fn(Duration) -> bool,
    ) -> bool {
        let now = SystemTime::now();
        let result = loader.load(key.clone()).await;
        let elapsed = now.elapsed().unwrap();
        let measurement_is_ok = measure_fn(elapsed);
        let values_are_eq = result == expected_val;
        if !measurement_is_ok {
            println!("Error: measure fn failed!");
        }
        if !values_are_eq {
            println!("Error: expected and loaded values are not equal");
            println!(
                "key: {:?}, value: {:?}, expected value: {:?}",
                key, result, expected_val
            );
        }
        values_are_eq && measurement_is_ok
    }

    #[tokio::test]
    async fn test_timed() {
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<u64, String> for Loadable {
            type Cache = TimedCache<u64, String>;

            async fn load_fn(&mut self, keys: &[u64]) -> Vec<String> {
                sleep(SLEEP_DUR).await;
                keys.into_iter().map(|k| format!("num: {}", k)).collect()
            }

            fn init_cache() -> Self::Cache {
                TimedCache::with_lifespan(3) //seconds to persist in cache
            }
        }

        let loader = Loadable {};
        assert!(measure_load(&loader, 4, "num: 4".to_string(), is_not_cached).await);

        //value is cached
        assert!(measure_load(&loader, 4, "num: 4".to_string(), is_cached).await);
        sleep(Duration::from_secs(3)).await;

        //value is dropped due to ttl
        assert!(measure_load(&loader, 4, "num: 4".to_string(), is_not_cached).await);
    }

    #[tokio::test]
    async fn test_sized() {
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<isize, String> for Loadable {
            type Cache = SizedCache<isize, String>;

            async fn load_fn(&mut self, keys: &[isize]) -> Vec<String> {
                sleep(SLEEP_DUR).await;
                keys.into_iter().map(|k| format!("num: {}", k)).collect()
            }

            fn init_cache() -> Self::Cache {
                SizedCache::with_size(1)
            }
        }

        let loader = Loadable {};
        assert!(measure_load(&loader, -65535, "num: -65535".to_string(), is_not_cached).await);

        //value is cached
        assert!(measure_load(&loader, -65535, "num: -65535".to_string(), is_cached).await);

        //rewriting the only available cache cell
        assert!(measure_load(&loader, -4, "num: -4".to_string(), is_not_cached).await);
        assert!(measure_load(&loader, -4, "num: -4".to_string(), is_cached).await);

        //first value is dropped because cache size is exceeded
        assert!(measure_load(&loader, -65535, "num: -65535".to_string(), is_not_cached).await);
    }

    #[tokio::test]
    async fn test_cache_strategy() {
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<isize, Option<String>> for Loadable {
            type Cache = UnboundCache<isize, Option<String>>;

            async fn load_fn(&mut self, keys: &[isize]) -> Vec<Option<String>> {
                sleep(SLEEP_DUR).await;
                keys.into_iter()
                    .map(|k| {
                        if k % 2 == 0 {
                            // loader fn returns string only with even numbers
                            Some(format!("num: {}", k))
                        } else {
                            None
                        }
                    })
                    .collect()
            }

            fn init_cache() -> Self::Cache {
                UnboundCache::new()
            }

            fn cache_strategy(_: &isize, v: &Option<String>) -> bool {
                v.is_some()
            }
        }

        //even number
        let loader = Loadable {};
        assert!(measure_load(&loader, 28, Some("num: 28".to_string()), is_not_cached).await);

        //is cached
        assert!(measure_load(&loader, 28, Some("num: 28".to_string()), is_cached).await);

        //odd number
        assert!(measure_load(&loader, 5, None, is_not_cached).await);

        //is not cached
        assert!(measure_load(&loader, 5, None, is_not_cached).await);
    }
}
