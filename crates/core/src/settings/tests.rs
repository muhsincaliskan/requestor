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
