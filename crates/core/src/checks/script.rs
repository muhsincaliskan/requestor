use serde_json::Value;

use super::literal::{quote, split_quoted};
use super::{Check, ExtractSource, Extraction, JsonPath, ResponseRules, VariableTarget};

const TEST_OPEN: &str = "pm.test(";
const TEST_ARROW: &str = ", () => ";
const CALL_CLOSE: &str = ");";
const ARGUMENT_SEPARATOR: &str = ", ";
const STATUS_TITLE: &str = "Status is ";
const TIME_TITLE: &str = "Response time is below ";
const TIME_UNIT: &str = " ms";
const BODY_TITLE: &str = "Body contains ";
const HEADER_TITLE: &str = "Header ";
const CONTAINS: &str = " contains ";
const JSON_TITLE: &str = "JSON ";
const EQUALS: &str = " equals ";
const JSON_SOURCE: &str = "pm.response.json()";
const HEADER_SOURCE: &str = "pm.response.headers.get(";
const SOURCE_CLOSE: &str = ")";
const TITLE_PARSERS: [fn(&str) -> Option<Check>; 5] = [
    status_from_title,
    time_from_title,
    body_from_title,
    header_from_title,
    json_from_title,
];

impl ResponseRules {
    pub fn from_script(lines: &[&str]) -> Option<Self> {
        let mut rules = ResponseRules::default();
        for line in lines
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
        {
            if let Some(check) = Check::from_script_line(line) {
                rules.checks.push(check);
            } else {
                rules.extractions.push(Extraction::from_script_line(line)?);
            }
        }
        Some(rules)
    }

    pub fn script_lines(&self) -> Vec<String> {
        let checks = self.checks.iter().map(Check::script_line);
        let extractions = self.extractions.iter().map(Extraction::script_line);
        checks.chain(extractions).collect()
    }
}

impl Check {
    pub fn title(&self) -> String {
        match self {
            Check::Status(code) => format!("{STATUS_TITLE}{code}"),
            Check::ResponseTimeBelow(limit) => format!("{TIME_TITLE}{limit}{TIME_UNIT}"),
            Check::BodyContains(text) => format!("{BODY_TITLE}{}", quote(text)),
            Check::HeaderContains { name, text } => {
                format!("{HEADER_TITLE}{name}{CONTAINS}{}", quote(text))
            }
            Check::JsonEquals { path, expected } => {
                format!("{JSON_TITLE}{}{EQUALS}{expected}", path.text())
            }
        }
    }

    fn assertion(&self) -> String {
        match self {
            Check::Status(code) => format!("pm.response.to.have.status({code})"),
            Check::ResponseTimeBelow(limit) => {
                format!("pm.expect(pm.response.responseTime).to.be.below({limit})")
            }
            Check::BodyContains(text) => {
                format!("pm.expect(pm.response.text()).to.include({})", quote(text))
            }
            Check::HeaderContains { name, text } => format!(
                "pm.expect({HEADER_SOURCE}{}{SOURCE_CLOSE}).to.include({})",
                quote(name),
                quote(text)
            ),
            Check::JsonEquals { path, expected } => format!(
                "pm.expect({JSON_SOURCE}{}).to.eql({expected})",
                path.accessor()
            ),
        }
    }

    fn script_line(&self) -> String {
        format!(
            "{TEST_OPEN}{}{TEST_ARROW}{}{CALL_CLOSE}",
            quote(&self.title()),
            self.assertion()
        )
    }

    fn from_script_line(line: &str) -> Option<Check> {
        let (title, _) = split_quoted(line.strip_prefix(TEST_OPEN)?)?;
        let check = TITLE_PARSERS.iter().find_map(|parse| parse(&title))?;
        (check.script_line() == line).then_some(check)
    }
}

impl VariableTarget {
    fn setter(self) -> &'static str {
        match self {
            VariableTarget::Environment => "pm.environment.set(",
            VariableTarget::Collection => "pm.collectionVariables.set(",
        }
    }
}

impl ExtractSource {
    fn script(&self) -> String {
        match self {
            ExtractSource::Json(path) => format!("{JSON_SOURCE}{}", path.accessor()),
            ExtractSource::Header(name) => format!("{HEADER_SOURCE}{}{SOURCE_CLOSE}", quote(name)),
        }
    }

    fn from_script(text: &str) -> Option<ExtractSource> {
        if let Some(accessor) = text.strip_prefix(JSON_SOURCE) {
            return JsonPath::from_accessor(accessor).map(ExtractSource::Json);
        }
        let (name, rest) = split_quoted(text.strip_prefix(HEADER_SOURCE)?)?;
        (rest == SOURCE_CLOSE).then_some(ExtractSource::Header(name))
    }
}

impl Extraction {
    fn script_line(&self) -> String {
        format!(
            "{}{}{ARGUMENT_SEPARATOR}{}{CALL_CLOSE}",
            self.target.setter(),
            quote(&self.variable),
            self.source.script()
        )
    }

    fn from_script_line(line: &str) -> Option<Extraction> {
        let target = [VariableTarget::Environment, VariableTarget::Collection]
            .into_iter()
            .find(|target| line.starts_with(target.setter()))?;
        let (variable, rest) = split_quoted(&line[target.setter().len()..])?;
        let source = rest
            .strip_prefix(ARGUMENT_SEPARATOR)?
            .strip_suffix(CALL_CLOSE)?;
        let extraction = Extraction {
            variable,
            source: ExtractSource::from_script(source)?,
            target,
        };
        (extraction.script_line() == line).then_some(extraction)
    }
}

fn status_from_title(title: &str) -> Option<Check> {
    title
        .strip_prefix(STATUS_TITLE)?
        .parse()
        .ok()
        .map(Check::Status)
}

fn time_from_title(title: &str) -> Option<Check> {
    let limit = title.strip_prefix(TIME_TITLE)?.strip_suffix(TIME_UNIT)?;
    limit.parse().ok().map(Check::ResponseTimeBelow)
}

fn body_from_title(title: &str) -> Option<Check> {
    let (text, rest) = split_quoted(title.strip_prefix(BODY_TITLE)?)?;
    rest.is_empty().then_some(Check::BodyContains(text))
}

fn header_from_title(title: &str) -> Option<Check> {
    let (name, quoted) = title.strip_prefix(HEADER_TITLE)?.split_once(CONTAINS)?;
    let (text, rest) = split_quoted(quoted)?;
    rest.is_empty().then(|| Check::HeaderContains {
        name: name.to_string(),
        text,
    })
}

fn json_from_title(title: &str) -> Option<Check> {
    let (path, expected) = title.strip_prefix(JSON_TITLE)?.split_once(EQUALS)?;
    Some(Check::JsonEquals {
        path: JsonPath::parse(path)?,
        expected: serde_json::from_str::<Value>(expected).ok()?,
    })
}

#[cfg(test)]
mod tests;
