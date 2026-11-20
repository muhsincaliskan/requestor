use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{PacConfig, PacEvaluator, PacTarget, ProxyDecision};

const DECISION_LIFETIME: Duration = Duration::from_secs(600);
const FAILURE_LIFETIME: Duration = Duration::from_secs(30);

static SHARED: Mutex<Option<Arc<PacResolver>>> = Mutex::new(None);

pub struct PacResolver {
    config: PacConfig,
    evaluator: Arc<dyn PacEvaluator>,
    decisions: Mutex<HashMap<PacTarget, KnownDecision>>,
    evaluating: Mutex<()>,
}

struct KnownDecision {
    decision: ProxyDecision,
    expires: Instant,
}

impl PacResolver {
    pub fn new(config: PacConfig, evaluator: Arc<dyn PacEvaluator>) -> Self {
        PacResolver {
            config,
            evaluator,
            decisions: Mutex::new(HashMap::new()),
            evaluating: Mutex::new(()),
        }
    }

    pub fn system() -> Option<Arc<Self>> {
        let evaluator = super::evaluator::system()?;
        let config = evaluator.config()?;
        let mut shared = SHARED.lock().ok()?;
        match shared.as_ref() {
            Some(resolver) if resolver.config == config => Some(Arc::clone(resolver)),
            _ => {
                let resolver = Arc::new(PacResolver::new(config, evaluator));
                *shared = Some(Arc::clone(&resolver));
                Some(resolver)
            }
        }
    }

    pub fn decide(&self, target: &PacTarget) -> ProxyDecision {
        if self.config.bypasses(&target.host) {
            return ProxyDecision::Direct;
        }
        if let Some(known) = self.known(target) {
            return known;
        }
        let _evaluating = self.evaluating.lock();
        if let Some(known) = self.known(target) {
            return known;
        }
        let result = self.evaluator.find_proxy(&self.config.script_url, target);
        let lifetime = if result.is_some() {
            DECISION_LIFETIME
        } else {
            FAILURE_LIFETIME
        };
        let decision = result.map_or(ProxyDecision::Direct, |result| {
            ProxyDecision::from_pac_result(&result)
        });
        self.remember(target, &decision, lifetime);
        decision
    }

    fn known(&self, target: &PacTarget) -> Option<ProxyDecision> {
        let decisions = self.decisions.lock().ok()?;
        decisions
            .get(target)
            .filter(|known| known.expires > Instant::now())
            .map(|known| known.decision.clone())
    }

    fn remember(&self, target: &PacTarget, decision: &ProxyDecision, lifetime: Duration) {
        if let Ok(mut decisions) = self.decisions.lock() {
            decisions.insert(
                target.clone(),
                KnownDecision {
                    decision: decision.clone(),
                    expires: Instant::now() + lifetime,
                },
            );
        }
    }
}
