use std::cell::RefCell;
use std::future::poll_fn;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

const DNS_WORKER: &str = "requestor-dns";
const UNSPECIFIED_PORT: u16 = 0;

#[derive(Clone, Default)]
pub(super) struct DnsTiming(Arc<Mutex<Option<Duration>>>);

thread_local! {
    static CONNECTION_DNS: RefCell<Option<DnsTiming>> = const { RefCell::new(None) };
}

struct DnsScope(Option<DnsTiming>);

#[derive(Default)]
struct Lookup {
    addresses: Option<io::Result<Vec<SocketAddr>>>,
    waker: Option<Waker>,
}

pub(super) struct SystemResolver;

impl DnsTiming {
    pub(super) fn scope<T>(&self, operation: impl FnOnce() -> T) -> T {
        let _scope = DnsScope(CONNECTION_DNS.replace(Some(self.clone())));
        operation()
    }

    pub(super) fn elapsed(&self) -> Option<Duration> {
        *self.0.lock().unwrap()
    }

    fn record(&self, elapsed: Duration) {
        let mut total = self.0.lock().unwrap();
        *total = Some(total.unwrap_or_default() + elapsed);
    }
}

impl Drop for DnsScope {
    fn drop(&mut self) {
        CONNECTION_DNS.replace(self.0.take());
    }
}

impl Resolve for SystemResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let timing = CONNECTION_DNS.with(|current| current.borrow().clone());
        Box::pin(async move {
            let started = Instant::now();
            let lookup = Lookup::start(name.as_str().to_owned())?;
            let addresses = poll_fn(|context| lookup.lock().unwrap().poll(context.waker())).await;
            if let Some(timing) = timing {
                timing.record(started.elapsed());
            }
            Ok(Box::new(addresses?.into_iter()) as Addrs)
        })
    }
}

impl Lookup {
    fn start(host: String) -> io::Result<Arc<Mutex<Self>>> {
        let pending = Arc::new(Mutex::new(Lookup::default()));
        let completed = Arc::clone(&pending);
        thread::Builder::new()
            .name(DNS_WORKER.to_string())
            .spawn(move || {
                let addresses = (host.as_str(), UNSPECIFIED_PORT)
                    .to_socket_addrs()
                    .map(Iterator::collect);
                let waker = completed.lock().unwrap().complete(addresses);
                if let Some(waker) = waker {
                    waker.wake();
                }
            })?;
        Ok(pending)
    }

    fn complete(&mut self, addresses: io::Result<Vec<SocketAddr>>) -> Option<Waker> {
        self.addresses = Some(addresses);
        self.waker.take()
    }

    fn poll(&mut self, waker: &Waker) -> Poll<io::Result<Vec<SocketAddr>>> {
        match self.addresses.take() {
            Some(addresses) => Poll::Ready(addresses),
            None => {
                self.waker = Some(waker.clone());
                Poll::Pending
            }
        }
    }
}
