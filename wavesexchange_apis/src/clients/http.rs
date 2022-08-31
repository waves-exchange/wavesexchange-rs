use crate::{error, ApiResult, BaseApi};
use futures::{future::BoxFuture, Future};
use reqwest::{Client, ClientBuilder, Error as ReqError, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::borrow::Cow;
use std::collections::HashMap;
use std::marker::PhantomData;
use wavesexchange_log::debug;

#[derive(Clone, Debug)]
pub struct HttpClient<A: BaseApi> {
    base_url: Option<String>,
    client: Client,
    _pd: PhantomData<A>,
}

impl<A: BaseApi> HttpClient<A> {
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

    pub fn http_get(&self, url: impl Into<String>) -> RequestBuilder {
        self.client.get(self.prepare_url(url))
    }

    pub fn http_post(&self, url: impl Into<String>) -> RequestBuilder {
        self.client.post(self.prepare_url(url))
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn base_url(&self) -> Cow<'_, str> {
        match &self.base_url {
            Some(s) => Cow::Borrowed(s),
            None => Cow::Owned(String::new()),
        }
    }

    pub async fn do_request(
        &self,
        req: RequestBuilder,
        req_info: impl Into<String>,
    ) -> ApiResult<Response> {
        let req_info = req_info.into();
        let request = req.build().unwrap();
        let method = request.method().as_str();
        let url = request.url().as_str();
        let log_method_url = format!("{method} {url}");

        debug!("requesting '{}', url: {}", req_info, log_method_url);

        let req_start_time = chrono::Utc::now();
        let resp = self
            .client
            .execute(request)
            .await
            .map_err(|err| error::request_failed(err, &req_info))?;

        let req_end_time = chrono::Utc::now();
        debug!(
            "request '{}' took {:?}ms, status: {:?}",
            req_info,
            (req_end_time - req_start_time).num_milliseconds(),
            resp.status(),
        );
        Ok(resp)
    }

    pub(crate) fn create_req_handler<T: DeserializeOwned>(
        &self,
        req: RequestBuilder,
        req_info: impl Into<String> + Clone + Send,
    ) -> WXRequestHandler<A, T> {
        WXRequestHandler::from_request(self, req, req_info)
    }
}

pub struct HttpClientBuilder<A: BaseApi> {
    base_url: Option<String>,
    builder: ClientBuilder,
    _pd: PhantomData<A>,
}

impl<A: BaseApi> HttpClientBuilder<A> {
    pub fn new() -> Self {
        let this = HttpClientBuilder {
            base_url: None,
            builder: ClientBuilder::new(),
            _pd: PhantomData,
        };
        this.with_reqwest_builder(|b| b.pool_max_idle_per_host(1))
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn with_reqwest_builder(
        mut self,
        builder: impl Fn(ClientBuilder) -> ClientBuilder,
    ) -> Self {
        self.builder = builder(self.builder);
        self
    }

    pub fn try_build(self) -> Result<HttpClient<A>, ReqError> {
        Ok(HttpClient {
            base_url: self.base_url,
            client: self.builder.build()?,
            _pd: PhantomData,
        })
    }

    pub fn build(self) -> HttpClient<A> {
        self.try_build().unwrap()
    }
}

#[derive(PartialEq, Eq, Hash)]
pub enum StatusCodes {
    Concrete(StatusCode),
    Other,
}

impl From<StatusCode> for StatusCodes {
    fn from(s: StatusCode) -> Self {
        StatusCodes::Concrete(s)
    }
}

type StatusHandler<T> = Box<dyn FnOnce(Response) -> BoxFuture<'static, ApiResult<T>> + Send>;

/// Optional helper struct for handling requests-responses
///
/// ```no_run
/// HttpClient::create_req_handler(self, self.http_get("search"), "search in my service")
///      // 200 OK has a default handler, you don't need to set it up explicitly all the time.
///      // Same for other statuses without explicit handlers (they have another default handler).
///     .handle_status_code(
///         StatusCode::NOT_FOUND,
///         |resp| async { resp.text().await.unwrap_or("not found!") }
///     )
///     .execute()
/// ```
pub(crate) struct WXRequestHandler<'cli, A, T>
where
    A: BaseApi,
    T: DeserializeOwned,
{
    client: &'cli HttpClient<A>,
    req: RequestBuilder,
    req_info: String,
    status_handlers: HashMap<StatusCodes, StatusHandler<T>>,
}

impl<'cli, A, T> WXRequestHandler<'cli, A, T>
where
    A: BaseApi,
    T: DeserializeOwned,
{
    pub fn from_request(
        client: &'cli HttpClient<A>,
        req: RequestBuilder,
        req_info: impl Into<String>,
    ) -> Self {
        let this = Self {
            client,
            req,
            req_info: req_info.into(),
            status_handlers: HashMap::new(),
        };
        this.set_default_handlers()
    }

    pub fn handle_status_code<Fut>(
        mut self,
        code: impl Into<StatusCodes>,
        handler: impl FnOnce(Response) -> Fut + Send + 'static,
    ) -> Self
    where
        Fut: Future<Output = ApiResult<T>> + Send + 'static,
    {
        self.status_handlers
            .insert(code.into(), Box::new(move |resp| Box::pin(handler(resp))));
        self
    }

    fn set_default_handlers(self) -> Self {
        let req_info = self.req_info.clone();
        let req_info_ = req_info.clone();
        self.handle_status_code(
            StatusCodes::Concrete(StatusCode::OK),
            move |resp| async move {
                let response = resp
                    .text()
                    .await
                    .map_err(|err| error::request_failed(err, &req_info))?;
                serde_json::from_str(&response)
                    .map_err(|err| error::json_error(err.to_string(), req_info, response))
            },
        )
        .handle_status_code(StatusCodes::Other, move |resp| async move {
            Err(error::invalid_status(resp, req_info_).await)
        })
    }

    pub async fn execute(mut self) -> ApiResult<T> {
        let resp = self.client.do_request(self.req, self.req_info).await?;
        let status = resp.status();
        let handler =
            if let Some(handler) = self.status_handlers.remove(&StatusCodes::Concrete(status)) {
                handler
            } else if let Some(handler) = self.status_handlers.remove(&StatusCodes::Other) {
                handler
            } else {
                // if invariants above are not satisfied, then something really bad happened
                unreachable!("No appropriate handler for status {status} found");
            };
        handler(resp).await
    }
}
