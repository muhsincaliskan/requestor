use serde_json::Number;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Expr {
    Name(String),
    Text(String),
    Number(Number),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    New(Box<Expr>),
    Not(Box<Expr>),
    Negate(Box<Expr>),
    Group(Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Function(Body),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Body {
    Expression(Box<Expr>),
    Block(Vec<Located>),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum JsStatement {
    Expression(Expr),
    Declare {
        name: String,
        value: Expr,
    },
    If {
        condition: Expr,
        then: Vec<Located>,
        otherwise: Option<Vec<Located>>,
    },
    Throw(Expr),
    Comment(String),
    Blank,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Located {
    pub(super) statement: JsStatement,
    pub(super) first_line: usize,
    pub(super) last_line: usize,
}

impl Expr {
    pub(super) fn path(&self) -> Option<String> {
        match self {
            Expr::Name(name) => Some(name.clone()),
            Expr::Member(object, field) => Some(format!("{}.{field}", object.path()?)),
            _ => None,
        }
    }

    pub(super) fn call_of(&self, path: &str) -> Option<&[Expr]> {
        match self {
            Expr::Call(callee, arguments) if callee.path().as_deref() == Some(path) => {
                Some(arguments)
            }
            _ => None,
        }
    }

    pub(super) fn method_call(&self, method: &str) -> Option<(&Expr, &[Expr])> {
        match self {
            Expr::Call(callee, arguments) => match callee.as_ref() {
                Expr::Member(object, name) if name == method => Some((object, arguments)),
                _ => None,
            },
            _ => None,
        }
    }

    pub(super) fn text(&self) -> Option<&str> {
        match self {
            Expr::Text(text) => Some(text),
            _ => None,
        }
    }

    pub(super) fn ungrouped(&self) -> &Expr {
        match self {
            Expr::Group(inner) => inner.ungrouped(),
            other => other,
        }
    }
}
