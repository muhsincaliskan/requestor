use requestor_core::settings::{
    ClientCertificate, HttpVersion, ProxyChoice, ProxySettings, RequestOverrides, RequestSettings,
};
use serde_json::{json, Map, Value};

fn extra(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

#[test]
fn postman_behaviour_keys_are_read() {
    let item = extra(json!({
        "protocolProfileBehavior": {
            "followRedirects": false,
            "strictSSL": false,
            "maxRedirects": 3,
            "protocolVersion": "http2",
            "disableBodyPruning": true
        }
    }));
    let overrides = RequestOverrides::from_extra(&item);
    assert_eq!(overrides.follow_redirects, Some(false));
    assert_eq!(overrides.verify_ssl, Some(false));
    assert_eq!(overrides.max_redirects, Some(3));
    assert_eq!(overrides.http_version, Some(HttpVersion::Http2));
    assert_eq!(overrides.count(), 4);
}

#[test]
fn writing_keeps_unknown_behaviour_keys_and_round_trips() {
    let mut item = extra(json!({
        "protocolProfileBehavior": { "disableBodyPruning": true, "strictSSL": true }
    }));
    let overrides = RequestOverrides {
        timeout_ms: Some(1500),
        proxy: ProxyChoice::Custom("http://proxy.local:8080".to_string()),
        skip_client_certificates: true,
        ..RequestOverrides::default()
    };
    overrides.write_into(&mut item);
    let behavior = &item["protocolProfileBehavior"];
    assert_eq!(behavior["disableBodyPruning"], json!(true));
    assert!(behavior.get("strictSSL").is_none());
    assert_eq!(RequestOverrides::from_extra(&item), overrides);
}

#[test]
fn clearing_every_override_removes_an_empty_behaviour_object() {
    let mut item = extra(json!({ "protocolProfileBehavior": { "strictSSL": false } }));
    RequestOverrides::default().write_into(&mut item);
    assert!(!item.contains_key("protocolProfileBehavior"));
}

#[test]
fn an_empty_override_leaves_a_foreign_behaviour_value_alone() {
    let mut item = extra(json!({ "protocolProfileBehavior": "unexpected" }));
    RequestOverrides::default().write_into(&mut item);
    assert_eq!(item["protocolProfileBehavior"], json!("unexpected"));
}

#[test]
fn unset_fields_fall_back_to_the_global_settings() {
    let global = RequestSettings {
        proxy: Some(ProxySettings {
            url: "http://global:3128".to_string(),
            ..ProxySettings::default()
        }),
        client_certificates: vec![ClientCertificate::default()],
        ..RequestSettings::default()
    };
    assert_eq!(RequestOverrides::default().apply_to(&global), global);
}

#[test]
fn set_fields_replace_the_global_settings() {
    let global = RequestSettings {
        client_certificates: vec![ClientCertificate::default()],
        ..RequestSettings::default()
    };
    let overrides = RequestOverrides {
        http_version: Some(HttpVersion::Http1),
        timeout_ms: Some(250),
        verify_ssl: Some(false),
        follow_redirects: Some(false),
        max_redirects: Some(2),
        proxy: ProxyChoice::Direct,
        skip_client_certificates: true,
    };
    let effective = overrides.apply_to(&global);
    assert_eq!(effective.http_version, HttpVersion::Http1);
    assert_eq!(effective.request_timeout_ms, 250);
    assert!(!effective.verify_ssl);
    assert!(!effective.follow_redirects);
    assert_eq!(effective.max_redirects, 2);
    assert!(!effective.use_system_proxy);
    assert!(effective.proxy.is_none());
    assert!(effective.client_certificates.is_empty());
}

#[test]
fn a_custom_proxy_without_an_address_keeps_the_global_proxy() {
    let global = RequestSettings::default();
    let overrides = RequestOverrides {
        proxy: ProxyChoice::Custom("  ".to_string()),
        ..RequestOverrides::default()
    };
    assert_eq!(overrides.apply_to(&global), global);
}

#[test]
fn number_text_rejects_anything_but_digits_and_empties_to_unset() {
    let mut overrides = RequestOverrides::default();
    overrides.set_timeout_text("500");
    overrides.set_timeout_text("5s00");
    assert_eq!(overrides.timeout_ms, Some(500));
    assert_eq!(overrides.timeout_text(), "500");
    overrides.set_timeout_text("");
    assert_eq!(overrides.timeout_ms, None);
    overrides.set_max_redirects_text("7");
    assert_eq!(overrides.max_redirects_text(), "7");
}

#[test]
fn saving_keeps_values_it_cannot_read() {
    let mut item = extra(json!({
        "protocolProfileBehavior": { "maxRedirects": "5", "protocolVersion": "http3" }
    }));
    let overrides = RequestOverrides {
        verify_ssl: Some(false),
        ..RequestOverrides::from_extra(&item)
    };
    overrides.write_into(&mut item);
    let behavior = &item["protocolProfileBehavior"];
    assert_eq!(behavior["maxRedirects"], json!("5"));
    assert_eq!(behavior["protocolVersion"], json!("http3"));
    assert_eq!(behavior["strictSSL"], json!(false));
}

#[test]
fn stored_overrides_tell_cleared_apart_from_missing() {
    let mut extra = Map::new();
    assert_eq!(RequestOverrides::stored_in(&extra), None);
    RequestOverrides::default().store_in(&mut extra);
    assert_eq!(
        RequestOverrides::stored_in(&extra),
        Some(RequestOverrides::default())
    );
}

#[test]
fn proxy_addresses_with_credentials_are_recognised() {
    let with = ProxyChoice::Custom("http://user:secret@proxy:8080".to_string());
    let without = ProxyChoice::Custom("http://proxy:8080/path@x".to_string());
    assert!(with.has_credentials());
    assert!(!without.has_credentials());
    assert!(RequestOverrides {
        proxy: with,
        ..RequestOverrides::default()
    }
    .lowers_security());
}
