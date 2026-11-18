use crate::response::ResponseView;

pub enum Exchange {
    Idle,
    Sending { job: u64, previous: Option<String> },
    Completed(Box<ResponseView>),
    Failed(String),
}

impl Exchange {
    pub fn followed_by_send(&self, job: u64) -> Exchange {
        let previous = match self {
            Exchange::Completed(view) => view.comparison_text(),
            Exchange::Sending { previous, .. } => previous.clone(),
            Exchange::Idle | Exchange::Failed(_) => None,
        };
        Exchange::Sending { job, previous }
    }

    pub fn take_previous(&mut self) -> Option<String> {
        match self {
            Exchange::Sending { previous, .. } => previous.take(),
            _ => None,
        }
    }
}
