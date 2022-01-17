use crate::BaseApi;
use reqwest::{Client, ClientBuilder, Error as ReqError, RequestBuilder};
use std::{marker::PhantomData, ops::Deref};

#[derive(Clone)]
pub struct HttpClient<A: BaseApi<Self>> {
    base_url: Option<String>,
    client: Client,
    api: Option<A>,
}

impl<A: BaseApi<Self>> HttpClient<A> {
    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn builder() -> HttpClientBuilder<A> {
        HttpClientBuilder::new()
    }

    pub fn from_base_url(url: impl Into<String>) -> Self {
        HttpClientBuilder::new().with_base_url(url).build()
    }

    fn prepare_url(&self, url: impl Into<String>) -> String {
        match &self.base_url {
            Some(u) => format!("{}/{}", u, url.into()),
            None => url.into(),
        }
    }

    pub fn get(&self, url: impl Into<String>) -> RequestBuilder {
        self.client.get(self.prepare_url(url))
    }

    pub fn post(&self, url: impl Into<String>) -> RequestBuilder {
        self.client.post(self.prepare_url(url))
    }

    /// `self.client` is private to prevent ambiguation in `self.get` vs `self.client.get`
    /// so use this if you really need inner reqwest::Client
    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn base_url(&self) -> String {
        match &self.base_url {
            Some(s) => s.clone(),
            None => String::new(),
        }
    }
}

pub struct HttpClientBuilder<A: BaseApi<HttpClient<A>>> {
    base_url: Option<String>,
    builder: ClientBuilder,
    _pd: PhantomData<A>,
}

impl<A: BaseApi<HttpClient<A>>> HttpClientBuilder<A> {
    pub fn new() -> Self {
        HttpClientBuilder {
            base_url: None,
            builder: ClientBuilder::new(),
            _pd: PhantomData,
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.builder = self.builder.user_agent(user_agent.into());
        self
    }

    pub fn try_build(mut self) -> Result<HttpClient<A>, ReqError> {
        self.builder = self.builder.pool_max_idle_per_host(1);
        let mut client = HttpClient {
            base_url: self.base_url,
            client: self.builder.build()?,
            api: None,
        };
        client.api = Some(A::new(&client));
        Ok(client)
    }

    pub fn build(self) -> HttpClient<A> {
        self.try_build().unwrap()
    }
}

impl<A: BaseApi<Self>> Deref for HttpClient<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        self.api.as_ref().unwrap()
    }
}
