use reqwest::{Client, ClientBuilder, Error as ReqError, RequestBuilder};

#[derive(Clone)]
pub struct HttpClient {
    pub root_url: Option<String>,
    client: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::new()
    }

    pub fn from_root_url(url: impl Into<String>) -> Self {
        HttpClientBuilder::new().with_root_url(url).build()
    }

    fn prepare_url(&self, url: impl Into<String>) -> String {
        match &self.root_url {
            Some(ru) => format!("{}/{}", ru, url.into()),
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
}

pub struct HttpClientBuilder {
    root_url: Option<String>,
    builder: ClientBuilder,
}

impl HttpClientBuilder {
    pub fn new() -> Self {
        HttpClientBuilder {
            root_url: None,
            builder: ClientBuilder::new(),
        }
    }

    pub fn with_root_url(mut self, url: impl Into<String>) -> Self {
        self.root_url = Some(url.into());
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.builder = self.builder.user_agent(user_agent.into());
        self
    }

    pub fn try_build(mut self) -> Result<HttpClient, ReqError> {
        self.builder = self.builder.pool_max_idle_per_host(1);
        Ok(HttpClient {
            root_url: self.root_url,
            client: self.builder.build()?,
        })
    }

    pub fn build(self) -> HttpClient {
        self.try_build().unwrap()
    }
}