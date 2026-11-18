use std::io::Read;
use std::time::Instant;

use reqwest::blocking::Response;

use super::{Cancellation, StreamProgress, TransportError};
use crate::header::Header;
use crate::response::HttpResponse;
use crate::settings::RequestSettings;

const READ_CHUNK_BYTES: usize = 16 * 1024;
const INITIAL_BODY_CAPACITY: usize = 64 * 1024;
const MAX_PREALLOCATED_BODY_BYTES: usize = 16 * 1024 * 1024;

pub(super) struct ReadWatch<'a> {
    pub(super) deadline: Option<Instant>,
    pub(super) cancellation: &'a Cancellation,
}

impl ReadWatch<'_> {
    fn check(&self) -> Result<(), TransportError> {
        if self.cancellation.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(TransportError::Timeout);
        }
        Ok(())
    }
}

pub(super) fn read_response(
    response: Response,
    settings: &RequestSettings,
    started: Instant,
    watch: ReadWatch<'_>,
    on_stream: &mut dyn FnMut(StreamProgress<'_>),
) -> Result<HttpResponse, TransportError> {
    let status = response.status();
    let mut head = HttpResponse {
        status: status.as_u16(),
        reason: status.canonical_reason().unwrap_or_default().to_string(),
        version: format!("{:?}", response.version()),
        headers: response
            .headers()
            .iter()
            .map(|(name, value)| {
                Header::new(name.as_str(), String::from_utf8_lossy(value.as_bytes()))
            })
            .collect(),
        body: Vec::new(),
        truncated: false,
        elapsed: started.elapsed(),
    };
    let (body, truncated) = if head.is_event_stream() {
        on_stream(StreamProgress::Opened(&head));
        let endless = ReadWatch {
            deadline: None,
            ..watch
        };
        read_stream(response, settings.event_stream_limit(), &endless, on_stream)?
    } else {
        let expected = response
            .content_length()
            .map_or(INITIAL_BODY_CAPACITY, |length| {
                usize::try_from(length).unwrap_or(usize::MAX)
            });
        let limit = settings.response_limit();
        let capacity = expected.min(limit).min(MAX_PREALLOCATED_BODY_BYTES);
        read_capped(response, limit, capacity, &watch)?
    };
    head.body = body;
    head.truncated = truncated;
    head.elapsed = started.elapsed();
    Ok(head)
}

fn read_capped(
    mut reader: impl Read,
    limit: usize,
    capacity: usize,
    watch: &ReadWatch<'_>,
) -> Result<(Vec<u8>, bool), TransportError> {
    let mut body = Vec::with_capacity(capacity);
    let mut chunk = vec![0u8; READ_CHUNK_BYTES];
    loop {
        watch.check()?;
        let count = reader
            .read(&mut chunk)
            .map_err(|error| TransportError::from_io(&error))?;
        if count == 0 {
            return Ok((body, false));
        }
        let room = limit - body.len();
        if count > room {
            body.extend_from_slice(&chunk[..room]);
            return Ok((body, true));
        }
        body.extend_from_slice(&chunk[..count]);
    }
}

fn read_stream(
    mut reader: impl Read,
    limit: usize,
    watch: &ReadWatch<'_>,
    on_stream: &mut dyn FnMut(StreamProgress<'_>),
) -> Result<(Vec<u8>, bool), TransportError> {
    let mut body = Vec::with_capacity(INITIAL_BODY_CAPACITY);
    let mut chunk = vec![0u8; READ_CHUNK_BYTES];
    while watch.check().is_ok() {
        let count = reader
            .read(&mut chunk)
            .map_err(|error| TransportError::from_io(&error))?;
        let taken = count.min(limit - body.len());
        if count == 0 {
            break;
        }
        on_stream(StreamProgress::Received(&chunk[..taken]));
        body.extend_from_slice(&chunk[..taken]);
        if taken < count {
            return Ok((body, true));
        }
    }
    Ok((body, false))
}
