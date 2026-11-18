mod support;

use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;

use requestor_core::collection::{Item, RequestSpec};
use requestor_core::import::{
    import_address, is_wsdl_address, ImportError, Imported, ImportedCollection,
};
use requestor_core::prepared::PreparedRequest;
use requestor_core::response::HttpResponse;
use requestor_core::settings::RequestSettings;
use requestor_core::transport::{DocumentTransport, HttpTransport, TransportError};
use support::{http_response, TestServer};

const BANK: &str = "https://bank.example.test/split/";
const CALCULATOR_ADDRESS: &str = "https://calc.example.test/calc?wsdl";
const MAX_DOCUMENT_BYTES: usize = 1024;

struct FakeWeb {
    pages: Vec<(String, Vec<u8>)>,
    truncated: bool,
    requested: RefCell<Vec<String>>,
}

impl FakeWeb {
    fn serving(pages: Vec<(String, Vec<u8>)>) -> Self {
        FakeWeb {
            pages,
            truncated: false,
            requested: RefCell::new(Vec::new()),
        }
    }

    fn page(url: &str, body: impl Into<Vec<u8>>) -> Self {
        FakeWeb::serving(vec![(url.to_string(), body.into())])
    }

    fn truncating(mut self) -> Self {
        self.truncated = true;
        self
    }
}

impl HttpTransport for FakeWeb {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        self.requested.borrow_mut().push(request.url.clone());
        let found = self.pages.iter().find(|(url, _)| *url == request.url);
        Ok(HttpResponse {
            status: if found.is_some() { 200 } else { 404 },
            reason: String::new(),
            version: String::new(),
            headers: Vec::new(),
            body: found.map(|(_, body)| body.clone()).unwrap_or_default(),
            truncated: self.truncated,
            elapsed: Duration::ZERO,
            timing: None,
        })
    }
}

fn fixture(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/wsdl")
        .join(relative);
    std::fs::read_to_string(path).unwrap()
}

fn bank_page(name: &str) -> (String, Vec<u8>) {
    (
        format!("{BANK}{name}"),
        fixture(&format!("split/{name}")).into_bytes(),
    )
}

fn calculator_with(schema: &str) -> String {
    fixture("calculator.wsdl").replace(
        "<s:schema elementFormDefault",
        &format!("<s:schema>{schema}</s:schema><s:schema elementFormDefault"),
    )
}

fn collection_of(imported: Result<Imported, ImportError>) -> ImportedCollection {
    match imported.unwrap() {
        Imported::Collection(imported) => imported,
        other => panic!("expected a collection, got {other:?}"),
    }
}

fn download_error(imported: Result<Imported, ImportError>) -> String {
    match imported {
        Err(ImportError::Download(reason)) => reason,
        other => panic!("expected a download error, got {other:?}"),
    }
}

fn body_of(item: &Item) -> String {
    match item.request.as_ref().unwrap() {
        RequestSpec::Full(request) => request.body.clone().unwrap().raw.unwrap(),
        other => panic!("expected a full request, got {other:?}"),
    }
}

#[test]
fn a_wsdl_address_is_downloaded_with_the_documents_it_imports() {
    let web = FakeWeb::serving(
        [
            "service.wsdl",
            "abstract.wsdl",
            "types/accounts.xsd",
            "types/common.xsd",
        ]
        .map(bank_page)
        .into(),
    );
    let address = format!(" {BANK}service.wsdl ");
    let imported = collection_of(import_address(&address, &web));
    assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
    assert!(body_of(&imported.collection.item[0]).contains("<number>TR00-0000</number>"));

    let partial = FakeWeb::serving(vec![bank_page("service.wsdl")]);
    let imported = collection_of(import_address(address.trim(), &partial));
    assert_eq!(
        imported.warnings,
        ["abstract.wsdl was not loaded: the server answered 404"]
    );
    let missing = download_error(import_address(
        "https://bank.example.test/missing?wsdl",
        &partial,
    ));
    assert!(missing.ends_with("the server answered 404"), "{missing}");
}

#[test]
fn remote_documents_never_reach_local_files() {
    let web = FakeWeb::page(
        CALCULATOR_ADDRESS,
        calculator_with(r#"<s:include schemaLocation="file:///etc/hosts"/>"#),
    );
    let imported = collection_of(import_address(CALCULATOR_ADDRESS, &web));
    assert_eq!(
        imported.warnings,
        ["file:///etc/hosts was not loaded: only http and https addresses are downloaded"]
    );

    let root = download_error(import_address("file:///etc/passwd", &web));
    assert!(
        root.ends_with("only http and https addresses are downloaded"),
        "{root}"
    );
}

#[test]
fn absolute_schema_locations_stay_on_the_remote_host() {
    let web = FakeWeb::page(
        CALCULATOR_ADDRESS,
        calculator_with(r#"<s:include schemaLocation="/etc/hosts"/>"#),
    );
    let imported = collection_of(import_address(CALCULATOR_ADDRESS, &web));
    assert_eq!(
        imported.warnings,
        ["/etc/hosts was not loaded: the server answered 404"]
    );
    assert_eq!(
        *web.requested.borrow(),
        [CALCULATOR_ADDRESS, "https://calc.example.test/etc/hosts"]
    );
}

#[test]
fn truncated_and_binary_documents_are_refused() {
    let truncated = FakeWeb::page(CALCULATOR_ADDRESS, fixture("calculator.wsdl")).truncating();
    let reason = download_error(import_address(CALCULATOR_ADDRESS, &truncated));
    assert!(reason.contains("MB"), "{reason}");

    let binary = FakeWeb::page(CALCULATOR_ADDRESS, vec![0xff, 0xfe, 0x00]);
    let reason = download_error(import_address(CALCULATOR_ADDRESS, &binary));
    assert!(
        reason.ends_with("the response is not UTF-8 text"),
        "{reason}"
    );
}

#[test]
fn event_streams_are_not_downloaded_as_documents() {
    let server = TestServer::spawn(|_| {
        http_response(
            "200 OK",
            &["Content-Type: text/event-stream"],
            b"data: tick\n\n",
        )
    });
    let transport =
        DocumentTransport::new(&RequestSettings::default(), MAX_DOCUMENT_BYTES).unwrap();
    let error = transport
        .send(PreparedRequest::get(server.url("/events"), Vec::new()))
        .unwrap_err();
    assert!(error.to_string().contains("event stream"), "{error}");
    let sent = server.requests().concat().to_ascii_lowercase();
    assert!(!sent.contains("postman-token"), "{sent}");
    assert!(!sent.contains("cache-control"), "{sent}");
}

#[test]
fn wsdl_addresses_are_recognised_by_their_path_or_query() {
    for address in [
        "http://h/Service.asmx?WSDL",
        "https://h/svc?singleWsdl",
        "https://h/svc?wsdl=wsdl0",
        "https://h/svc?WSDL&x=1",
        "HTTPS://h/files/calc.wsdl",
    ] {
        assert!(is_wsdl_address(address), "{address}");
    }
    for text in [
        "https://h/api?wsdlx=1",
        "https://h/users",
        "ftp://h/a.wsdl",
        "calc.wsdl",
    ] {
        assert!(!is_wsdl_address(text), "{text}");
    }
}
