use super::condition::compare;
use crate::script::{Comparison, Condition};

pub(super) fn normalized(condition: Condition) -> Condition {
    match condition {
        Condition::Not(inner) => match normalized(*inner) {
            Condition::Not(twice) => *twice,
            Condition::All(parts) => Condition::Any(negate_each(parts)),
            Condition::Any(parts) => Condition::All(negate_each(parts)),
            other => Condition::Not(Box::new(other)),
        },
        Condition::All(parts) => Condition::All(flattened(parts, |part| match part {
            Condition::All(inner) => Ok(inner),
            other => Err(other),
        })),
        Condition::Any(parts) => Condition::Any(flattened(parts, |part| match part {
            Condition::Any(inner) => Ok(inner),
            other => Err(other),
        })),
        compare => compare,
    }
}

fn negate_each(parts: Vec<Condition>) -> Vec<Condition> {
    parts
        .into_iter()
        .map(|part| normalized(Condition::Not(Box::new(part))))
        .collect()
}

fn flattened(
    parts: Vec<Condition>,
    split: fn(Condition) -> Result<Vec<Condition>, Condition>,
) -> Vec<Condition> {
    parts
        .into_iter()
        .map(normalized)
        .flat_map(|part| split(part).unwrap_or_else(|single| vec![single]))
        .collect()
}

pub(super) fn expressible(condition: Condition) -> Option<Condition> {
    let valid = match &condition {
        Condition::Any(parts) => parts.iter().all(|part| match part {
            Condition::All(inner) => inner.iter().all(is_unary),
            other => is_unary(other),
        }),
        Condition::All(parts) => parts.iter().all(is_unary),
        other => is_unary(other),
    };
    valid.then_some(condition)
}

fn is_unary(condition: &Condition) -> bool {
    match condition {
        Condition::Compare { .. } => true,
        Condition::Not(inner) => is_unary(inner),
        _ => false,
    }
}

pub(super) fn negated(condition: Condition) -> Condition {
    match condition {
        Condition::Compare {
            subject,
            comparison,
            operand,
        } => match comparison.opposite() {
            Some(opposite) => compare(subject, opposite, operand),
            None => Condition::Not(Box::new(compare(subject, comparison, operand))),
        },
        other => Condition::Not(Box::new(other)),
    }
}

impl Comparison {
    fn opposite(self) -> Option<Comparison> {
        Some(match self {
            Comparison::Is => Comparison::IsNot,
            Comparison::IsNot => Comparison::Is,
            Comparison::Contains => Comparison::NotContains,
            Comparison::NotContains => Comparison::Contains,
            Comparison::Exists => Comparison::NotExists,
            Comparison::NotExists => Comparison::Exists,
            Comparison::IsEmpty => Comparison::IsNotEmpty,
            Comparison::IsNotEmpty => Comparison::IsEmpty,
            Comparison::Above | Comparison::Below => return None,
        })
    }
}
