use crate::{error, BaseApi, Error};
use futures::{future::BoxFuture, Future};
use reqwest::{Client, ClientBuilder, Error as ReqError, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::marker::PhantomData;
use wavesexchange_log::{debug, trace};

#[derive(Clone)]
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

    pub async fn do_request(
        &self,
        req: RequestBuilder,
        req_info: impl Into<String>,
    ) -> Result<Response, Error> {
        let req_info = req_info.into();
        let request = req.build().unwrap();
        let method = request.method().as_str();
        let url = request.url().as_str();
        let log_method_url = format!("{method} {url}");

        trace!("performing request '{}', url: {}", req_info, log_method_url);

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

    pub(crate) fn create_req_handler<T: DeserializeOwned, RS: Into<String> + Clone + Send>(
        &self,
        req: RequestBuilder,
        req_info: RS,
    ) -> WXRequestHandler<A, T, RS> {
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

/// Optional helper struct for handling requests-responses
///
/// ```no_run
/// // call from HttpClient's method
/// WXRequestHandler::from_request(self, self.get("search"), "search in my service")
///      // 200 OK has a default handler, you don't need to set it up explicitly all the time.
///      // Same for other statuses without explicit handlers (they have another default handler).
///     .handle_status_code(
///         StatusCode::NOT_FOUND,
///         |resp| async { resp.text().await.unwrap_or("not found!") }
///     )
///     .execute()
/// ```

#[derive(PartialEq, Eq, Hash)]
pub enum StatusCodes {
    Concrete(StatusCode),
    Other,
}

type StatusHandler<T> = Box<dyn Fn(Response) -> BoxFuture<'static, Result<T, Error>> + Send>;

pub(crate) struct WXRequestHandler<
    'cli,
    A: BaseApi,
    T: DeserializeOwned,
    RS: Into<String> + Clone + Send,
> {
    client: &'cli HttpClient<A>,
    req: Option<RequestBuilder>,
    req_info: RS,
    status_handlers: HashMap<StatusCodes, StatusHandler<T>>,
}

impl<'cli, A: BaseApi, T: DeserializeOwned, RS: Into<String> + Clone + Send>
    WXRequestHandler<'cli, A, T, RS>
{
    pub fn from_request(client: &'cli HttpClient<A>, req: RequestBuilder, req_info: RS) -> Self {
        let mut this = Self {
            client,
            req: Some(req),
            req_info,
            status_handlers: HashMap::new(),
        };
        this.set_default_handlers();
        this
    }

    pub fn handle_status_code<Fut>(
        &mut self,
        code: StatusCodes,
        handler: impl Fn(Response) -> Fut + Send + 'static,
    ) -> &mut Self
    where
        Fut: Future<Output = Result<T, Error>> + Send + 'static,
    {
        self.status_handlers
            .insert(code, Box::new(move |resp| Box::pin(handler(resp))));
        self
    }

    fn set_default_handlers(&mut self) {
        let req_info = self.req_info.into();
        self.handle_status_code(StatusCodes::Concrete(StatusCode::OK), |resp| async {
            resp.json()
                .await
                .map_err(|err| error::json_error(err, req_info.clone()))
        });
        self.handle_status_code(StatusCodes::Other, |resp| async {
            Err(error::invalid_status(resp, req_info).await)
        });
    }

    pub async fn execute(&mut self) -> Result<T, Error> {
        let req = self.req.take().unwrap();
        let req_info = self.req_info.clone().into();
        let resp = self.client.do_request(req, req_info).await?;
        let status = resp.status();
        let handler =
            if let Some(handler) = self.status_handlers.get(&StatusCodes::Concrete(status)) {
                handler
            } else if let Some(default_handler) = self.status_handlers.get(&StatusCodes::Other) {
                default_handler
            } else {
                unreachable!()
            };
        handler(resp).await
    }
}
