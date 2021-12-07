use crate::cacher::{CacheBounds, CacheKey, CacheVal, Cacher, SharedObj};
use dataloader::{cached, non_cached, BatchFn};
use std::collections::HashMap;
use std::fmt::Debug;
use wavesexchange_log::error;

pub trait BaseLoader<K: CacheKey, V: CacheVal, L> {}

impl<'b, K: CacheKey, V: CacheVal, L: NonCachedLoader<K, V>> BaseLoader<K, V, L>
    for non_cached::Loader<K, V, &'b mut BatchFnWrapper<K, V, L>>
{
}

impl<'b, K: CacheKey, V: CacheVal, L: CachedLoader<K, V>> BaseLoader<K, V, L>
    for cached::Loader<
        K,
        V,
        &'b mut BatchFnWrapperCached<K, V, L>,
        &'b mut Cacher<K, V, <L as CachedLoader<K, V>>::Cache>,
    >
{
}

#[async_trait]
pub trait NonCachedLoader<K: CacheKey, V: CacheVal>: SharedObj + Clone {
    type LoadError: Debug + Send;

    /// Modify loader params
    fn init_loader<DL: BaseLoader<K, V, Self>>(loader: DL) -> DL {
        loader
    }

    /// Like a method in BatchFn
    async fn load_fn(&mut self, keys: &[K]) -> Result<Vec<V>, Self::LoadError>;

    /// Don't override this
    async fn load(&self, key: K) -> Result<V, Self::LoadError> {
        let mut batch_wrapper = BatchFnWrapper::new(self.clone());
        let loader = non_cached::Loader::new(&mut batch_wrapper);
        let result = Self::init_loader(loader).load(key.clone()).await;
        match batch_wrapper.error {
            Some(e) => Err(e),
            None => Ok(result),
        }
    }
}

#[async_trait]
pub trait CachedLoader<K: CacheKey, V: CacheVal>: SharedObj + Clone {
    type Cache: CacheBounds<K, V>;
    type LoadError: Debug + Send;

    /// Modify loader params
    fn init_loader<DL: BaseLoader<K, V, Self>>(loader: DL) -> DL {
        loader
    }

    /// Like a method in BatchFn
    async fn load_fn(&mut self, keys: &[K]) -> Result<Vec<V>, Self::LoadError>;

    /// Setup cache type
    fn init_cache() -> Self::Cache;

    /// I.e. cache only Some(...), but not None
    #[inline]
    fn cache_strategy(_: &K, _: &V) -> bool {
        true
    }

    /// Don't override this
    async fn load(&self, key: K) -> Result<V, Self::LoadError> {
        let mut batch_wrapper = BatchFnWrapperCached::new(self.clone());
        let cache = Cacher::get_or_init(Self::init_cache, Self::cache_strategy).await;
        let mut cache_lock = cache.lock().await;
        let loader = cached::Loader::with_cache(&mut batch_wrapper, &mut *cache_lock);
        let result = Self::init_loader(loader).load(key.clone()).await;
        if batch_wrapper.error.is_some() {
            cache_lock.add_key_to_drop(&key);
        }
        cache_lock.cleanup();
        match batch_wrapper.error {
            Some(e) => Err(e),
            None => Ok(result),
        }
    }
}

// sorry, waiting for specialization
pub(crate) struct BatchFnWrapper<K: CacheKey, V: CacheVal, C: NonCachedLoader<K, V>> {
    inner: C,
    error: Option<C::LoadError>,
}
pub(crate) struct BatchFnWrapperCached<K: CacheKey, V: CacheVal, C: CachedLoader<K, V>> {
    inner: C,
    error: Option<C::LoadError>,
}

impl<K: CacheKey, V: CacheVal, C: NonCachedLoader<K, V>> BatchFnWrapper<K, V, C> {
    pub fn new(inner: C) -> Self {
        BatchFnWrapper { inner, error: None }
    }
}

impl<K: CacheKey, V: CacheVal, C: CachedLoader<K, V>> BatchFnWrapperCached<K, V, C> {
    pub fn new(inner: C) -> Self {
        BatchFnWrapperCached { inner, error: None }
    }
}

#[async_trait]
impl<K: CacheKey, V: CacheVal, C: NonCachedLoader<K, V>> BatchFn<K, V>
    for &mut BatchFnWrapper<K, V, C>
{
    async fn load(&mut self, keys: &[K]) -> HashMap<K, V> {
        let values = self.inner.load_fn(keys).await;
        let (values, err) = check_values(keys, values);
        self.error = err;
        values
    }
}

#[async_trait]
impl<K: CacheKey, V: CacheVal, C: CachedLoader<K, V>> BatchFn<K, V>
    for &mut BatchFnWrapperCached<K, V, C>
{
    async fn load(&mut self, keys: &[K]) -> HashMap<K, V> {
        let values = self.inner.load_fn(keys).await;
        let (values, err) = check_values(keys, values);
        self.error = err;
        values
    }
}

fn check_values<K: CacheKey, V: CacheVal, E: Debug>(
    keys: &[K],
    values: Result<Vec<V>, E>,
) -> (HashMap<K, V>, Option<E>) {
    match values {
        Ok(values) => {
            if keys.len() != values.len() {
                error!(
                    "Keys and values vectors aren't length-equal! keys: {:?} ;;; values: {:?}",
                    &keys, &values
                );
            }
            (keys.iter().cloned().zip(values).collect(), None)
        }
        Err(e) => {
            let placeholder_value = V::default();
            // even on error we need to fill cache with some values
            // because dataloader is sad when they are not present
            (
                keys.iter()
                    .cloned()
                    .map(|k| (k, placeholder_value.clone()))
                    .collect(),
                Some(e),
            )
        }
    }
}
