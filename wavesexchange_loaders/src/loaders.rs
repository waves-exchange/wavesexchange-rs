use crate::cacher::{CacheBounds, CacheKey, CacheVal, Cacher, SharedObj};
use dataloader::{cached, non_cached, BatchFn};
use std::collections::HashMap;
use std::fmt::Debug;
use wavesexchange_log::error;

pub type InnerLoader<'b, LDR> = non_cached::Loader<
    <LDR as NonCachedLoader>::K,
    <LDR as NonCachedLoader>::V,
    &'b mut BatchFnWrapper<LDR>,
>;

pub type InnerCachedLoader<'b, LDR> = cached::Loader<
    <LDR as CachedLoader>::K,
    <LDR as CachedLoader>::V,
    &'b mut BatchFnWrapperCached<LDR>,
    &'b mut Cacher<
        <LDR as CachedLoader>::K,
        <LDR as CachedLoader>::V,
        <LDR as CachedLoader>::Cache,
    >,
>;
#[async_trait]
pub trait NonCachedLoader: SharedObj + Clone {
    type K: CacheKey;
    type V: CacheVal;
    type LoadError: Debug + Send;

    /// Modify loader params
    #[inline]
    fn init_loader(loader: InnerLoader<Self>) -> InnerLoader<Self> {
        loader
    }

    /// Setup loader function
    async fn load_fn(&mut self, keys: &[Self::K]) -> Result<Vec<Self::V>, Self::LoadError>;

    /// Don't override this
    async fn load(&self, key: Self::K) -> Result<Self::V, Self::LoadError> {
        let mut batch_wrapper = BatchFnWrapper::new(self.clone());
        let loader = InnerLoader::new(&mut batch_wrapper);
        let result = Self::init_loader(loader).load(key.clone()).await;
        match batch_wrapper.error {
            Some(e) => Err(e),
            None => Ok(result),
        }
    }
}

#[async_trait]
pub trait CachedLoader: SharedObj + Clone {
    type K: CacheKey;
    type V: CacheVal;
    type Cache: CacheBounds<Self::K, Self::V>;
    type LoadError: Debug + Send;

    /// Modify loader params
    #[inline]
    fn init_loader(loader: InnerCachedLoader<Self>) -> InnerCachedLoader<Self> {
        loader
    }

    /// Setup loader function
    async fn load_fn(&mut self, keys: &[Self::K]) -> Result<Vec<Self::V>, Self::LoadError>;

    /// Setup cache type
    fn init_cache() -> Self::Cache;

    /// Determine values that will be cached, i.e. only Some(...), but not None
    #[inline]
    fn cache_strategy(_: &Self::K, _: &Self::V) -> bool {
        true
    }

    /// Don't override this
    async fn load(&self, key: Self::K) -> Result<Self::V, Self::LoadError> {
        let mut batch_wrapper = BatchFnWrapperCached::new(self.clone());
        let cache = Cacher::get_or_init(Self::init_cache, Self::cache_strategy).await;
        let mut cache_lock = cache.lock().await;
        let loader = InnerCachedLoader::with_cache(&mut batch_wrapper, &mut *cache_lock);
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
pub struct BatchFnWrapper<L: NonCachedLoader> {
    inner: L,
    error: Option<L::LoadError>,
}
pub struct BatchFnWrapperCached<L: CachedLoader> {
    inner: L,
    error: Option<L::LoadError>,
}

impl<L: NonCachedLoader> BatchFnWrapper<L> {
    pub fn new(inner: L) -> Self {
        BatchFnWrapper { inner, error: None }
    }
}

impl<L: CachedLoader> BatchFnWrapperCached<L> {
    pub fn new(inner: L) -> Self {
        BatchFnWrapperCached { inner, error: None }
    }
}

#[async_trait]
impl<L: NonCachedLoader> BatchFn<L::K, L::V> for &mut BatchFnWrapper<L> {
    async fn load(&mut self, keys: &[L::K]) -> HashMap<L::K, L::V> {
        let values = self.inner.load_fn(keys).await;
        let (values, err) = check_values(keys, values);
        self.error = err;
        values
    }
}

#[async_trait]
impl<L: CachedLoader> BatchFn<L::K, L::V> for &mut BatchFnWrapperCached<L> {
    async fn load(&mut self, keys: &[L::K]) -> HashMap<L::K, L::V> {
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
            // because dataloader is sad when they are not present,
            // they'll be deleted soon
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
