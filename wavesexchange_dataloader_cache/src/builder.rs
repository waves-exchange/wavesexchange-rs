use std::{collections::HashMap, future::Future, hash::Hash, marker::PhantomData, time::Duration};

use dataloader::{
    cached::{Cache, Loader},
    BatchFn,
};

pub struct DataLoaderBuilder<K, V, FutV, F>
where
    K: Eq + Hash + Clone,
    V: Clone,
    F: Fn(Vec<K>) -> FutV,
    FutV: Future<Output = HashMap<K, V>>,
{
    load_batch: F,
    size: usize,
    ttl: Duration,
}

impl<K, V, FutV, F> DataLoaderBuilder<K, V, FutV, F>
where
    K: Eq + Hash + Clone,
    V: Clone,
    FutV: Future<Output = HashMap<K, V>>,
    F: Fn(Vec<K>) -> FutV,
{
    // todo impl IntoIterator
    pub fn new(load_batch: F) -> Self {
        Self {
            load_batch,
            size: 4096,
            ttl: Duration::from_secs(86400),
        }
    }

    pub fn with_ttl<'a>(&'a mut self, ttl: Duration) -> &'a mut Self {
        self.ttl = ttl;
        self
    }

    pub fn build(self) -> Loader<K, V, impl BatchFn<K, V>, impl Cache<Key = K, Val = V>> {
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn dataloader_builder() {
        let load_batch = |keys: Vec<String>| async {
            println!("loading keys {:?}", &keys);
            keys.into_iter()
                .map(|k| (k.clone(), k))
                .collect::<HashMap<_, _>>()
        };
        let loader = DataLoaderBuilder::new(load_batch).build();
        let qwe = loader.load(String::from("qwe")).await;
        assert_eq!(qwe, "qwe");
    }
}
