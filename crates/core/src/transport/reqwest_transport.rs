use std::time::{Duration, Instant};

use reqwest::blocking::{Client, ClientBuilder};
use reqwest::redirect::Policy;
use reqwest::{Method, NoProxy, Proxy};

use super::certified::CertifiedClients;
use super::reading::{read_response, ReadWatch};
use super::{Cancellation, HttpTransport, StreamProgress, TransportError};
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::settings::{HttpVersion, ProxySettings, RequestSettings};

const USER_AGENT: &str = concat!("Requestor/", env!("CARGO_PKG_VERSION"));
const HOST_LIST_SEPARATOR: &str = ",";
const SECURE_SCHEME_PREFIX: &str = "https://";

pub struct ReqwestTransport {
    client: Client,
    secure_client: Option<Client>,
    certified_clients: CertifiedClients,
    settings: RequestSettings,
}

impl ReqwestTransport {
    pub fn new(settings: &RequestSettings) -> Result<Self, TransportError> {
        let secure_client = match settings.http_version {
            HttpVersion::Http2 => Some(build(settings, ClientBuilder::http2_prior_knowledge)?),
            _ => None,
        };
        let client = match settings.http_version {
            HttpVersion::Http1 => build(settings, ClientBuilder::http1_only)?,
            _ => build(settings, |builder| builder)?,
        };
        Ok(ReqwestTransport {
            client,
            secure_client,
            certified_clients: CertifiedClients::new(settings),
            settings: settings.clone(),
        })
    }

    fn client_for(&self, url: &str) -> Result<&Client, TransportError> {
        if let Some(certified) = self.certified_clients.client_for(url) {
            return certified;
        }
        Ok(match &self.secure_client {
            Some(secure) if url.starts_with(SECURE_SCHEME_PREFIX) => secure,
            _ => &self.client,
        })
    }
}

impl HttpTransport for ReqwestTransport {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        self.send_cancellable(request, &Cancellation::default(), &mut |_| {})
    }
}

impl ReqwestTransport {
    pub fn send_cancellable(
        &self,
        request: PreparedRequest,
        cancellation: &Cancellation,
        on_stream: &mut dyn FnMut(StreamProgress<'_>),
    ) -> Result<HttpResponse, TransportError> {
        let timeout = self.settings.request_timeout();
        self.send_within(request, timeout, cancellation, on_stream)
    }

    pub fn send_within(
        &self,
        request: PreparedRequest,
        timeout: Option<Duration>,
        cancellation: &Cancellation,
        on_stream: &mut dyn FnMut(StreamProgress<'_>),
    ) -> Result<HttpResponse, TransportError> {
        let method = Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|error| TransportError::InvalidRequest(error.to_string()))?;
        let mut builder = self.client_for(&request.url)?.request(method, &request.url);
        for header in &request.headers {
            builder = builder.header(&header.name, &header.value);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        if let Some(timeout) = timeout {
            builder = builder.timeout(timeout);
        }
        let started = Instant::now();
        let response = builder.send().map_err(TransportError::from_reqwest)?;
        let watch = ReadWatch {
            deadline: timeout.map(|timeout| started + timeout),
            cancellation,
        };
        read_response(response, &self.settings, started, watch, on_stream)
    }
}

fn build(
    settings: &RequestSettings,
    version: impl FnOnce(ClientBuilder) -> ClientBuilder,
) -> Result<Client, TransportError> {
    finish(version(configure(Client::builder(), settings)?))
}

pub(super) fn finish(builder: ClientBuilder) -> Result<Client, TransportError> {
    builder
        .build()
        .map_err(|error| TransportError::InvalidSettings(error.without_url().to_string()))
}

pub(super) fn configure(
    builder: ClientBuilder,
    settings: &RequestSettings,
) -> Result<ClientBuilder, TransportError> {
    let mut builder = builder
        .user_agent(USER_AGENT)
        .danger_accept_invalid_certs(!settings.verify_ssl)
        .cookie_store(settings.cookies_enabled)
        .redirect(redirect_policy(settings))
        .timeout(settings.request_timeout());
    if let Some(connect_timeout) = settings.connect_timeout() {
        builder = builder.connect_timeout(connect_timeout);
    }
    match (&settings.proxy, settings.use_system_proxy) {
        (Some(proxy), _) => Ok(builder.proxy(build_proxy(proxy)?)),
        (None, false) => Ok(builder.no_proxy()),
        (None, true) => Ok(builder),
    }
}

fn redirect_policy(settings: &RequestSettings) -> Policy {
    if settings.follow_redirects {
        Policy::limited(settings.max_redirects)
    } else {
        Policy::none()
    }
}

fn build_proxy(settings: &ProxySettings) -> Result<Proxy, TransportError> {
    let mut proxy = Proxy::all(&settings.url)
        .map_err(|error| TransportError::InvalidSettings(error.without_url().to_string()))?;
    if let Some(username) = &settings.username {
        let password = settings
            .password
            .as_ref()
            .map_or("", |secret| secret.expose());
        proxy = proxy.basic_auth(username, password);
    }
    if !settings.bypass_hosts.is_empty() {
        let hosts = settings.bypass_hosts.join(HOST_LIST_SEPARATOR);
        proxy = proxy.no_proxy(NoProxy::from_string(&hosts));
    }
    Ok(proxy)
}
