use requestor_core::environment::{Environment, EnvironmentColor};

#[test]
fn production_names_are_protected_and_red_unless_chosen_otherwise() {
    let mut production = Environment::new("Shop Prod");
    assert!(production.is_protected());
    assert_eq!(production.color(), Some(EnvironmentColor::Red));
    production.set_color(Some(EnvironmentColor::Purple));
    assert_eq!(production.color(), Some(EnvironmentColor::Purple));
    production.set_protected(false);
    assert!(!production.is_protected());
    assert!(production.extra.contains_key("requestorProtected"));
}

#[test]
fn other_names_are_unprotected_and_uncoloured_by_default() {
    let mut staging = Environment::new("staging-products");
    assert!(!staging.is_protected());
    assert_eq!(staging.color(), None);
    staging.set_protected(true);
    assert!(staging.is_protected());
    staging.set_protected(false);
    assert!(staging.extra.is_empty());
}

#[test]
fn names_split_on_any_punctuation() {
    for name in ["API (prod)", "Shop/Prod", "prod:eu", "LIVE"] {
        assert!(Environment::new(name).is_protected(), "{name}");
    }
}

#[test]
fn switching_protection_on_survives_a_rename() {
    let mut environment = Environment::new("prod-api");
    environment.set_protected(true);
    environment.name = "api".to_string();
    assert!(environment.is_protected());
}

#[test]
fn marking_survives_a_json_round_trip_and_clears_cleanly() {
    let mut environment = Environment::new("Local");
    environment.set_color(Some(EnvironmentColor::Green));
    environment.set_protected(true);
    let restored = Environment::from_json(&environment.to_json_pretty().unwrap()).unwrap();
    assert_eq!(restored.color(), Some(EnvironmentColor::Green));
    assert!(restored.is_protected());
    environment.set_color(None);
    environment.set_protected(false);
    assert!(environment.extra.is_empty());
    assert!(!environment.is_protected());
}
