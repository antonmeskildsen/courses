use crate::notebook::{Cell, CellEventIterator, CellOutput, Notebook};
use pulldown_cmark::CodeBlockKind::Fenced;
use pulldown_cmark::Tag::CodeBlock;
use pulldown_cmark::{CowStr, Event, Options, Parser};
use std::iter::FlatMap;
use std::slice::Iter;
use std::vec::IntoIter;

#[derive(Debug, Clone, Default)]
pub enum Element {
    Markdown {
        content: String,
    },
    Code {
        content: String,
        output: Option<Vec<CellOutput>>,
    },
    Raw {
        content: String,
    },
    #[default]
    Default,
}

#[derive(Debug, Clone, Default)]
pub struct Document {
    elements: Vec<Element>,
}

impl From<String> for Document {
    fn from(s: String) -> Self {
        Document {
            elements: vec![Element::Markdown { content: s }],
        }
    }
}

impl From<Notebook> for Document {
    fn from(n: Notebook) -> Self {
        let elements = n
            .cells
            .into_iter()
            .map(|cell| match cell {
                Cell::Markdown { common } => Element::Markdown {
                    content: common.source,
                },
                Cell::Code {
                    common, outputs, ..
                } => Element::Code {
                    content: common.source,
                    output: Some(outputs),
                },
                Cell::Raw { common } => Element::Raw {
                    content: common.source,
                },
            })
            .collect();

        Document { elements }
    }
}

pub enum ElementIterator<'a, 'b> {
    Markdown { parser: Parser<'a, 'b> },
    Code { events: IntoIter<Event<'a>> },
    Raw {},
}

impl<'a, 'b> Iterator for ElementIterator<'a, 'b> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ElementIterator::Markdown { parser, .. } => parser.next(),
            ElementIterator::Code { events, .. } => events.next(),
            ElementIterator::Raw { .. } => None,
        }
    }
}

#[derive(Default, Copy, Clone)]
pub struct IteratorConfig {
    include_output: bool,
    include_solutions: bool,
}

pub trait ConfigureIterator {
    type Item;
    type IntoIter;

    fn configure_iterator(self, config: IteratorConfig) -> Self::IntoIter;
}

impl<'a> ConfigureIterator for &'a Element {
    type Item = Event<'a>;
    type IntoIter = ElementIterator<'a, 'a>;

    fn configure_iterator(self, config: IteratorConfig) -> Self::IntoIter {
        match self {
            Element::Markdown { content } => ElementIterator::Markdown {
                parser: Parser::new_ext(&content, Options::all()),
            },

            Element::Code {
                content,
                output: outputs,
            } => {
                let cblock = CodeBlock(Fenced(CowStr::Boxed("python".into())));
                let mut events = vec![
                    Event::Start(cblock.clone()),
                    Event::Text(CowStr::Borrowed(content)),
                    Event::End(cblock),
                ];
                if config.include_output {
                    if let Some(os) = outputs {
                        for o in os {
                            events.append(&mut o.to_events());
                        }
                    }
                }

                ElementIterator::Code {
                    events: events.into_iter(),
                }
            }
            Element::Raw { .. } => ElementIterator::Raw {},
            _ => ElementIterator::Raw {},
        }
    }
}

impl<'a> ConfigureIterator for &'a Document {
    type Item = Event<'a>;
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn configure_iterator(self, config: IteratorConfig) -> Self::IntoIter {
        Box::new(
            self.elements
                .iter()
                .flat_map(move |elem: &Element| elem.configure_iterator(config)),
        )
    }
}
