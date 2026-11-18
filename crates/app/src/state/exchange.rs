use crate::response::{ComparisonBase, ResponseView};

pub enum Exchange {
    Idle,
    Sending {
        job: u64,
        previous: Option<ComparisonBase>,
    },
    Completed(Box<ResponseView>),
    Failed(String),
}

impl Exchange {
    pub fn start_send(&mut self, job: u64) -> Option<u64> {
        let (replaced, previous) = match std::mem::replace(self, Exchange::Idle) {
            Exchange::Completed(view) => (None, view.into_comparison_base()),
            Exchange::Sending { job, previous } => (Some(job), previous),
            Exchange::Idle | Exchange::Failed(_) => (None, None),
        };
        *self = Exchange::Sending { job, previous };
        replaced
    }

    pub fn take_previous(&mut self) -> Option<ComparisonBase> {
        match self {
            Exchange::Sending { previous, .. } => previous.take(),
            _ => None,
        }
    }
}
