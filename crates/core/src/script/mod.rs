mod complete;
mod dynamic;
mod js;
mod literal;
mod parse;
mod path;
mod print;
mod report;
mod run;
mod stored;

use serde_json::Value;

pub use complete::Completion;
pub use dynamic::Dynamic;
pub use js::{Unconverted, UnconvertedLine};
pub use parse::ScriptError;
pub use path::JsonPath;
pub use report::{CheckOutcome, ScriptReport, VariableWrite, Written};
pub use run::ScriptContext;
pub use stored::{RequestScripts, StoredScript};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    BeforeRequest,
    AfterResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VariableTarget {
    #[default]
    Environment,
    Collection,
    Request,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    Status,
    Time,
    Body,
    Header(String),
    Json(JsonPath),
    Variable(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Literal(Value),
    Variable(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Is,
    IsNot,
    Contains,
    NotContains,
    Above,
    Below,
    Exists,
    NotExists,
    IsEmpty,
    IsNotEmpty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Compare {
        subject: Subject,
        comparison: Comparison,
        operand: Option<Operand>,
    },
    Not(Box<Condition>),
    All(Vec<Condition>),
    Any(Vec<Condition>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetValue {
    Literal(Value),
    Dynamic(Dynamic),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Blank,
    Comment(String),
    Set {
        variable: String,
        value: SetValue,
        target: VariableTarget,
    },
    Clear {
        variable: String,
        target: VariableTarget,
    },
    Expect {
        condition: Condition,
        name: Option<String>,
    },
    Save {
        subject: Subject,
        variable: String,
        target: VariableTarget,
    },
    Fail(String),
    If(Conditional),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    pub condition: Condition,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conditional {
    pub branches: Vec<Branch>,
    pub otherwise: Option<Vec<Statement>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Script {
    pub statements: Vec<Statement>,
}

impl Script {
    pub fn is_empty(&self) -> bool {
        self.statements
            .iter()
            .all(|statement| matches!(statement, Statement::Blank))
    }
}

impl VariableTarget {
    pub fn persists(self) -> bool {
        self != VariableTarget::Request
    }
}

impl Subject {
    pub fn needs_response(&self) -> bool {
        !matches!(self, Subject::Variable(_))
    }
}

impl Comparison {
    pub fn takes_operand(self) -> bool {
        !matches!(
            self,
            Comparison::Exists
                | Comparison::NotExists
                | Comparison::IsEmpty
                | Comparison::IsNotEmpty
        )
    }
}
