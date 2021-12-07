mod cacher;
mod loaders;

pub use cached::{SizedCache, TimedCache, TimedSizedCache, UnboundCache};
pub use loaders::{BaseLoader, CachedLoader, NonCachedLoader};

#[macro_use]
extern crate async_trait;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cacher::{CacheKey, CacheVal};
    use std::fmt::Debug;
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

    async fn measure_load<
        E: Debug + Eq,
        K: CacheKey,
        V: CacheVal + Eq,
        C: CachedLoader<K, V, LoadError = E>,
    >(
        loader: &C,
        key: K,
        expected_val: Result<V, E>,
        measure_fn: impl Fn(Duration) -> bool,
    ) -> bool {
        let now = SystemTime::now();
        let result = loader.load(key.clone()).await;
        let elapsed = now.elapsed().unwrap();
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

    #[tokio::test]
    async fn test_timed() {
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<u64, String> for Loadable {
            type Cache = TimedCache<u64, String>;
            type LoadError = ();

            async fn load_fn(&mut self, keys: &[u64]) -> Result<Vec<String>, Self::LoadError> {
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
    async fn test_sized() {
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<isize, String> for Loadable {
            type Cache = SizedCache<isize, String>;
            type LoadError = ();

            async fn load_fn(&mut self, keys: &[isize]) -> Result<Vec<String>, Self::LoadError> {
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
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<isize, Option<String>> for Loadable {
            type Cache = UnboundCache<isize, Option<String>>;
            type LoadError = ();

            async fn load_fn(
                &mut self,
                keys: &[isize],
            ) -> Result<Vec<Option<String>>, Self::LoadError> {
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
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl NonCachedLoader<i64, i64> for Loadable {
            type LoadError = ();

            async fn load_fn(&mut self, keys: &[i64]) -> Result<Vec<i64>, Self::LoadError> {
                sleep(SLEEP_DUR).await;
                Ok(keys.to_vec())
            }
        }

        let loader = Loadable {};
        assert!(measure_load(&loader, 5555, Ok(5555), is_not_cached).await);

        //not caching errors
        assert!(measure_load(&loader, 5555, Ok(5555), is_not_cached).await);
    }

    #[tokio::test]
    async fn test_error_during_loading() {
        #[derive(Clone)]
        struct Loadable;

        #[async_trait]
        impl CachedLoader<isize, ()> for Loadable {
            type Cache = UnboundCache<isize, ()>;
            type LoadError = String;

            async fn load_fn(&mut self, _keys: &[isize]) -> Result<Vec<()>, Self::LoadError> {
                sleep(SLEEP_DUR).await;
                Err("oh, no!".to_string())
            }

            fn init_cache() -> Self::Cache {
                UnboundCache::new()
            }
        }

        let loader = Loadable {};
        assert!(measure_load(&loader, 12345, Err("oh, no!".to_string()), is_not_cached).await);

        //not caching errors
        assert!(measure_load(&loader, 12345, Err("oh, no!".to_string()), is_not_cached).await);
    }
}
