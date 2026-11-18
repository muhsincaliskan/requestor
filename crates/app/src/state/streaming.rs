use requestor_core::response::HttpResponse;

use super::sections::ResponseSection;
use super::stream::StreamUpdate;
use super::tab::TabId;
use super::AppState;

const STREAM_ENDED: &str = "The event stream stopped:";

impl AppState {
    pub(super) fn stream_progress(&mut self, id: TabId, job: u64, update: StreamUpdate) {
        if let Some(tab) = self.tab_mut(id) {
            if matches!(update, StreamUpdate::Opened(_)) {
                tab.response_section = ResponseSection::Events;
            }
            tab.exchange.stream(job, update);
        }
    }

    pub(super) fn finish_send(
        &mut self,
        id: TabId,
        job: u64,
        result: Result<HttpResponse, String>,
    ) {
        let result = match result {
            Err(reason) => match self.received_before(id, job) {
                Some(received) => {
                    self.warn(format!("{STREAM_ENDED} {reason}"));
                    Ok(received)
                }
                None => Err(reason),
            },
            answered => answered,
        };
        self.send_finished(id, job, result);
    }

    fn received_before(&mut self, id: TabId, job: u64) -> Option<HttpResponse> {
        let (live_job, stream) = self.tab_mut(id)?.exchange.take_live()?;
        (live_job == job).then(|| stream.into_response())
    }

    pub(super) fn cancel_send(&mut self, id: TabId) {
        let live = self.tab_mut(id).and_then(|tab| tab.exchange.take_live());
        match live {
            Some((job, stream)) => {
                self.send_finished(id, job, Ok(stream.into_response()));
                self.abandoned_jobs.push(job);
            }
            None => self.abandon_send(id),
        }
    }
}
