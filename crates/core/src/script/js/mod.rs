mod export;
mod lexer;
mod parser;
mod recognize;
mod syntax;
mod title;

use parser::Parser;
use recognize::Recognizer;

use super::Script;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnconvertedLine {
    pub line: usize,
    pub text: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unconverted {
    pub lines: Vec<UnconvertedLine>,
}

impl Script {
    pub fn from_javascript(lines: &[String]) -> Result<Script, Unconverted> {
        let source = lines.join("\n");
        let tokens = lexer::tokens(&source).map_err(|failure| Unconverted::at(&source, failure))?;
        let program = Parser::new(tokens, &source)
            .program()
            .map_err(|failure| Unconverted::at(&source, failure))?;
        Recognizer::default()
            .statements(&program)
            .map(|statements| Script { statements })
            .map_err(|failures| Unconverted {
                lines: failures
                    .into_iter()
                    .map(|(line, reason)| UnconvertedLine::new(&source, line, reason))
                    .collect(),
            })
    }
}

impl Unconverted {
    fn at(source: &str, (line, reason): (usize, String)) -> Unconverted {
        Unconverted {
            lines: vec![UnconvertedLine::new(source, line, reason)],
        }
    }
}

impl UnconvertedLine {
    fn new(source: &str, line: usize, reason: String) -> UnconvertedLine {
        UnconvertedLine {
            line,
            text: source
                .lines()
                .nth(line.saturating_sub(1))
                .unwrap_or_default()
                .trim()
                .to_string(),
            reason,
        }
    }
}

#[cfg(test)]
mod tests;
