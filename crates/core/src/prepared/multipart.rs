use std::path::Path;

use super::body::EncodedBody;
use crate::collection::Param;
use crate::error::CoreError;
use crate::identity::random_u64;
use crate::media_type;
use crate::variables::VariableScope;

const FILE_KIND: &str = "file";
const LINE_BREAK: &str = "\r\n";
const BOUNDARY_PREFIX: &str = "----requestor";
const ESCAPED_REPLACEMENTS: [(char, &str); 3] = [('"', "%22"), ('\r', "%0D"), ('\n', "%0A")];

pub(super) fn encode(params: &[&Param], scope: &VariableScope) -> Result<EncodedBody, CoreError> {
    let boundary = format!(
        "{BOUNDARY_PREFIX}{:016x}{:016x}",
        random_u64(),
        random_u64()
    );
    let mut data = Vec::new();
    for param in params {
        write_param(&mut data, &boundary, param, scope)?;
    }
    data.extend_from_slice(format!("--{boundary}--{LINE_BREAK}").as_bytes());
    Ok(EncodedBody {
        content_type: Some(format!(
            "{}; boundary={boundary}",
            media_type::MULTIPART_FORM_DATA
        )),
        data,
    })
}

fn write_param(
    data: &mut Vec<u8>,
    boundary: &str,
    param: &Param,
    scope: &VariableScope,
) -> Result<(), CoreError> {
    let name = escape(&scope.resolve(param.key.as_deref().unwrap_or_default()).text);
    let content_type = checked_content_type(param)?;
    if param.kind.as_deref() == Some(FILE_KIND) {
        return write_files(data, boundary, param, &name, content_type);
    }
    let value = scope
        .resolve(param.value.as_deref().unwrap_or_default())
        .text;
    write_part(
        data,
        boundary,
        &format!("name=\"{name}\""),
        content_type,
        value.as_bytes(),
    );
    Ok(())
}

fn write_files(
    data: &mut Vec<u8>,
    boundary: &str,
    param: &Param,
    name: &str,
    content_type: Option<&str>,
) -> Result<(), CoreError> {
    let paths = param
        .src
        .as_ref()
        .map(|source| source.items())
        .unwrap_or_default();
    for path in paths.into_iter().filter(|path| !path.is_empty()) {
        let contents = std::fs::read(path).map_err(|error| CoreError::UnreadableFile {
            path: path.to_string(),
            reason: error.to_string(),
        })?;
        let file_name = Path::new(path)
            .file_name()
            .map(|file_name| escape(&file_name.to_string_lossy()))
            .unwrap_or_default();
        let disposition = format!("name=\"{name}\"; filename=\"{file_name}\"");
        let content_type = content_type.unwrap_or(media_type::OCTET_STREAM);
        write_part(data, boundary, &disposition, Some(content_type), &contents);
    }
    Ok(())
}

fn write_part(
    data: &mut Vec<u8>,
    boundary: &str,
    disposition: &str,
    content_type: Option<&str>,
    contents: &[u8],
) {
    data.extend_from_slice(
        format!(
            "--{boundary}{LINE_BREAK}Content-Disposition: form-data; {disposition}{LINE_BREAK}"
        )
        .as_bytes(),
    );
    if let Some(content_type) = content_type {
        data.extend_from_slice(format!("Content-Type: {content_type}{LINE_BREAK}").as_bytes());
    }
    data.extend_from_slice(LINE_BREAK.as_bytes());
    data.extend_from_slice(contents);
    data.extend_from_slice(LINE_BREAK.as_bytes());
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
