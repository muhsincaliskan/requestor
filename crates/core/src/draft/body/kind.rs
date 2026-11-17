use super::{OPTIONS_LANGUAGE_KEY, OPTIONS_RAW_KEY};
use crate::collection::{Body, BodyMode};
use crate::soap::SoapVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyKind {
    #[default]
    None,
    Raw,
    Urlencoded,
    FormData,
    File,
    GraphQl,
    Soap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RawLanguage {
    #[default]
    Text,
    Json,
    Xml,
    Html,
    JavaScript,
}

impl BodyKind {
    pub const ALL: [BodyKind; 7] = [
        BodyKind::None,
        BodyKind::Raw,
        BodyKind::Urlencoded,
        BodyKind::FormData,
        BodyKind::File,
        BodyKind::GraphQl,
        BodyKind::Soap,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BodyKind::None => "none",
            BodyKind::Raw => "raw",
            BodyKind::Urlencoded => "x-www-form-urlencoded",
            BodyKind::FormData => "form-data",
            BodyKind::File => "binary",
            BodyKind::GraphQl => "GraphQL",
            BodyKind::Soap => "SOAP",
        }
    }

    pub(super) fn from_body(body: &Body) -> Self {
        if !body.is_enabled() {
            return BodyKind::None;
        }
        match body.mode {
            Some(BodyMode::Raw) if body.raw.as_deref().is_some_and(is_soap_envelope) => {
                BodyKind::Soap
            }
            Some(BodyMode::Raw) => BodyKind::Raw,
            Some(BodyMode::Urlencoded) => BodyKind::Urlencoded,
            Some(BodyMode::Formdata) => BodyKind::FormData,
            Some(BodyMode::File) => BodyKind::File,
            Some(BodyMode::Graphql) => BodyKind::GraphQl,
            None => BodyKind::None,
        }
    }

    pub(super) fn mode(self) -> Option<BodyMode> {
        match self {
            BodyKind::None => None,
            BodyKind::Raw | BodyKind::Soap => Some(BodyMode::Raw),
            BodyKind::Urlencoded => Some(BodyMode::Urlencoded),
            BodyKind::FormData => Some(BodyMode::Formdata),
            BodyKind::File => Some(BodyMode::File),
            BodyKind::GraphQl => Some(BodyMode::Graphql),
        }
    }
}

impl RawLanguage {
    pub const ALL: [RawLanguage; 5] = [
        RawLanguage::Text,
        RawLanguage::Json,
        RawLanguage::Xml,
        RawLanguage::Html,
        RawLanguage::JavaScript,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RawLanguage::Text => "Text",
            RawLanguage::Json => "JSON",
            RawLanguage::Xml => "XML",
            RawLanguage::Html => "HTML",
            RawLanguage::JavaScript => "JavaScript",
        }
    }

    pub fn postman_name(self) -> &'static str {
        match self {
            RawLanguage::Text => "text",
            RawLanguage::Json => "json",
            RawLanguage::Xml => "xml",
            RawLanguage::Html => "html",
            RawLanguage::JavaScript => "javascript",
        }
    }

    fn from_postman_name(name: &str) -> Self {
        RawLanguage::ALL
            .into_iter()
            .find(|language| language.postman_name() == name)
            .unwrap_or_default()
    }

    pub(super) fn of_body(body: &Body) -> Self {
        body.options
            .as_ref()
            .and_then(|options| {
                options
                    .get(OPTIONS_RAW_KEY)?
                    .get(OPTIONS_LANGUAGE_KEY)?
                    .as_str()
            })
            .map(RawLanguage::from_postman_name)
            .unwrap_or_default()
    }
}

fn is_soap_envelope(raw: &str) -> bool {
    SoapVersion::of_envelope(raw).is_some()
}
