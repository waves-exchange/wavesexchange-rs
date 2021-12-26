use crate::cacher::{CacheBounds, CacheKey, CacheVal, Cacher, ErrBounds, SharedObj};
use crate::error::LoaderError;
use dataloader::{cached, non_cached, BatchFn};
use std::collections::HashMap;
use std::marker::PhantomData;

pub type InnerLoader<'b, K, V, L> = non_cached::Loader<
    K,
    V,
    &'b mut BatchFnWrapper<K, V, L, <L as NonCachedLoader<K, V>>::Error, false>,
>;

pub type InnerCachedLoader<'b, K, V, L> = cached::Loader<
    K,
    V,
    &'b mut BatchFnWrapper<K, V, L, <L as CachedLoader<K, V>>::Error, true>,
    &'b mut Cacher<K, V, <L as CachedLoader<K, V>>::Cache>,
>;

#[async_trait]
pub trait NonCachedLoader<K: CacheKey, V: CacheVal>: SharedObj + Clone {
    type Error: ErrBounds;

    /// Modify loader params
    #[inline]
    fn init_loader(loader: InnerLoader<K, V, Self>) -> InnerLoader<K, V, Self> {
        loader
    }

    /// Setup loader function
    async fn load_fn(&mut self, keys: &[K]) -> Result<Vec<V>, Self::Error>;
}

#[async_trait]
pub trait CachedLoader<K: CacheKey, V: CacheVal>: SharedObj + Clone {
    type Cache: CacheBounds<K, V>;
    type Error: ErrBounds;

    /// Modify loader params
    #[inline]
    fn init_loader(loader: InnerCachedLoader<K, V, Self>) -> InnerCachedLoader<K, V, Self> {
        loader
    }

    /// Setup loader function
    async fn load_fn(&mut self, keys: &[K]) -> Result<Vec<V>, Self::Error>;

    /// Setup cache params
    fn init_cache() -> Self::Cache;

    /// Determine values that will be cached, i.e. only Some(...), but not None
    #[inline]
    fn cache_strategy(_: &K, _: &V) -> bool {
        true
    }
}

/// Use this trait only for import, no need to impl it
#[async_trait]
pub trait Loader<K, V, E: ErrBounds, const HAS_CACHE: bool> {
    async fn load(&self, key: K) -> Result<V, LoaderError<E>>;

    async fn load_many(&self, keys: Vec<K>) -> Result<HashMap<K, V>, LoaderError<E>>;
}

#[async_trait]
impl<K, V, L> Loader<K, V, L::Error, false> for L
where
    K: CacheKey,
    V: CacheVal,
    L: NonCachedLoader<K, V>,
{
    async fn load(&self, key: K) -> Result<V, LoaderError<L::Error>> {
        let mut batch_wrapper = BatchFnWrapper::<_, _, _, _, false>::new(self.clone());
        let loader = InnerLoader::new(&mut batch_wrapper);
        let result = Self::init_loader(loader).try_load(key).await;
        parse_loader_result(result, batch_wrapper.error)
    }

    async fn load_many(&self, keys: Vec<K>) -> Result<HashMap<K, V>, LoaderError<L::Error>> {
        let mut batch_wrapper = BatchFnWrapper::<_, _, _, _, false>::new(self.clone());
        let loader = InnerLoader::new(&mut batch_wrapper);
        let result = Self::init_loader(loader).try_load_many(keys).await;
        parse_loader_result(result, batch_wrapper.error)
    }
}

