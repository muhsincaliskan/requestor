use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{PacConfig, PacEvaluator, PacTarget, ProxyDecision};

pub struct PacResolver {
    config: PacConfig,
    evaluator: Arc<dyn PacEvaluator>,
    decisions: Mutex<HashMap<PacTarget, ProxyDecision>>,
}

impl PacResolver {
    pub fn new(config: PacConfig, evaluator: Arc<dyn PacEvaluator>) -> Self {
        PacResolver {
            config,
            evaluator,
            decisions: Mutex::new(HashMap::new()),
        }
    }

    pub fn system() -> Option<Self> {
        let evaluator = super::evaluator::system()?;
        Some(PacResolver::new(evaluator.config()?, evaluator))
    }

    pub fn decide(&self, target: &PacTarget) -> ProxyDecision {
        if self.config.bypasses(&target.host) {
            return ProxyDecision::Direct;
        }
        if let Some(known) = self.known(target) {
            return known;
        }
        let decision = self
            .evaluator
            .find_proxy(&self.config.script_url, target)
            .map_or(ProxyDecision::Direct, |result| {
                ProxyDecision::from_pac_result(&result)
            });
        if let Ok(mut decisions) = self.decisions.lock() {
            decisions.insert(target.clone(), decision.clone());
        }
        decision
    }

    fn known(&self, target: &PacTarget) -> Option<ProxyDecision> {
        self.decisions.lock().ok()?.get(target).cloned()
    }
}
