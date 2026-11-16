use std::path::Path;

use super::body::EncodedBody;
use crate::collection::Param;
use crate::error::CoreError;
use crate::identity::random_u64;
use crate::variables::VariableScope;

const FILE_KIND: &str = "file";
const DEFAULT_FILE_CONTENT_TYPE: &str = "application/octet-stream";
const LINE_BREAK: &str = "\r\n";

pub(super) fn encode(params: &[&Param], scope: &VariableScope) -> Result<EncodedBody, CoreError> {
    let boundary = format!("----requestor{:016x}{:016x}", random_u64(), random_u64());
    let mut data = Vec::new();
    for param in params {
        data.extend_from_slice(format!("--{boundary}{LINE_BREAK}").as_bytes());
        write_part(&mut data, param, scope)?;
        data.extend_from_slice(LINE_BREAK.as_bytes());
    }
    data.extend_from_slice(format!("--{boundary}--{LINE_BREAK}").as_bytes());
    Ok(EncodedBody {
        content_type: Some(format!("multipart/form-data; boundary={boundary}")),
        data,
    })
}

fn write_part(data: &mut Vec<u8>, param: &Param, scope: &VariableScope) -> Result<(), CoreError> {
    let name = scope.resolve(param.key.as_deref().unwrap_or_default()).text;
    if param.kind.as_deref() == Some(FILE_KIND) {
        return write_file_part(data, param, &name);
    }
    let value = scope
        .resolve(param.value.as_deref().unwrap_or_default())
        .text;
    data.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{name}\"{LINE_BREAK}").as_bytes(),
    );
    if let Some(content_type) = &param.content_type {
        data.extend_from_slice(format!("Content-Type: {content_type}{LINE_BREAK}").as_bytes());
    }
    data.extend_from_slice(LINE_BREAK.as_bytes());
    data.extend_from_slice(value.as_bytes());
    Ok(())
}

fn write_file_part(data: &mut Vec<u8>, param: &Param, name: &str) -> Result<(), CoreError> {
    let path = param
        .src
        .as_ref()
        .map(|source| source.joined(""))
        .unwrap_or_default();
    let contents = std::fs::read(&path).map_err(|error| CoreError::UnreadableFile {
        path: path.clone(),
        reason: error.to_string(),
    })?;
    let file_name = Path::new(&path)
        .file_name()
        .map(|file_name| file_name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let content_type = param
        .content_type
        .as_deref()
        .unwrap_or(DEFAULT_FILE_CONTENT_TYPE);
    data.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{name}\"; filename=\"{file_name}\"{LINE_BREAK}Content-Type: {content_type}{LINE_BREAK}{LINE_BREAK}"
        )
        .as_bytes(),
    );
    data.extend_from_slice(&contents);
    Ok(())
}
