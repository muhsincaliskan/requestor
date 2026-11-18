use requestor_core::graphql::GraphQlSchema;

#[derive(Debug, Default)]
pub enum SchemaLookup {
    #[default]
    Idle,
    Fetching(u64),
    Ready(GraphQlSchema),
    Failed(String),
}

#[derive(Debug, Default)]
pub struct SchemaView {
    pub lookup: SchemaLookup,
    pub filter: String,
}

impl SchemaView {
    pub fn start(&mut self, job: u64) {
        self.lookup = SchemaLookup::Fetching(job);
    }

    pub fn fail(&mut self, reason: String) {
        self.lookup = SchemaLookup::Failed(reason);
    }

    pub fn finish(&mut self, job: u64, result: Result<GraphQlSchema, String>) {
        if !matches!(self.lookup, SchemaLookup::Fetching(running) if running == job) {
            return;
        }
        self.lookup = match result {
            Ok(schema) => SchemaLookup::Ready(schema),
            Err(reason) => SchemaLookup::Failed(reason),
        };
    }

    pub fn cancel(&mut self) -> Option<u64> {
        let SchemaLookup::Fetching(job) = self.lookup else {
            return None;
        };
        self.lookup = SchemaLookup::Idle;
        Some(job)
    }

    pub fn is_fetching(&self) -> bool {
        matches!(self.lookup, SchemaLookup::Fetching(_))
    }
}
