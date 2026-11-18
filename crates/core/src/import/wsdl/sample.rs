use super::definitions::{Part, PartKind};
use super::particle::{ComplexType, Content, ElementType, Particle, TypeDef};
use super::qname::QName;
use super::schema::Schema;
use super::xml_writer::XmlWriter;

const PLACEHOLDER: &str = "?";
const MAX_DEPTH: usize = 12;
const MAX_BASE_TYPES: usize = 16;
const MAX_SAMPLE_ELEMENTS: usize = 500;

#[derive(PartialEq)]
enum Expansion {
    Type(QName),
    Element(QName),
    Group(QName),
}

pub struct SampleWriter<'s> {
    schema: &'s Schema,
    writer: XmlWriter,
    expanding: Vec<Expansion>,
    elements: usize,
}

impl<'s> SampleWriter<'s> {
    fn new(schema: &'s Schema) -> Self {
        SampleWriter {
            schema,
            writer: XmlWriter::default(),
            expanding: Vec::new(),
            elements: 0,
        }
    }

    pub fn part(schema: &'s Schema, part: &Part) -> String {
        let mut sample = SampleWriter::new(schema);
        sample.message_part(part);
        sample.writer.finish()
    }

    pub fn rpc(schema: &'s Schema, wrapper: &QName, parts: &[&Part]) -> String {
        let mut sample = SampleWriter::new(schema);
        sample.writer.open(wrapper, &[]);
        parts.iter().for_each(|part| sample.message_part(part));
        sample.writer.close(wrapper);
        sample.writer.finish()
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
        let Some(declaration) = schema.element(name) else {
            return self.writer.leaf(name, &[], PLACEHOLDER);
        };
        let expansion = Expansion::Element(name.clone());
        if self.expanding.contains(&expansion) {
            return self.writer.empty(&declaration.name, &[]);
        }
        self.expanding.push(expansion);
        self.element(&declaration.name, &declaration.kind);
        self.expanding.pop();
    }

    fn element(&mut self, name: &QName, kind: &ElementType) {
        if self.writer.depth > MAX_DEPTH || self.elements >= MAX_SAMPLE_ELEMENTS {
            return;
        }
        self.elements += 1;
        match kind {
            ElementType::Any => self.writer.leaf(name, &[], PLACEHOLDER),
            ElementType::Inline(definition) => self.typed(name, definition),
            ElementType::Named(type_name) => self.named(name, type_name),
        }
    }

    fn named(&mut self, name: &QName, type_name: &QName) {
        let schema = self.schema;
        let expansion = Expansion::Type(type_name.clone());
        match schema.type_def(type_name) {
            None => self.writer.leaf(name, &[], PLACEHOLDER),
            Some(_) if self.expanding.contains(&expansion) => self.writer.empty(name, &[]),
            Some(definition) => {
                self.expanding.push(expansion);
                self.typed(name, definition);
                self.expanding.pop();
            }
        }
    }

    fn typed(&mut self, name: &QName, definition: &TypeDef) {
        match definition {
            TypeDef::Simple(simple) => self.writer.leaf(
                name,
                &[],
                simple.first_value.as_deref().unwrap_or(PLACEHOLDER),
            ),
            TypeDef::Complex(complex) => self.complex(name, complex),
        }
    }

    fn complex(&mut self, name: &QName, complex: &ComplexType) {
        let chain = self.base_chain(complex);
        let attributes: Vec<String> = chain
            .iter()
            .flat_map(|complex| complex.attributes.iter().cloned())
            .collect();
        if matches!(complex.content, Content::Text) {
            return self.writer.leaf(name, &attributes, PLACEHOLDER);
        }
        let particles: Vec<&Particle> = chain
            .iter()
            .flat_map(|complex| match &complex.content {
                Content::Particles(particles) => particles.as_slice(),
                Content::Text => &[],
            })
            .collect();
        if particles.is_empty() {
            return self.writer.empty(name, &attributes);
        }
        self.writer.open(name, &attributes);
        particles
            .into_iter()
            .for_each(|particle| self.particle(particle));
        self.writer.close(name);
    }

    fn base_chain<'x>(&self, complex: &'x ComplexType) -> Vec<&'x ComplexType>
    where
        's: 'x,
    {
        let mut chain = vec![complex];
        let mut base = complex.base.as_ref();
        while let Some(name) = base.filter(|_| chain.len() <= MAX_BASE_TYPES) {
            let Some(TypeDef::Complex(parent)) = self.schema.type_def(name) else {
                break;
            };
            chain.push(parent);
            base = parent.base.as_ref();
        }
        chain.reverse();
        chain
    }

    fn particle(&mut self, particle: &Particle) {
        match particle {
            Particle::Element(declaration) => self.element(&declaration.name, &declaration.kind),
            Particle::Reference(name) => self.reference(name),
            Particle::Group(name) => self.group(name),
        }
    }

    fn group(&mut self, name: &QName) {
        let schema = self.schema;
        let expansion = Expansion::Group(name.clone());
        if self.expanding.contains(&expansion) {
            return;
        }
        self.expanding.push(expansion);
        for member in schema.group(name).unwrap_or_default() {
            self.particle(member);
        }
        self.expanding.pop();
    }
}
