use std::fs::File;
use std::io;
use std::path::Path;

use super::body::EncodedBody;
use super::body_source::{BodySource, FormField};
use crate::collection::Param;
use crate::error::CoreError;
use crate::identity::random_u64;
use crate::media_type;
use crate::variables::VariableScope;

const LINE_BREAK: &str = "\r\n";
const BOUNDARY_PREFIX: &str = "----requestor";
const ESCAPED_REPLACEMENTS: [(char, &str); 3] = [('"', "%22"), ('\r', "%0D"), ('\n', "%0A")];

struct Writer<'a> {
    boundary: String,
    data: Vec<u8>,
    fields: Vec<FormField>,
    scope: &'a VariableScope,
}

pub(super) fn encode(params: &[&Param], scope: &VariableScope) -> Result<EncodedBody, CoreError> {
    let mut writer = Writer {
        boundary: format!(
            "{BOUNDARY_PREFIX}{:016x}{:016x}",
            random_u64(),
            random_u64()
        ),
        data: Vec::new(),
        fields: Vec::new(),
        scope,
    };
    for param in params {
        writer.write_param(param)?;
    }
    Ok(writer.finish())
}

impl Writer<'_> {
    fn finish(mut self) -> EncodedBody {
        let closing = format!("--{}--{LINE_BREAK}", self.boundary);
        self.data.extend_from_slice(closing.as_bytes());
        EncodedBody {
            content_type: Some(format!(
                "{}; boundary={}",
                media_type::MULTIPART_FORM_DATA,
                self.boundary
            )),
            data: self.data,
            source: BodySource::Form(self.fields),
        }
    }

    fn write_param(&mut self, param: &Param) -> Result<(), CoreError> {
        let raw_name = self
            .scope
            .resolve(param.key.as_deref().unwrap_or_default())
            .text;
        let content_type = checked_content_type(param)?;
        if param.is_file() {
            return self.write_files(param, &raw_name, content_type);
        }
        let value = self
            .scope
            .resolve(param.value.as_deref().unwrap_or_default())
            .text;
        let disposition = format!("name=\"{}\"", escape(&raw_name));
        self.start_part(&disposition, content_type);
        self.data.extend_from_slice(value.as_bytes());
        self.end_part();
        self.fields.push(FormField::text(raw_name, value));
        Ok(())
    }

    fn write_files(
        &mut self,
        param: &Param,
        raw_name: &str,
        content_type: Option<&str>,
    ) -> Result<(), CoreError> {
        let name = escape(raw_name);
        let paths = param
            .src
            .as_ref()
            .map(|source| source.items())
            .unwrap_or_default();
        for path in paths.into_iter().filter(|path| !path.is_empty()) {
            let unreadable = |error: io::Error| CoreError::UnreadableFile {
                path: path.to_string(),
                reason: error.to_string(),
            };
            let mut file = File::open(path).map_err(unreadable)?;
            let file_name = Path::new(path)
                .file_name()
                .map(|file_name| escape(&file_name.to_string_lossy()))
                .unwrap_or_default();
            let disposition = format!("name=\"{name}\"; filename=\"{file_name}\"");
            let part_type = content_type.unwrap_or(media_type::OCTET_STREAM);
            self.start_part(&disposition, Some(part_type));
            io::copy(&mut file, &mut self.data).map_err(unreadable)?;
            self.end_part();
            self.fields
                .push(FormField::file(raw_name, path, content_type));
        }
        Ok(())
    }

    fn start_part(&mut self, disposition: &str, content_type: Option<&str>) {
        let head = format!(
            "--{}{LINE_BREAK}Content-Disposition: form-data; {disposition}{LINE_BREAK}",
            self.boundary
        );
        self.data.extend_from_slice(head.as_bytes());
        if let Some(content_type) = content_type {
            let line = format!("Content-Type: {content_type}{LINE_BREAK}");
            self.data.extend_from_slice(line.as_bytes());
        }
        self.data.extend_from_slice(LINE_BREAK.as_bytes());
    }

    fn end_part(&mut self) {
        self.data.extend_from_slice(LINE_BREAK.as_bytes());
    }
}

fn checked_content_type(param: &Param) -> Result<Option<&str>, CoreError> {
    match param.content_type.as_deref() {
        Some(value) if value.contains(['\r', '\n']) => Err(CoreError::InvalidFormField(
            param.key.clone().unwrap_or_default(),
        )),
        other => Ok(other),
    }
}

fn escape(text: &str) -> String {
    ESCAPED_REPLACEMENTS
        .iter()
        .fold(text.to_string(), |escaped, (character, replacement)| {
            escaped.replace(*character, replacement)
        })
}
