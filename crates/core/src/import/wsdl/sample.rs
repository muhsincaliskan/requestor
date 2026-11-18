use super::definitions::{Part, PartKind};
use super::particle::{ComplexType, Content, ElementType, Particle, TypeDef};
use super::qname::QName;
use super::schema::Schema;
use super::xml::escaped;

const INDENT: &str = "  ";
const PLACEHOLDER: &str = "?";
const PREFIX: &str = "ns";
const MAX_DEPTH: usize = 12;
const TAG_NAME_ENDS: [char; 3] = [' ', '>', '/'];

pub struct SampleWriter<'s> {
    schema: &'s Schema,
    namespaces: Vec<String>,
    named_types: Vec<QName>,
    out: String,
    depth: usize,
}

impl<'s> SampleWriter<'s> {
    fn new(schema: &'s Schema) -> Self {
        SampleWriter {
            schema,
            namespaces: Vec::new(),
            named_types: Vec::new(),
            out: String::new(),
            depth: 0,
        }
    }

    pub fn part(schema: &'s Schema, part: &Part) -> String {
        let mut writer = SampleWriter::new(schema);
        writer.message_part(part);
        writer.finish()
    }

    pub fn rpc(schema: &'s Schema, wrapper: &QName, parts: &[&Part]) -> String {
        let mut writer = SampleWriter::new(schema);
        writer.open(wrapper, &[]);
        writer.depth += 1;
        parts.iter().for_each(|part| writer.message_part(part));
        writer.depth -= 1;
        writer.close(wrapper);
        writer.finish()
    }

    fn message_part(&mut self, part: &Part) {
        match &part.kind {
            PartKind::Element(name) => self.reference(name),
            PartKind::Type(name) => self.element(
                &QName::unqualified(part.name.as_str()),
                &ElementType::Named(name.clone()),
            ),
        }
    }

    fn reference(&mut self, name: &QName) {
        let schema = self.schema;
        match schema.element(name) {
            Some(declaration) => self.element(&declaration.name, &declaration.kind),
            None => self.leaf(name, &[], PLACEHOLDER),
        }
    }

    fn element(&mut self, name: &QName, kind: &ElementType) {
        if self.depth > MAX_DEPTH {
            return;
        }
        match kind {
            ElementType::Any => self.leaf(name, &[], PLACEHOLDER),
            ElementType::Inline(definition) => self.typed(name, definition),
            ElementType::Named(type_name) => self.named(name, type_name),
        }
    }

    fn named(&mut self, name: &QName, type_name: &QName) {
        let schema = self.schema;
        match schema.type_def(type_name) {
            None => self.leaf(name, &[], PLACEHOLDER),
            Some(_) if self.named_types.contains(type_name) => self.empty(name, &[]),
            Some(definition) => {
                self.named_types.push(type_name.clone());
                self.typed(name, definition);
                self.named_types.pop();
            }
        }
    }

    fn typed(&mut self, name: &QName, definition: &TypeDef) {
        match definition {
            TypeDef::Simple(simple) => self.leaf(
                name,
                &[],
                simple.first_value.as_deref().unwrap_or(PLACEHOLDER),
            ),
            TypeDef::Complex(complex) => self.complex(name, complex),
        }
    }

    fn complex(&mut self, name: &QName, complex: &ComplexType) {
        if matches!(complex.content, Content::Text) {
            return self.leaf(name, &complex.attributes, PLACEHOLDER);
        }
        let particles = self.particles_of(complex);
        if particles.is_empty() {
            return self.empty(name, &complex.attributes);
        }
        self.open(name, &complex.attributes);
        self.depth += 1;
        particles
            .into_iter()
            .for_each(|particle| self.particle(particle));
        self.depth -= 1;
        self.close(name);
    }

    fn particles_of<'x>(&self, complex: &'x ComplexType) -> Vec<&'x Particle>
    where
        's: 'x,
    {
        let mut chain = vec![complex];
        let mut base = complex.base.as_ref();
        while let Some(name) = base.filter(|_| chain.len() <= MAX_DEPTH) {
            let Some(TypeDef::Complex(parent)) = self.schema.type_def(name) else {
                break;
            };
            chain.push(parent);
            base = parent.base.as_ref();
        }
        chain
            .into_iter()
            .rev()
            .flat_map(|complex| match &complex.content {
                Content::Particles(particles) => particles.as_slice(),
                Content::Text => &[],
            })
            .collect()
    }

    fn particle(&mut self, particle: &Particle) {
        match particle {
            Particle::Element(declaration) => self.element(&declaration.name, &declaration.kind),
            Particle::Reference(name) => self.reference(name),
            Particle::Group(name) => {
                let schema = self.schema;
                if self.depth > MAX_DEPTH {
                    return;
                }
                for member in schema.group(name).unwrap_or_default() {
                    self.particle(member);
                }
            }
        }
    }

    fn open(&mut self, name: &QName, attributes: &[String]) {
        let start = self.start_tag(name, attributes);
        self.line(&format!("{start}>"));
    }

    fn close(&mut self, name: &QName) {
        let tag = self.tag(name);
        self.line(&format!("</{tag}>"));
    }

    fn leaf(&mut self, name: &QName, attributes: &[String], value: &str) {
        let start = self.start_tag(name, attributes);
        let tag = self.tag(name);
        self.line(&format!("{start}>{}</{tag}>", escaped(value)));
    }

    fn empty(&mut self, name: &QName, attributes: &[String]) {
        let start = self.start_tag(name, attributes);
        self.line(&format!("{start}/>"));
    }

    fn start_tag(&mut self, name: &QName, attributes: &[String]) -> String {
        let mut start = format!("<{}", self.tag(name));
        for attribute in attributes {
            start.push_str(&format!(" {attribute}=\"{PLACEHOLDER}\""));
        }
        start
    }

    fn tag(&mut self, name: &QName) -> String {
        if !name.is_qualified() {
            return name.local.clone();
        }
        let index = match self
            .namespaces
            .iter()
            .position(|known| known == &name.namespace)
        {
            Some(index) => index,
            None => {
                self.namespaces.push(name.namespace.clone());
                self.namespaces.len() - 1
            }
        };
        format!("{PREFIX}{}:{}", index + 1, name.local)
    }

    fn line(&mut self, text: &str) {
        self.out.push_str(&INDENT.repeat(self.depth));
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn finish(self) -> String {
        let declarations: String = self
            .namespaces
            .iter()
            .enumerate()
            .map(|(index, namespace)| {
                format!(" xmlns:{PREFIX}{}=\"{}\"", index + 1, escaped(namespace))
            })
            .collect();
        let mut out = self.out.trim_end().to_string();
        if let Some(end) = out.find(TAG_NAME_ENDS) {
            out.insert_str(end, &declarations);
        }
        out
    }
}
