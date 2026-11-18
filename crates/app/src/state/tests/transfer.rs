use std::path::PathBuf;

use requestor_core::collection::Collection;
use requestor_core::workspace::Workspace;

use super::{open, state_with, state_with_request};
use requestor_core::draft::BodyKind;
use requestor_core::import::{ImportError, Imported, ImportedCollection};
use requestor_core::soap::SoapVersion;

use crate::state::{Action, Effect, LibraryTarget, Overlay};

const IMPORT_DIRECTORY: &str = "requestor-transfer-tests";

fn scratch_file(name: &str, contents: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("{IMPORT_DIRECTORY}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn imported_text_becomes_a_collection_a_tab_or_an_error_on_the_form() {
    let mut state = state_with(Workspace::default());
    state.apply(
        Action::ImportText("curl https://example.test/a".to_string()),
        0.0,
    );
    assert_eq!(state.active().unwrap().draft.url, "https://example.test/a");

    let collection = Collection::new("Imported").to_json_pretty().unwrap();
    state.apply(Action::ImportText(collection), 0.0);
    assert_eq!(state.workspace.collections.len(), 1);

    state.apply(Action::Show(Overlay::Import(Default::default())), 0.0);
    state.apply(Action::ImportText("{\"neither\":true}".to_string()), 0.0);
    let Some(Overlay::Import(form)) = &state.overlay else {
        panic!("the import form stays open");
    };
    assert!(form.error.is_some());
}

#[test]
fn choosing_files_asks_the_shell_for_a_native_picker() {
    let (mut state, collection_id, _) = state_with_request();
    let effects = state.apply(Action::ChooseImportFile, 0.0);
    assert!(matches!(effects.as_slice(), [Effect::PickImportFile]));
    let target = LibraryTarget::Collection { collection_id };
    let effects = state.apply(
        Action::ChooseExportLocation {
            target: target.clone(),
            suggested: "Sample.postman_collection.json".into(),
        },
        0.0,
    );
    assert!(matches!(
        effects.as_slice(),
        [Effect::PickExportLocation { target: chosen, .. }] if *chosen == target
    ));
}

#[test]
fn an_imported_file_closes_only_the_import_dialog() {
    let path = scratch_file(
        "dropped.postman_collection.json",
        &Collection::new("Dropped").to_json_pretty().unwrap(),
    );
    let mut state = state_with(Workspace::default());
    state.apply(Action::Show(Overlay::Settings), 0.0);
    state.apply(Action::ImportFile(path.clone()), 0.0);
    assert_eq!(state.overlay, Some(Overlay::Settings));
    assert_eq!(state.workspace.collections.len(), 1);
    state.apply(Action::Show(Overlay::Import(Default::default())), 0.0);
    state.apply(Action::ImportFile(path), 0.0);
    assert!(state.overlay.is_none());
    assert_eq!(state.workspace.collections.len(), 2);
}

#[test]
fn an_unreadable_file_reports_on_the_form_or_in_the_status_bar() {
    let missing = std::env::temp_dir().join("requestor-transfer-tests-missing.json");
    let mut state = state_with(Workspace::default());
    state.apply(Action::Show(Overlay::Import(Default::default())), 0.0);
    state.apply(Action::ImportFile(missing.clone()), 0.0);
    let Some(Overlay::Import(form)) = &state.overlay else {
        panic!("the import dialog should stay open");
    };
    assert!(form
        .error
        .as_deref()
        .is_some_and(|error| error.starts_with("Could not read")));
    state.overlay = None;
    state.apply(Action::ImportFile(missing), 0.0);
    assert!(state.overlay.is_none());
    assert!(state.notices.current(0.0).is_some());
}

#[test]
fn a_wsdl_file_opens_its_operations_as_soap_requests() {
    let schema = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:ping" elementFormDefault="qualified">
        <xs:element name="Ping"><xs:complexType><xs:sequence><xs:element name="text" type="xs:string"/></xs:sequence></xs:complexType></xs:element>
    </xs:schema>"#;
    scratch_file("ping.xsd", schema);
    let wsdl = r#"<definitions xmlns="http://schemas.xmlsoap.org/wsdl/" xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
        xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:p="urn:ping" targetNamespace="urn:ping">
      <types><xs:schema><xs:import namespace="urn:ping" schemaLocation="ping.xsd"/></xs:schema></types>
      <message name="PingIn"><part name="parameters" element="p:Ping"/></message>
      <portType name="Pinger"><operation name="Ping"><input message="p:PingIn"/></operation></portType>
      <binding name="PingerSoap" type="p:Pinger">
        <soap:binding transport="http://schemas.xmlsoap.org/soap/http"/>
        <operation name="Ping"><soap:operation soapAction="urn:ping/Ping"/><input><soap:body use="literal"/></input></operation>
      </binding>
      <service name="PingService"><port name="PingerSoap" binding="p:PingerSoap"><soap:address location="https://ping.example.test/soap"/></port></service>
    </definitions>"#;
    let path = scratch_file("ping.wsdl", wsdl);
    let mut state = state_with(Workspace::default());
    state.apply(Action::ImportFile(path), 0.0);
    let collection = &state.workspace.collections[0];
    assert_eq!(collection.info.name, "PingService");
    let collection_id = collection.id().to_string();
    let item_id = collection.item[0].id.clone().unwrap();
    let tab = open(&mut state, &collection_id, &item_id);
    let draft = &state.tab_mut(tab).unwrap().draft;
    assert_eq!(draft.body.kind, BodyKind::Soap);
    assert_eq!(draft.body.soap_version(), Some(SoapVersion::V1_1));
    assert_eq!(draft.soap_action(), "urn:ping/Ping");
    assert!(
        draft.body.raw.contains("<ns1:text>?</ns1:text>"),
        "{}",
        draft.body.raw
    );
}

#[test]
fn a_wsdl_address_is_downloaded_before_it_is_imported() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::Show(Overlay::Import(Default::default())), 0.0);
    let effects = state.apply(
        Action::ImportText(" https://calc.example.test/service.asmx?WSDL ".to_string()),
        0.0,
    );
    assert!(matches!(
        effects.as_slice(),
        [Effect::DownloadImport { address, .. }] if address == "https://calc.example.test/service.asmx?WSDL"
    ));
    let Some(Overlay::Import(form)) = &state.overlay else {
        panic!("the import dialog stays open while downloading");
    };
    assert!(form.downloading);

    let failed = Err(ImportError::Download("the server answered 404".to_string()));
    state.apply(Action::ImportDownloaded(Box::new(failed)), 0.0);
    let Some(Overlay::Import(form)) = &state.overlay else {
        panic!("a failed download keeps the dialog open");
    };
    assert!(!form.downloading);
    assert!(form
        .error
        .as_deref()
        .is_some_and(|error| error.contains("404")));

    let collection = ImportedCollection::without_warnings(Collection::new("Calculator"));
    state.apply(
        Action::ImportDownloaded(Box::new(Ok(Imported::Collection(collection)))),
        0.0,
    );
    assert!(state.overlay.is_none());
    assert_eq!(state.workspace.collections[0].info.name, "Calculator");
}
