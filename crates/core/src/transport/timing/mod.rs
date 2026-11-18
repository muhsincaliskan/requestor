mod connector;
mod resolver;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::blocking::{Client, ClientBuilder};

use self::connector::TimingLayer;
use self::resolver::SystemResolver;
use super::TransportError;
use crate::response::ResponseTiming;

pub(super) struct MeasuredClient {
    pub(super) client: Client,
    timings: Arc<ConnectionTimings>,
}

#[derive(Default)]
struct ConnectionTimings {
    pending: Mutex<PendingRequests>,
}

#[derive(Default)]
struct PendingRequests {
    count: usize,
    only: Option<Arc<Measurement>>,
}

struct Measurement(Mutex<Option<ConnectionPhases>>);

#[derive(Default)]
struct ConnectionPhases {
    dns: Option<Duration>,
    connect: Option<Duration>,
}

pub(super) struct RequestTiming<'a> {
    started: Instant,
    timings: &'a ConnectionTimings,
    measurement: Option<Arc<Measurement>>,
}

impl MeasuredClient {
    pub(super) fn build(builder: ClientBuilder) -> Result<Self, TransportError> {
        let timings = Arc::new(ConnectionTimings::default());
        let client = builder
            .dns_resolver(Arc::new(SystemResolver))
            .connector_layer(TimingLayer(Arc::clone(&timings)))
            .build()
            .map_err(|error| TransportError::InvalidSettings(error.without_url().to_string()))?;
        Ok(MeasuredClient { client, timings })
    }

    pub(super) fn measure(&self, started: Instant) -> RequestTiming<'_> {
        self.timings.begin(started)
    }
}

impl ConnectionTimings {
    fn begin(&self, started: Instant) -> RequestTiming<'_> {
        let mut pending = self.pending.lock().unwrap();
        pending.count += 1;
        if let Some(previous) = pending.only.take() {
            previous.take();
        }
        let measurement = (pending.count == 1)
            .then(|| Arc::new(Measurement(Mutex::new(Some(ConnectionPhases::default())))));
        pending.only = measurement.clone();
        RequestTiming {
            started,
            timings: self,
            measurement,
        }
    }

    fn current(&self) -> Option<Arc<Measurement>> {
        self.pending.lock().unwrap().only.clone()
    }
}

impl Measurement {
    fn take(&self) -> Option<ConnectionPhases> {
        self.0.lock().unwrap().take()
    }

    fn record(&self, elapsed: Duration, dns: Option<Duration>) {
        let mut measurement = self.0.lock().unwrap();
        if let Some(phases) = measurement.as_mut() {
            phases.connect = Some(
                phases.connect.unwrap_or_default()
                    + elapsed.saturating_sub(dns.unwrap_or_default()),
            );
            if let Some(dns) = dns {
                phases.dns = Some(phases.dns.unwrap_or_default() + dns);
            }
        }
    }
}

impl RequestTiming<'_> {
    pub(super) fn received_headers(&self, received: Instant) -> ResponseTiming {
        let phases = self
            .measurement
            .as_deref()
            .and_then(Measurement::take)
            .unwrap_or_default();
        ResponseTiming {
            dns: phases.dns,
            connect: phases.connect,
            first_byte: received - self.started,
            download: Duration::ZERO,
        }
    }
}

impl Drop for RequestTiming<'_> {
    fn drop(&mut self) {
        let mut pending = self.timings.pending.lock().unwrap();
        pending.count -= 1;
        if pending.count == 0 {
            pending.only = None;
        }
    }
}

#[cfg(test)]
mod tests;
