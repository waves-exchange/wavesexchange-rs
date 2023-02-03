/*!
Combines `dataloader` and `cached` libs.

`wavesexchange_loaders` provides interfaces to create cached or non-cached dataloaders.

Usage example:
```
mod my_loader {
    use async_trait::async_trait;
    use wavesexchange_loaders::{CachedLoader, TimedCache};

    pub type MyError = ();

    #[derive(Clone)]
    pub struct MyLoader;

    #[async_trait]
    impl CachedLoader<i32, String> for MyLoader {
        type Cache = TimedCache<i32, String>;
        type Error = MyError;

        // Note: vec of values and array of keys must have the same size
        async fn load_fn(&mut self, keys: &[i32]) -> Result<Vec<String>, Self::Error> {
            Ok(keys.into_iter().map(|k| format!("answer: {}", k)).collect())
        }

        // keys will be cached for 3 seconds
        fn init_cache() -> Self::Cache {
            TimedCache::with_lifespan(3)
        }
    }
}

// Usage
use my_loader::{MyLoader, MyError};
use wavesexchange_loaders::{Loader, LoaderError};

# tokio_test::block_on(async {
let s = MyLoader {};
// result type is specified here just for reference, it can be inferred by the compiler
let result: Result<String, LoaderError<MyError>> = s.load(42).await;
assert_eq!(result.ok(), Some("answer: 42".to_string()));
# })
```
*/

mod cacher;
mod error;
mod loaders;

pub use cached::{SizedCache, TimedCache, TimedSizedCache, UnboundCache};
pub use error::LoaderError;
pub use loaders::{CachedLoader, InnerCachedLoader, InnerLoader, Loader, NonCachedLoader};

// Reexport cached
pub use cached;

#[macro_use]
extern crate async_trait;

#[cfg(test)]
mod tests {
    use super::LoaderError;
    use crate::cacher::{CacheKey, CacheVal};
    use std::fmt::Debug;
    use std::future::Future;
    use std::time::{Duration, Instant};
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

    fn always_valid_duration(_: Duration) -> bool {
        true
    }

