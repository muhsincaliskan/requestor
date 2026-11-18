use super::{
    HttpVersion, ProxySettings, RequestSettings, BYTES_PER_MEGABYTE, MAX_EVENT_STREAM_BYTES,
    MAX_SOCKET_MESSAGE_BYTES,
};

#[test]
fn http_versions_have_distinct_labels() {
    let labels: std::collections::HashSet<_> = HttpVersion::ALL
        .iter()
        .map(|version| version.label())
        .collect();
    assert_eq!(labels.len(), HttpVersion::ALL.len());
    assert!(HttpVersion::ALL.contains(&HttpVersion::default()));
}

#[test]
fn response_size_converts_between_megabytes_and_bytes() {
    let mut settings = RequestSettings::default();
    assert_eq!(settings.max_response_megabytes(), 50);
    settings.set_max_response_megabytes(2);
    assert_eq!(settings.max_response_bytes, 2 * 1024 * 1024);
    settings.set_max_response_megabytes(0);
    assert_eq!(settings.response_limit(), usize::MAX);
    settings.set_max_response_megabytes(u64::MAX);
    assert_eq!(settings.max_response_bytes, usize::MAX);
}

#[test]
fn empty_credentials_text_clears_the_value() {
    let mut proxy = ProxySettings::default();
    proxy.set_username_text("user");
    proxy.set_password_text("pw");
    assert_eq!(proxy.username_text(), "user");
    assert_eq!(proxy.password.as_ref().unwrap().expose(), "pw");
    proxy.set_username_text("");
    proxy.set_password_text("");
    assert_eq!(proxy.username_text(), "");
    assert!(proxy.username.is_none() && proxy.password.is_none());
}

#[test]
fn bypass_hosts_text_is_normalized() {
    let mut proxy = ProxySettings::default();
    proxy.set_bypass_hosts_text(" localhost, ,*.corp.example ,localhost,");
    assert_eq!(proxy.bypass_hosts, ["localhost", "*.corp.example"]);
    assert_eq!(proxy.bypass_hosts_text(), "localhost, *.corp.example");
    proxy.set_bypass_hosts_text("");
    assert!(proxy.bypass_hosts.is_empty());
}

#[test]
fn zero_timeout_means_no_timeout() {
    let settings = RequestSettings {
        request_timeout_ms: 0,
        ..RequestSettings::default()
    };
    assert_eq!(settings.request_timeout(), None);
    assert!(settings.connect_timeout().is_some());
}

#[test]
fn event_streams_are_capped_even_without_a_response_limit() {
    let mut settings = RequestSettings::default();
    settings.set_max_response_megabytes(0);
    assert_eq!(settings.event_stream_limit(), MAX_EVENT_STREAM_BYTES);
    settings.set_max_response_megabytes(1);
    assert_eq!(settings.event_stream_limit(), BYTES_PER_MEGABYTE);
}

#[test]
fn zero_response_limit_means_unlimited() {
    let settings = RequestSettings {
        max_response_bytes: 0,
        ..RequestSettings::default()
    };
    assert_eq!(settings.response_limit(), usize::MAX);
    assert_eq!(
        RequestSettings::default().response_limit(),
        RequestSettings::default().max_response_bytes
    );
}

#[test]
fn defaults_are_safe() {
    let settings = RequestSettings::default();
    assert!(settings.verify_ssl);
    assert!(settings.follow_redirects);
    assert_eq!(settings.http_version, HttpVersion::Auto);
}

#[test]
fn partial_documents_fall_back_to_defaults() {
    let settings: RequestSettings =
        serde_json::from_str(r#"{"verify_ssl":false,"http_version":"http2"}"#).unwrap();
    assert!(!settings.verify_ssl);
    assert_eq!(settings.http_version, HttpVersion::Http2);
    assert_eq!(
        settings.max_redirects,
        RequestSettings::default().max_redirects
    );
}

#[test]
fn proxy_password_is_masked_in_debug_output() {
    let json =
        r#"{"proxy":{"url":"http://p:8080","username":"u","password":"pw","bypass_hosts":[]}}"#;
    let settings: RequestSettings = serde_json::from_str(json).unwrap();
    assert!(!format!("{settings:?}").contains("pw\""));
}

#[test]
fn socket_messages_are_capped_even_without_a_response_limit() {
    let mut settings = RequestSettings::default();
    settings.set_max_response_megabytes(0);
    assert_eq!(settings.socket_message_limit(), MAX_SOCKET_MESSAGE_BYTES);
    settings.set_max_response_megabytes(2);
    assert_eq!(settings.socket_message_limit(), 2 * BYTES_PER_MEGABYTE);
}

#[test]
fn bypass_hosts_match_exact_names_and_subdomains() {
    let mut proxy = ProxySettings::default();
    proxy.set_bypass_hosts_text("localhost, .internal.test");
    assert!(proxy.bypasses("localhost"));
    assert!(proxy.bypasses("API.internal.test"));
    assert!(proxy.bypasses("internal.test"));
    assert!(!proxy.bypasses("example.com"));
    proxy.set_bypass_hosts_text("*");
    assert!(proxy.bypasses("example.com"));
}

fn certificate_for(host: &str) -> super::ClientCertificate {
    super::ClientCertificate {
        host: host.to_string(),
        certificate_path: "client.pem".into(),
        key_path: None,
    }
}

#[test]
fn client_certificates_match_their_host_port_and_subdomains() {
    let cases = [
        ("API.example.com", "api.example.com", 443, true),
        ("api.example.com", "example.com", 443, false),
        ("api.example.com:8443", "api.example.com", 8443, true),
        ("api.example.com:8443", "api.example.com", 443, false),
        ("*.example.com", "a.b.EXAMPLE.com", 443, true),
        ("*.example.com", "example.com", 443, false),
        ("*.example.com", "badexample.com", 443, false),
        ("*", "anything.test", 1, true),
        ("[::1]:9443", "[::1]", 9443, true),
    ];
    for (pattern, host, port, expected) in cases {
        assert_eq!(
            certificate_for(pattern).matches(host, port),
            expected,
            "{pattern} vs {host}:{port}"
        );
    }
}

#[test]
fn client_certificates_need_a_host_and_a_certificate_file() {
    assert!(certificate_for("api.example.com").is_complete());
    assert!(!certificate_for(" ").is_complete());
    let without_file = super::ClientCertificate {
        certificate_path: "".into(),
        ..certificate_for("api.example.com")
    };
    assert!(!without_file.is_complete());
}

#[test]
fn client_certificate_pem_joins_the_certificate_and_key_files() {
    let folder = std::env::temp_dir().join(format!("requestor-pem-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let (certificate_path, key_path) = (folder.join("cert.pem"), folder.join("key.pem"));
    std::fs::write(&certificate_path, "CERT").unwrap();
    std::fs::write(&key_path, "KEY").unwrap();
    let certificate =
        super::ClientCertificate::for_every_host(certificate_path.clone(), Some(key_path));
    assert_eq!(certificate.pem().unwrap(), b"CERT\nKEY");

    let missing = super::ClientCertificate::for_every_host(folder.join("missing.pem"), None);
    let error = missing.pem().unwrap_err();
    assert!(error.starts_with("cannot read"), "{error}");
    assert!(error.contains("missing.pem"), "{error}");
    std::fs::remove_dir_all(folder).unwrap();
}
