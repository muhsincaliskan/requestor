use std::cell::RefCell;
use std::sync::mpsc::channel;

use requestor_core::collection::{Collection, RunPacing};
use requestor_core::header::Header;
use requestor_core::settings::{ProxySettings, RequestSettings};

use super::*;

const SAMPLE: &str = r#"{
  "info": {"name": "Run"},
  "item": [
    {"name":"First","request":{"method":"GET","url":"http://shop.test/first"},
     "event":[{"listen":"test","script":{"exec":[
       "pm.collectionVariables.set(\"id\", pm.response.headers.get(\"X-Id\"));"
     ]}}]},
    {"name":"Next","request":{"method":"GET","url":"http://shop.test/{{id}}"}}
  ]
}"#;

#[derive(Default)]
struct FakeTransport {
    urls: RefCell<Vec<String>>,
    cancel: Option<Cancellation>,
}

impl HttpTransport for FakeTransport {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        self.urls.borrow_mut().push(request.url);
        if let Some(cancellation) = &self.cancel {
            cancellation.cancel();
        }
        Ok(HttpResponse {
            status: 200,
            reason: String::new(),
            version: String::new(),
            headers: vec![Header::new("X-Id", "42")],
            body: Vec::new(),
            truncated: false,
            elapsed: Duration::ZERO,
        })
    }
}

fn sample_job() -> CollectionRunJob {
    let collection = Collection::from_json(SAMPLE).unwrap();
    CollectionRunJob {
        job: 1,
        requests: collection.runnable_requests(),
        collection_variables: collection.variable,
        environment: None,
        settings: RequestSettings::default(),
        pacing: RunPacing::new(2, 0),
    }
}

fn reporter(report: Sender<Action>, job: u64) -> RunReporter {
    RunReporter {
        job,
        report,
        wake: Context::default(),
    }
}

#[test]
fn the_worker_reports_each_step_and_carries_extractions_to_later_steps() {
    let run = sample_job();
    let (report, actions) = channel();
    let transport = FakeTransport::default();
    run_steps(
        &run,
        &Cancellation::default(),
        &reporter(report, run.job),
        &transport,
    );
    let events: Vec<_> = actions.try_iter().collect();
    assert_eq!(events.len(), 4);
    assert!(matches!(
        &events[0],
        Action::RunStepFinished { job: 1, outcome } if outcome.extracted.len() == 1
    ));
    assert!(matches!(
        &events[3],
        Action::RunStepFinished { job: 1, outcome } if outcome.result.iteration == 2
    ));
    assert_eq!(transport.urls.borrow()[1], "http://shop.test/42");
}

#[test]
fn a_cancelled_step_is_not_reported_and_no_next_request_is_sent() {
    let run = sample_job();
    let (report, actions) = channel();
    let cancellation = Cancellation::default();
    let transport = FakeTransport {
        cancel: Some(cancellation.clone()),
        ..FakeTransport::default()
    };
    run_steps(&run, &cancellation, &reporter(report, run.job), &transport);
    assert_eq!(transport.urls.borrow().len(), 1);
    assert!(actions.try_recv().is_err());
}

#[test]
fn a_transport_configuration_failure_reports_an_error_without_starting_requests() {
    let mut run = sample_job();
    run.settings.proxy = Some(ProxySettings {
        url: "http://[".to_string(),
        ..ProxySettings::default()
    });
    let (report, actions) = channel();
    let failure = execute(&run, &Cancellation::default(), &reporter(report, run.job)).unwrap_err();
    assert!(!failure.is_empty());
    assert!(actions.try_recv().is_err());
}
