use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use tower_layer::Layer;
use tower_service::Service;

use super::resolver::DnsTiming;
use super::ConnectionTimings;

#[derive(Clone)]
pub(super) struct TimingLayer(pub(super) Arc<ConnectionTimings>);

#[derive(Clone)]
pub(super) struct TimingConnector<S> {
    inner: S,
    timings: Arc<ConnectionTimings>,
}

type ConnectorFuture<Response, Error> =
    Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>>;

impl<S> Layer<S> for TimingLayer {
    type Service = TimingConnector<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TimingConnector {
            inner,
            timings: Arc::clone(&self.0),
        }
    }
}

impl<S, Request> Service<Request> for TimingConnector<S>
where
    S: Service<Request>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ConnectorFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(context)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let Some(measurement) = self.timings.current() else {
            return Box::pin(self.inner.call(request));
        };
        let started = Instant::now();
        let dns = DnsTiming::default();
        let mut operation = dns.scope(|| Box::pin(self.inner.call(request)));
        Box::pin(async move {
            let result = poll_fn(|context| dns.scope(|| operation.as_mut().poll(context))).await;
            if result.is_ok() {
                measurement.record(started.elapsed(), dns.elapsed());
            }
            result
        })
    }
}
