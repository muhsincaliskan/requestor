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

pub struct ReqwestTransport {
    client: Client,
    max_response_bytes: usize,
}

impl ReqwestTransport {
    pub fn new(settings: &RequestSettings) -> Result<Self, TransportError> {
        let client = configure(Client::builder(), settings)?
            .build()
            .map_err(|error| TransportError::InvalidSettings(error.without_url().to_string()))?;
        Ok(ReqwestTransport {
            client,
            max_response_bytes: settings.max_response_bytes,
        })
    }
}

impl HttpTransport for ReqwestTransport {
    fn send(&self, request: &PreparedRequest) -> Result<HttpResponse, TransportError> {
        let method = Method::from_bytes(request.method.as_str().as_bytes())
            .map_err(|error| TransportError::InvalidRequest(error.to_string()))?;
        let mut builder = self.client.request(method, &request.url);
        for header in &request.headers {
            builder = builder.header(&header.name, &header.value);
        }
        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }
        let started = Instant::now();
        let response = builder.send().map_err(TransportError::from_reqwest)?;
        read_response(response, self.max_response_bytes, started)
    }
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
    builder = match settings.http_version {
        HttpVersion::Auto => builder,
        HttpVersion::Http1 => builder.http1_only(),
        HttpVersion::Http2 => builder.http2_prior_knowledge(),
    };
    match &settings.proxy {
        Some(proxy) => Ok(builder.proxy(build_proxy(proxy)?)),
        None => Ok(builder),
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
    limit: usize,
    started: Instant,
) -> Result<HttpResponse, TransportError> {
    let status = response.status();
    let version = format!("{:?}", response.version());
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| Header::new(name.as_str(), String::from_utf8_lossy(value.as_bytes())))
        .collect();
    let (body, truncated) = read_capped(response, limit)?;
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

fn read_capped(reader: impl Read, limit: usize) -> Result<(Vec<u8>, bool), TransportError> {
    let mut body = Vec::new();
    let mut capped = reader.take(limit as u64 + 1);
    capped
        .read_to_end(&mut body)
        .map_err(|error| TransportError::from_io(&error))?;
    let truncated = body.len() > limit;
    body.truncate(limit);
    Ok((body, truncated))
}
