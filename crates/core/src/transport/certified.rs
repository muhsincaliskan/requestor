use reqwest::blocking::{Client, ClientBuilder};
use reqwest::redirect::{Attempt, Policy};
use reqwest::{Identity, Url};

use super::reqwest_transport::{configure, finish};
use super::TransportError;
use crate::settings::{ClientCertificate, HttpVersion, RequestSettings};

const SECURE_SCHEME: &str = "https";
const TOO_MANY_REDIRECTS: &str = "too many redirects";

pub struct CertifiedClients {
    entries: Vec<(ClientCertificate, Result<Client, TransportError>)>,
}

impl CertifiedClients {
    pub fn new(settings: &RequestSettings) -> Self {
        let entries = settings
            .client_certificates
            .iter()
            .filter(|certificate| certificate.is_complete())
            .map(|certificate| {
                let client = certified_builder(settings, certificate).and_then(finish);
                (certificate.clone(), client)
            })
            .collect();
        CertifiedClients { entries }
    }

    pub fn client_for(&self, url: &str) -> Option<Result<&Client, TransportError>> {
        if self.entries.is_empty() {
            return None;
        }
        let url = Url::parse(url).ok()?;
        self.entries
            .iter()
            .find(|(certificate, _)| presents_to(certificate, &url))
            .map(|(_, client)| client.as_ref().map_err(TransportError::clone))
    }
}

fn presents_to(certificate: &ClientCertificate, url: &Url) -> bool {
    url.scheme() == SECURE_SCHEME
        && match (url.host_str(), url.port_or_known_default()) {
            (Some(host), Some(port)) => certificate.matches(host, port),
            _ => false,
        }
}

fn certified_builder(
    settings: &RequestSettings,
    certificate: &ClientCertificate,
) -> Result<ClientBuilder, TransportError> {
    let builder = configure(Client::builder(), settings)?
        .identity(identity(certificate)?)
        .redirect(redirect_policy(settings, certificate.clone()));
    Ok(match settings.http_version {
        HttpVersion::Http1 => builder.http1_only(),
        HttpVersion::Http2 => builder.http2_prior_knowledge(),
        HttpVersion::Auto => builder,
    })
}

fn redirect_policy(settings: &RequestSettings, certificate: ClientCertificate) -> Policy {
    if !settings.follow_redirects {
        return Policy::none();
    }
    let limit = settings.max_redirects;
    Policy::custom(move |attempt: Attempt| {
        if attempt.previous().len() > limit {
            attempt.error(TOO_MANY_REDIRECTS)
        } else if presents_to(&certificate, attempt.url()) {
            attempt.follow()
        } else {
            attempt.stop()
        }
    })
}

fn identity(certificate: &ClientCertificate) -> Result<Identity, TransportError> {
    let invalid = |reason: String| {
        TransportError::InvalidSettings(format!(
            "client certificate for {}: {reason}",
            certificate.host.trim()
        ))
    };
    let pem = certificate.pem().map_err(invalid)?;
    Identity::from_pem(&pem).map_err(|error| invalid(error.without_url().to_string()))
}
