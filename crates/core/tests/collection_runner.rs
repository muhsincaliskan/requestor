use std::cell::RefCell;
use std::time::Duration;

use requestor_core::collection::{
    Collection, RunPacing, RunSession, RunStepResult, MAX_ITERATIONS,
};
use requestor_core::header::Header;
use requestor_core::prepared::PreparedRequest;
use requestor_core::response::HttpResponse;
use requestor_core::settings::RequestSettings;
use requestor_core::transport::{HttpTransport, TransportError};

const COLLECTION: &str = r#"{
  "info": {"name": "Shop"},
  "item": [
    {"name": "Login", "request": {"method": "POST", "url": "http://shop.test/login"},
     "event": [{"listen": "test", "script": {"exec": [
       "pm.test(\"Status is 200\", () => pm.response.to.have.status(200));",
       "pm.collectionVariables.set(\"session\", pm.response.headers.get(\"X-Session\"));"
     ]}}]},
    {"name": "Orders", "item": [
      {"name": "List", "request": {"method": "GET", "url": "http://shop.test/orders/{{session}}"},
       "event": [{"listen": "test", "script": {"exec": [
         "pm.test(\"Status is 201\", () => pm.response.to.have.status(201));"
       ]}}]}
    ]},
    {"name": "Broken", "request": {"method": "GET", "url": "http://offline.test/"}}
  ]
}"#;

#[derive(Default)]
struct FakeShop {
    requested: RefCell<Vec<String>>,
}

impl HttpTransport for FakeShop {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        self.requested.borrow_mut().push(request.url.clone());
        if request.url.contains("offline") {
            return Err(TransportError::ConnectionFailed("no route".to_string()));
        }
        Ok(HttpResponse {
            status: 200,
            reason: "OK".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: vec![Header::new("X-Session", "s-42")],
            body: Vec::new(),
            truncated: false,
            elapsed: Duration::from_millis(5),
        })
    }
}

#[test]
fn a_run_carries_extracted_variables_to_later_requests_and_reports_each_step() {
    let collection = Collection::from_json(COLLECTION).unwrap();
    let runnable = collection.runnable_requests();
    let mut session = RunSession::new(
        collection.variable.clone(),
        None,
        RequestSettings::default(),
    );
    let shop = FakeShop::default();
    let results: Vec<_> = runnable
        .iter()
        .map(|request| session.run_one(1, request, &shop))
        .collect();
    assert_eq!(shop.requested.borrow()[1], "http://shop.test/orders/s-42");
    assert!(results[0].is_success());
    assert_eq!((results[0].checks_passed, results[0].checks_total), (1, 1));
    assert_eq!(results[1].name, "Orders/List");
    assert_eq!(results[1].method, "GET");
    assert!(!results[1].is_success());
    assert_eq!(results[2].status, None);
    assert!(results[2]
        .error
        .as_deref()
        .is_some_and(|error| error.contains("no route")));
}

#[test]
fn pacing_keeps_iterations_and_delay_in_range() {
    assert_eq!(RunPacing::new(0, 10).iterations(), 1);
    assert_eq!(RunPacing::new(u32::MAX, 0).iterations(), MAX_ITERATIONS);
    assert_eq!(RunPacing::new(3, 250).delay(), Duration::from_millis(250));
    assert_eq!(RunPacing::new(3, 0).step_count(4), 12);
}

#[test]
fn pacing_pauses_only_between_steps() {
    let pacing = RunPacing::new(2, 100);
    assert!(pacing.pauses_after(1, 2));
    assert!(pacing.pauses_after(3, 2));
    assert!(!pacing.pauses_after(4, 2));
    assert!(!RunPacing::new(2, 0).pauses_after(1, 2));
}

#[test]
fn an_error_status_fails_the_step_even_when_every_check_passes() {
    let step = |status| RunStepResult {
        iteration: 1,
        name: "List".to_string(),
        method: "GET".to_string(),
        status: Some(status),
        elapsed: Duration::ZERO,
        checks_passed: 1,
        checks_total: 1,
        error: None,
    };
    assert!(step(302).is_success());
    assert!(!step(404).is_success());
    assert!(!step(503).is_success());
}