#[async_trait]
impl<K, V, L> Loader<K, V, L::Error, true> for L
where
    K: CacheKey,
    V: CacheVal,
    L: CachedLoader<K, V>,
{
    async fn load(&self, key: K) -> Result<V, LoaderError<L::Error>> {
        let mut batch_wrapper = BatchFnWrapper::<_, _, _, _, true>::new(self.clone());
        let cache = Cacher::get_or_init(Self::init_cache, Self::cache_strategy).await;
        let mut cache_lock = cache.lock().await;
        let loader = InnerCachedLoader::with_cache(&mut batch_wrapper, &mut *cache_lock);
        let result = Self::init_loader(loader).try_load(key.clone()).await;
        if batch_wrapper.error.is_some() {
            cache_lock.add_key_to_drop(&key);
        }
        cache_lock.cleanup();
        parse_loader_result(result, batch_wrapper.error)
    }

    async fn load_many(&self, keys: Vec<K>) -> Result<HashMap<K, V>, LoaderError<L::Error>> {
        let mut batch_wrapper = BatchFnWrapper::<_, _, _, _, true>::new(self.clone());
        let cache = Cacher::get_or_init(Self::init_cache, Self::cache_strategy).await;
        let mut cache_lock = cache.lock().await;
        let loader = InnerCachedLoader::with_cache(&mut batch_wrapper, &mut *cache_lock);
        let result = Self::init_loader(loader).try_load_many(keys.clone()).await;
        if batch_wrapper.error.is_some() {
            keys.iter().for_each(|key| cache_lock.add_key_to_drop(key));
        }
        cache_lock.cleanup();
        parse_loader_result(result, batch_wrapper.error)
    }
}

pub struct BatchFnWrapper<K, V, C, E: ErrBounds, const HAS_CACHE: bool> {
    inner: C,
    error: Option<LoaderError<E>>,
    _pd: (PhantomData<K>, PhantomData<V>),
}

impl<K: CacheKey, V: CacheVal, L: NonCachedLoader<K, V>> BatchFnWrapper<K, V, L, L::Error, false> {
    fn new(inner: L) -> Self {
        BatchFnWrapper {
            inner,
            error: None,
            _pd: (PhantomData, PhantomData),
        }
    }
}

impl<K: CacheKey, V: CacheVal, L: CachedLoader<K, V>> BatchFnWrapper<K, V, L, L::Error, true> {
    fn new(inner: L) -> Self {
        BatchFnWrapper {
            inner,
            error: None,
            _pd: (PhantomData, PhantomData),
        }
    }
}

#[async_trait]
impl<K: CacheKey, V: CacheVal, C: NonCachedLoader<K, V>> BatchFn<K, V>
    for &mut BatchFnWrapper<K, V, C, C::Error, false>
{
    async fn load(&mut self, keys: &[K]) -> HashMap<K, V> {
        let values = self.inner.load_fn(keys).await;
        check_values(keys, values).unwrap_or_else(|e| {
            self.error = Some(e);
            HashMap::new()
        })
    }
}

#[async_trait]
impl<K: CacheKey, V: CacheVal, C: CachedLoader<K, V>> BatchFn<K, V>
    for &mut BatchFnWrapper<K, V, C, C::Error, true>
{
    async fn load(&mut self, keys: &[K]) -> HashMap<K, V> {
        let values = self.inner.load_fn(keys).await;
        check_values(keys, values).unwrap_or_else(|e| {
            self.error = Some(e);
            HashMap::new()
        })
    }
}

fn check_values<K: CacheKey, V: CacheVal, E: ErrBounds>(
    keys: &[K],
    values: Result<Vec<V>, E>,
) -> Result<HashMap<K, V>, LoaderError<E>> {
    values.map_err(LoaderError::Other).and_then(|values| {
        if keys.len() != values.len() {
            Err(LoaderError::MissingValues(format!(
                "Keys and values vectors aren't length-equal! keys: {:?} ;;; values: {:?}",
                &keys, &values
            )))
        } else {
            Ok(keys.iter().cloned().zip(values).collect())
        }
    })
}

fn parse_loader_result<R, E: ErrBounds>(
    result: Result<R, std::io::Error>,
    err: Option<LoaderError<E>>,
) -> Result<R, LoaderError<E>> {
    match err {
        Some(e) => Err(e),
        None => result.map_err(|e| LoaderError::MissingValues(e.to_string())),
    }
}
