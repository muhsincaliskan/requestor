use std::path::Path;

use requestor_core::collection::Collection;
use requestor_core::error::CoreError;
use requestor_core::settings::RequestSettings;

use super::overlay::{ExportForm, LibraryTarget};

const HTTP_EXTENSION: &str = "http";
const HTTP_SUFFIX: &str = ".http";
const POSTMAN_SUFFIX: &str = ".postman_collection.json";
const JSON_SUFFIX: &str = ".json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Postman,
    Http,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 2] = [ExportFormat::Postman, ExportFormat::Http];

    pub fn of(path: &Path) -> Self {
        if path
            .extension()
            .is_some_and(|extension| extension == HTTP_EXTENSION)
        {
            ExportFormat::Http
        } else {
            ExportFormat::Postman
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Postman => "Postman collection",
            ExportFormat::Http => ".http file",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            ExportFormat::Postman => "Opens in Postman and Requestor",
            ExportFormat::Http => {
                "One file with every request, for VS Code REST Client and JetBrains"
            }
        }
    }

    pub fn render(
        self,
        collection: &Collection,
        settings: &RequestSettings,
    ) -> Result<String, CoreError> {
        match self {
            ExportFormat::Postman => collection.to_export_json(),
            ExportFormat::Http => Ok(collection.to_http_file(settings)),
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            ExportFormat::Postman => POSTMAN_SUFFIX,
            ExportFormat::Http => HTTP_SUFFIX,
        }
    }

    fn stem(path: &str) -> &str {
        [POSTMAN_SUFFIX, HTTP_SUFFIX, JSON_SUFFIX]
            .iter()
            .find_map(|suffix| path.strip_suffix(suffix))
            .unwrap_or(path)
    }
}

impl ExportForm {
    pub fn offers_formats(&self) -> bool {
        !matches!(self.target, LibraryTarget::Environment { .. })
    }

    pub fn format(&self) -> ExportFormat {
        ExportFormat::of(Path::new(self.file_path.trim()))
    }

    pub fn set_format(&mut self, format: ExportFormat) {
        let stem = ExportFormat::stem(self.file_path.trim());
        self.file_path = format!("{stem}{}", format.suffix());
    }
}
