use std::path::{Path, PathBuf};

use requestor_core::collection::{Item, Request, RequestSpec};
use requestor_core::import::{
    import_file_text, import_text, ImportError, Imported, ImportedCollection,
};

const CALCULATOR: &str = include_str!("fixtures/wsdl/calculator.wsdl");
const SPLIT_SERVICE: &str = "fixtures/wsdl/split/service.wsdl";

fn collection_of(imported: Result<Imported, ImportError>) -> ImportedCollection {
    match imported.unwrap() {
        Imported::Collection(imported) => imported,
        other => panic!("expected a collection, got {other:?}"),
    }
}

fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(relative)
}

fn request_of(item: &Item) -> Request {
    match item.request.as_ref().unwrap() {
        RequestSpec::Full(request) => (**request).clone(),
        RequestSpec::Url(_) => panic!("expected a full request"),
    }
}

fn body_of(item: &Item) -> String {
    request_of(item).body.unwrap().raw.unwrap()
}

fn names(items: &[Item]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| item.name.as_deref())
        .collect()
}

#[test]
fn every_soap_port_becomes_a_folder_of_ready_requests() {
    let imported = collection_of(import_text(&format!("\u{feff}{CALCULATOR}")));
    let collection = &imported.collection;
    assert_eq!(collection.info.name, "Calculator");
    assert_eq!(
        names(&collection.item),
        ["CalculatorSoap", "CalculatorSoap12"]
    );
    let soap11 = collection.item[0].item.as_ref().unwrap();
    assert_eq!(names(soap11), ["Add", "PlaceOrder"]);
    let add = request_of(&soap11[0]);
    assert_eq!(add.method.as_deref(), Some("POST"));
    assert_eq!(
        add.url.as_ref().unwrap().to_text(),
        "http://www.example.test/calculator.asmx"
    );
    assert_eq!(
        add.header_value("SOAPAction"),
        Some("http://tempuri.org/Add")
    );
    assert_eq!(
        add.description.as_ref().unwrap().content(),
        Some("Adds two integers.")
    );
    assert_eq!(
        body_of(&soap11[0]),
        r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Header/>
  <soap:Body>
    <ns1:Add xmlns:ns1="http://tempuri.org/">
      <ns1:intA>?</ns1:intA>
      <ns1:intB>?</ns1:intB>
    </ns1:Add>
  </soap:Body>
</soap:Envelope>"#
    );
    assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
}

#[test]
fn soap_1_2_requests_carry_the_action_in_the_content_type() {
    let imported = collection_of(import_text(CALCULATOR));
    let soap12 = imported.collection.item[1].item.as_ref().unwrap();
    let add = request_of(&soap12[0]);
    assert_eq!(add.header_value("SOAPAction"), None);
    assert_eq!(
        add.header_value("Content-Type"),
        Some("application/soap+xml; charset=utf-8; action=\"http://tempuri.org/Add\"")
    );
    assert!(body_of(&soap12[0]).contains("http://www.w3.org/2003/05/soap-envelope"));
}

#[test]
fn samples_follow_extensions_choices_enumerations_and_stop_at_recursion() {
    let imported = collection_of(import_text(CALCULATOR));
    let soap11 = imported.collection.item[0].item.as_ref().unwrap();
    let body = body_of(&soap11[1]);
    let expected = r#"    <ns1:PlaceOrder xmlns:ns1="http://tempuri.org/">
      <ns1:order currency="?">
        <ns1:Id>?</ns1:Id>
        <ns1:Status>Open</ns1:Status>
        <ns1:Lines sku="?">?</ns1:Lines>
        <ns1:Card>?</ns1:Card>
        <ns1:Parent/>
      </ns1:order>
    </ns1:PlaceOrder>"#;
    assert!(body.contains(expected), "{body}");
}

#[test]
fn imported_files_and_schemas_load_relative_to_the_wsdl_file() {
    let path = fixture(SPLIT_SERVICE);
    let text = std::fs::read_to_string(&path).unwrap();
    let imported = collection_of(import_file_text(&text, &path));
    assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
    let collection = &imported.collection;
    assert_eq!(names(&collection.item), ["GetBalance"]);
    let request = request_of(&collection.item[0]);
    assert_eq!(
        request.header_value("SOAPAction"),
        Some("urn:accounts/GetBalance")
    );
    let body = body_of(&collection.item[0]);
    assert!(
        body.contains(
            "    <ns1:GetBalance xmlns:ns1=\"urn:accounts:types\">\n      <account>TR00-0000</account>\n    </ns1:GetBalance>"
        ),
        "{body}"
    );
}

