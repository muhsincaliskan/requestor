use std::io::Read;
use std::time::Instant;

use reqwest::blocking::{Client, ClientBuilder, Response};
use reqwest::redirect::Policy;
use reqwest::{Method, NoProxy, Proxy};

use super::{HttpTransport, TransportError};
use crate::header::Header;
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::settings::{HttpVersion, ProxySettings, RequestSettings};

const USER_AGENT: &str = concat!("Requestor/", env!("CARGO_PKG_VERSION"));
const HOST_LIST_SEPARATOR: &str = ",";
const SECURE_SCHEME_PREFIX: &str = "https://";
const READ_CHUNK_BYTES: usize = 16 * 1024;
const INITIAL_BODY_CAPACITY: usize = 64 * 1024;

pub struct ReqwestTransport {
    client: Client,
    secure_client: Option<Client>,
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
            settings: settings.clone(),
        })
    }

    fn client_for(&self, url: &str) -> &Client {
        match &self.secure_client {
            Some(secure) if url.starts_with(SECURE_SCHEME_PREFIX) => secure,
            _ => &self.client,
        }
    }
}

impl HttpTransport for ReqwestTransport {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        let method = Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|error| TransportError::InvalidRequest(error.to_string()))?;
        let mut builder = self.client_for(&request.url).request(method, &request.url);
        for header in &request.headers {
            builder = builder.header(&header.name, &header.value);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        let started = Instant::now();
        let response = builder.send().map_err(TransportError::from_reqwest)?;
        read_response(response, &self.settings, started)
    }
}

fn build(
    settings: &RequestSettings,
    version: impl FnOnce(ClientBuilder) -> ClientBuilder,
) -> Result<Client, TransportError> {
    version(configure(Client::builder(), settings)?)
        .build()
        .map_err(|error| TransportError::InvalidSettings(error.without_url().to_string()))
}

fn configure(
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

fn read_response(
    response: Response,
    settings: &RequestSettings,
    started: Instant,
) -> Result<HttpResponse, TransportError> {
    let status = response.status();
    let version = format!("{:?}", response.version());
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| Header::new(name.as_str(), String::from_utf8_lossy(value.as_bytes())))
        .collect();
    let expected = response
        .content_length()
        .map_or(INITIAL_BODY_CAPACITY, |length| {
            usize::try_from(length).unwrap_or(usize::MAX)
        });
    let limit = settings.response_limit();
    let deadline = settings.request_timeout().map(|timeout| started + timeout);
    let (body, truncated) = read_capped(response, limit, expected.min(limit), deadline)?;
    Ok(HttpResponse {
        status: status.as_u16(),
        reason: status.canonical_reason().unwrap_or_default().to_string(),
        version,
        headers,
        body,
        truncated,
        elapsed: started.elapsed(),
    })
}

fn read_capped(
    mut reader: impl Read,
    limit: usize,
    capacity: usize,
    deadline: Option<Instant>,
) -> Result<(Vec<u8>, bool), TransportError> {
    let mut body = Vec::with_capacity(capacity.min(limit));
    let mut chunk = vec![0u8; READ_CHUNK_BYTES];
    loop {
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(TransportError::Timeout);
        }
        let count = reader
            .read(&mut chunk)
            .map_err(|error| TransportError::from_io(&error))?;
        if count == 0 {
            return Ok((body, false));
        }
        let room = limit - body.len();
        if count > room {
            body.extend_from_slice(&chunk[..room]);
            return Ok((body, true));
        }
        body.extend_from_slice(&chunk[..count]);
    }
}
