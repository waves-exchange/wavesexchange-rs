use reqwest::{Client, ClientBuilder, Error as ReqError, RequestBuilder};

#[derive(Clone)]
pub struct HttpClient {
    base_url: Option<String>,
    client: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn builder() -> HttpClientBuilder {
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
}

pub struct HttpClientBuilder {
    base_url: Option<String>,
    builder: ClientBuilder,
}

impl HttpClientBuilder {
    pub fn new() -> Self {
        HttpClientBuilder {
            base_url: None,
            builder: ClientBuilder::new(),
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

    pub fn try_build(mut self) -> Result<HttpClient, ReqError> {
        self.builder = self.builder.pool_max_idle_per_host(1);
        Ok(HttpClient {
            base_url: self.base_url,
            client: self.builder.build()?,
        })
    }

    pub fn build(self) -> HttpClient {
        self.try_build().unwrap()
    }
}

pub trait ApiBaseUrl {
    fn base_url(&self) -> String;
}

impl ApiBaseUrl for HttpClient {
    fn base_url(&self) -> String {
        match &self.base_url {
            Some(s) => s.clone(),
            None => String::new(),
        }
    }
}