#[test]
fn pasted_text_names_the_imports_it_could_not_load() {
    let text = std::fs::read_to_string(fixture(SPLIT_SERVICE)).unwrap();
    let imported = collection_of(import_text(&text));
    assert_eq!(imported.warnings.len(), 1, "{:?}", imported.warnings);
    assert!(imported.warnings[0].starts_with("abstract.wsdl was not loaded"));
}

#[test]
fn rpc_operations_wrap_their_parts_in_the_operation_element() {
    let wsdl = r#"<definitions xmlns="http://schemas.xmlsoap.org/wsdl/"
        xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
        xmlns:xsd="http://www.w3.org/2001/XMLSchema"
        xmlns:tns="urn:weather" targetNamespace="urn:weather">
      <message name="ForecastRequest">
        <part name="city" type="xsd:string"/>
        <part name="days" type="xsd:int"/>
      </message>
      <portType name="Weather"><operation name="Forecast"><input message="tns:ForecastRequest"/></operation></portType>
      <binding name="WeatherBinding" type="tns:Weather">
        <soap:binding style="rpc" transport="http://schemas.xmlsoap.org/soap/http"/>
        <operation name="Forecast">
          <soap:operation soapAction=""/>
          <input><soap:body use="literal" namespace="urn:weather:rpc" parts="city"/></input>
        </operation>
      </binding>
    </definitions>"#;
    let imported = collection_of(import_text(wsdl));
    assert_eq!(imported.collection.info.name, "WSDL");
    let item = &imported.collection.item[0];
    let request = request_of(item);
    assert_eq!(request.header_value("SOAPAction"), None);
    assert_eq!(request.url.as_ref().unwrap().to_text(), "");
    assert_eq!(
        imported.warnings,
        ["some operations have no service address; fill in their URL"]
    );
    assert!(body_of(item).contains(
        "    <ns1:Forecast xmlns:ns1=\"urn:weather:rpc\">\n      <city>?</city>\n    </ns1:Forecast>"
    ));
}

#[test]
fn other_xml_documents_are_refused_with_a_reason() {
    let refused = |text: &str| match import_text(text) {
        Err(ImportError::UnsupportedFormat(reason)) => reason,
        other => panic!("unexpected {other:?}"),
    };
    assert!(refused(r#"<description xmlns="http://www.w3.org/ns/wsdl"/>"#).contains("WSDL 2.0"));
    assert!(refused("<note/>").contains("not a WSDL"));
    assert!(matches!(
        import_text("<definitions"),
        Err(ImportError::InvalidXml(_))
    ));
}

#[test]
fn header_parts_are_written_into_the_soap_header() {
    let wsdl = r#"<definitions xmlns="http://schemas.xmlsoap.org/wsdl/"
        xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
        xmlns:xsd="http://www.w3.org/2001/XMLSchema"
        xmlns:tns="urn:quotes" targetNamespace="urn:quotes">
      <types>
        <xsd:schema targetNamespace="urn:quotes" elementFormDefault="qualified">
          <xsd:element name="Ticket" type="xsd:string"/>
          <xsd:element name="Quote"><xsd:complexType><xsd:sequence>
            <xsd:element name="symbol" type="xsd:string"/>
          </xsd:sequence></xsd:complexType></xsd:element>
        </xsd:schema>
      </types>
      <message name="QuoteInput">
        <part name="ticket" element="tns:Ticket"/>
        <part name="body" element="tns:Quote"/>
      </message>
      <portType name="Quotes"><operation name="Quote"><input message="tns:QuoteInput"/></operation></portType>
      <binding name="QuotesBinding" type="tns:Quotes">
        <soap:binding style="document" transport="http://schemas.xmlsoap.org/soap/http"/>
        <operation name="Quote">
          <soap:operation soapAction="urn:quotes/Quote"/>
          <input>
            <soap:header message="tns:QuoteInput" part="ticket" use="literal"/>
            <soap:body use="literal"/>
          </input>
        </operation>
      </binding>
      <service name="QuoteService">
        <port name="QuotesPort" binding="tns:QuotesBinding">
          <soap:address location="http://quotes.example.test/soap"/>
        </port>
      </service>
    </definitions>"#;
    let imported = collection_of(import_text(wsdl));
    assert_eq!(imported.collection.info.name, "QuoteService");
    let body = body_of(&imported.collection.item[0]);
    assert!(
        body.contains(
            "  <soap:Header>\n    <ns1:Ticket xmlns:ns1=\"urn:quotes\">?</ns1:Ticket>\n  </soap:Header>\n  <soap:Body>\n    <ns1:Quote xmlns:ns1=\"urn:quotes\">\n      <ns1:symbol>?</ns1:symbol>\n    </ns1:Quote>\n  </soap:Body>"
        ),
        "{body}"
    );
}