    async fn _measure<E: Debug + PartialEq + Eq, K: CacheKey, V: CacheVal + Eq>(
        key: K,
        expected_val: Result<V, LoaderError<E>>,
        test_fn: impl Future<Output = Result<V, LoaderError<E>>>,
        measure_fn: impl Fn(Duration) -> bool,
    ) -> bool {
        let now = Instant::now();
        let result = test_fn.await;
        let elapsed = now.elapsed();
        let measurement_is_ok = measure_fn(elapsed);
        let values_are_eq = match (&result, &expected_val) {
            (Ok(r), Ok(ev)) => r == ev,
            (Err(r_err), Err(ev_err)) => r_err == ev_err,
            _ => false,
        };
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

    async fn measure_load_noncached<
        E: Debug + Send + Eq,
        K: CacheKey,
        V: CacheVal + Eq,
        L: super::NonCachedLoader<K, V, Error = E>,
    >(
        loader: &L,
        key: K,
        expected_val: Result<V, LoaderError<E>>,
        measure_fn: impl Fn(Duration) -> bool,
    ) -> bool {
        use super::Loader;
        _measure(key.clone(), expected_val, loader.load(key), measure_fn).await
    }

    async fn measure_load<
        E: Debug + Send + Eq,
        K: CacheKey,
        V: CacheVal + Eq,
        L: super::CachedLoader<K, V, Error = E>,
    >(
        loader: &L,
        key: K,
        expected_val: Result<V, LoaderError<E>>,
        measure_fn: impl Fn(Duration) -> bool,
    ) -> bool {
        use super::Loader;
        _measure(key.clone(), expected_val, loader.load(key), measure_fn).await
    }

    #[tokio::test]
    async fn test_timed_cache() {
        use super::{CachedLoader, TimedCache};

        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<u64, String> for Loadable {
            type Cache = TimedCache<u64, String>;
            type Error = ();

            async fn load_fn(&mut self, keys: &[u64]) -> Result<Vec<String>, Self::Error> {
                sleep(SLEEP_DUR).await;
                Ok(keys.into_iter().map(|k| format!("num: {}", k)).collect())
            }

            fn init_cache() -> Self::Cache {
                TimedCache::with_lifespan(3) //seconds to persist in cache
            }
        }

        let loader = Loadable {};
        assert!(measure_load(&loader, 4, Ok("num: 4".to_string()), is_not_cached).await);

        //value is cached
        assert!(measure_load(&loader, 4, Ok("num: 4".to_string()), is_cached).await);
        sleep(Duration::from_secs(3)).await;

        //value is dropped due to ttl
        assert!(measure_load(&loader, 4, Ok("num: 4".to_string()), is_not_cached).await);
    }

    #[tokio::test]
    async fn test_sized_cache() {
        use super::{CachedLoader, SizedCache};

        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<isize, String> for Loadable {
            type Cache = SizedCache<isize, String>;
            type Error = ();

            async fn load_fn(&mut self, keys: &[isize]) -> Result<Vec<String>, Self::Error> {
                sleep(SLEEP_DUR).await;
                Ok(keys.into_iter().map(|k| format!("num: {}", k)).collect())
            }

            fn init_cache() -> Self::Cache {
                SizedCache::with_size(1)
            }
        }

        let loader = Loadable {};
        assert!(
            measure_load(
                &loader,
                -65535,
                Ok("num: -65535".to_string()),
                is_not_cached
            )
            .await
        );

        //value is cached
        assert!(measure_load(&loader, -65535, Ok("num: -65535".to_string()), is_cached).await);

        //rewriting the only available cache cell
        assert!(measure_load(&loader, -4, Ok("num: -4".to_string()), is_not_cached).await);
        assert!(measure_load(&loader, -4, Ok("num: -4".to_string()), is_cached).await);

        //first value is dropped because there can be only one
        assert!(
            measure_load(
                &loader,
                -65535,
                Ok("num: -65535".to_string()),
                is_not_cached
            )
            .await
        );
    }

    #[tokio::test]
    async fn test_cache_strategy() {
        use super::{CachedLoader, UnboundCache};

        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<isize, Option<String>> for Loadable {
            type Cache = UnboundCache<isize, Option<String>>;
            type Error = ();

            async fn load_fn(
                &mut self,
                keys: &[isize],
            ) -> Result<Vec<Option<String>>, Self::Error> {
                sleep(SLEEP_DUR).await;
                Ok(keys
                    .into_iter()
                    .map(|k| {
                        if k % 2 == 0 {
                            // loader fn returns string only with even numbers
                            Some(format!("num: {}", k))
                        } else {
                            None
                        }
                    })
                    .collect())
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
        assert!(measure_load(&loader, 28, Ok(Some("num: 28".to_string())), is_not_cached).await);

        //is cached
        assert!(measure_load(&loader, 28, Ok(Some("num: 28".to_string())), is_cached).await);

        //odd number
        assert!(measure_load(&loader, 5, Ok(None), is_not_cached).await);

        //is not cached
        assert!(measure_load(&loader, 5, Ok(None), is_not_cached).await);
    }

    #[tokio::test]
    async fn test_no_cache() {
        use super::{InnerLoader, NonCachedLoader};

        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl NonCachedLoader<i32, i64> for Loadable {
            type Error = ();

            async fn load_fn(&mut self, keys: &[i32]) -> Result<Vec<i64>, Self::Error> {
                sleep(SLEEP_DUR).await;
                Ok(keys.into_iter().cloned().map(i64::from).collect())
            }

            fn init_loader(loader: InnerLoader<i32, i64, Self>) -> InnerLoader<i32, i64, Self> {
                loader.with_max_batch_size(2)
            }
        }

        let loader = Loadable {};
        assert!(measure_load_noncached(&loader, 5555, Ok(5555), is_not_cached).await);
        assert!(measure_load_noncached(&loader, 5555, Ok(5555), is_not_cached).await);
    }

    #[tokio::test]
    async fn test_error_during_loading() {
        use super::{CachedLoader, UnboundCache};

        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<isize, ()> for Loadable {
            type Cache = UnboundCache<isize, ()>;
            type Error = String;

            async fn load_fn(&mut self, _keys: &[isize]) -> Result<Vec<()>, Self::Error> {
                sleep(SLEEP_DUR).await;
                Err("oh, no!".to_string())
            }

            fn init_cache() -> Self::Cache {
                UnboundCache::new()
            }
        }

        let loader = Loadable {};
        assert!(
            measure_load(
                &loader,
                12345,
                Err(LoaderError::Other("oh, no!".to_string())),
                is_not_cached
            )
            .await
        );

        //not caching errors
        assert!(
            measure_load(
                &loader,
                12345,
                Err(LoaderError::Other("oh, no!".to_string())),
                is_not_cached
            )
            .await
        );
    }

    #[tokio::test]
    async fn test_load_fn_missed_some_values() {
        use super::NonCachedLoader;

        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl NonCachedLoader<isize, ()> for Loadable {
            type Error = String;

            async fn load_fn(&mut self, _keys: &[isize]) -> Result<Vec<()>, Self::Error> {
                Ok(vec![])
            }
        }

        let loader = Loadable {};
        assert!(
            measure_load_noncached(
                &loader,
                12345,
                Err(LoaderError::MissingValues(
                    "Keys and values vectors aren't length-equal! keys: [12345] ;;; values: []"
                        .to_string()
                )),
                always_valid_duration
            )
            .await
        );
    }
}
