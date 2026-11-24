use super::super::lexer::Kind;
use super::super::syntax::{Body, Expr};
use super::{Failure, Parser, LEVELS};

impl Parser {
    pub(super) fn expression(&mut self) -> Result<Expr, Failure> {
        let target = self.binary(0)?;
        if self.eat("=") {
            let value = self.expression()?;
            return Ok(Expr::Binary("=", Box::new(target), Box::new(value)));
        }
        Ok(target)
    }

    pub(super) fn binary(&mut self, level: usize) -> Result<Expr, Failure> {
        let Some(operators) = LEVELS.get(level) else {
            return self.unary();
        };
        let mut left = self.binary(level + 1)?;
        while let Some(operator) = operators.iter().find(|operator| self.eat(operator)) {
            let right = self.binary(level + 1)?;
            left = Expr::Binary(operator, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    pub(super) fn unary(&mut self) -> Result<Expr, Failure> {
        if self.eat("!") {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        if self.eat("-") {
            return Ok(Expr::Negate(Box::new(self.unary()?)));
        }
        let primary = self.primary()?;
        self.postfix(primary)
    }

    pub(super) fn postfix(&mut self, mut value: Expr) -> Result<Expr, Failure> {
        loop {
            if self.eat(".") || self.eat("?.") {
                value = Expr::Member(Box::new(value), self.name()?);
            } else if self.eat("[") {
                let index = self.expression()?;
                self.expect("]")?;
                value = Expr::Index(Box::new(value), Box::new(index));
            } else if self.eat("(") {
                value = Expr::Call(Box::new(value), self.arguments()?);
            } else {
                return Ok(value);
            }
        }
    }

    pub(super) fn arguments(&mut self) -> Result<Vec<Expr>, Failure> {
        let mut found = Vec::new();
        while !self.eat(")") {
            found.push(self.expression()?);
            if !self.eat(",") {
                self.expect(")")?;
                break;
            }
        }
        Ok(found)
    }

    pub(super) fn primary(&mut self) -> Result<Expr, Failure> {
        let Some(token) = self.next() else {
            return Err(self.failure("the script ends too early"));
        };
        match token.kind {
            Kind::Text(text) => Ok(Expr::Text(text)),
            Kind::Number(number) => Ok(Expr::Number(number)),
            Kind::Punct("(") => self.parenthesis(),
            Kind::Name(name) => self.named(name),
            _ => Err((
                token.line,
                "this kind of JavaScript is not supported".to_string(),
            )),
        }
    }

    pub(super) fn named(&mut self, name: String) -> Result<Expr, Failure> {
        match name.as_str() {
            "function" => self.function(),
            "new" => self.construct(),
            _ if self.eat("=>") => self.arrow_body(),
            _ => Ok(Expr::Name(name)),
        }
    }

    pub(super) fn construct(&mut self) -> Result<Expr, Failure> {
        let mut target = Expr::Name(self.name()?);
        while self.eat(".") {
            target = Expr::Member(Box::new(target), self.name()?);
        }
        let arguments = if self.eat("(") {
            self.arguments()?
        } else {
            Vec::new()
        };
        Ok(Expr::New(Box::new(Expr::Call(Box::new(target), arguments))))
    }

    pub(super) fn function(&mut self) -> Result<Expr, Failure> {
        if matches!(self.peek().map(|token| &token.kind), Some(Kind::Name(_))) {
            self.position += 1;
        }
        self.expect("(")?;
        self.skip_parameters()?;
        Ok(Expr::Function(Body::Block(self.block()?)))
    }

    pub(super) fn parenthesis(&mut self) -> Result<Expr, Failure> {
        if self.is_arrow_parameters() {
            self.skip_parameters()?;
            self.expect("=>")?;
            return self.arrow_body();
        }
        let inner = self.expression()?;
        self.expect(")")?;
        Ok(Expr::Group(Box::new(inner)))
    }

    pub(super) fn arrow_body(&mut self) -> Result<Expr, Failure> {
        if self.at("{") {
            return Ok(Expr::Function(Body::Block(self.block()?)));
        }
        Ok(Expr::Function(Body::Expression(Box::new(
            self.expression()?,
        ))))
    }

    pub(super) fn is_arrow_parameters(&self) -> bool {
        let mut offset = self.position;
        while let Some(token) = self.tokens.get(offset) {
            match &token.kind {
                Kind::Punct(")") => {
                    return matches!(
                        self.tokens.get(offset + 1).map(|next| &next.kind),
                        Some(Kind::Punct("=>"))
                    )
                }
                Kind::Name(_) | Kind::Punct(",") => offset += 1,
                _ => return false,
            }
        }
        false
    }

    pub(super) fn skip_parameters(&mut self) -> Result<(), Failure> {
        while !self.eat(")") {
            self.name()?;
            self.eat(",");
        }
        Ok(())
    }
}
