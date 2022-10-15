use crate::split::task_parser::parse_code_string;
use crate::split::types::CodeTaskDefinition;
use anyhow::Context;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};

pub trait ExtensionFactory {
    fn build<'a>(&self) -> Box<dyn Extension<'a>>;
}

pub trait Extension<'a> {
    fn each(&mut self, event: Event<'a>) -> anyhow::Result<Event<'a>>;
}

pub struct CodeSplitFactory {}

impl ExtensionFactory for CodeSplitFactory {
    fn build<'a>(&self) -> Box<dyn Extension<'a>> {
        Box::new(CodeSplit::default())
    }
}

#[derive(Debug, Default)]
pub struct CodeSplit {
    code_started: bool,
    pub solution_string: String,
    pub source_def: CodeTaskDefinition,
}

impl CodeSplit {
    pub fn get_source_def(&self) -> &CodeTaskDefinition {
        &self.source_def
    }
}

impl<'a> Extension<'a> for CodeSplit {
    fn each(&mut self, event: Event<'a>) -> anyhow::Result<Event<'a>> {
        let res = match event {
            Event::Start(tag) => match &tag {
                Tag::CodeBlock(attribute_string) => {
                    self.code_started = true;
                    // if let CodeBlockKind::Fenced(attr_str) = attribute_string {
                    //     if attr_str.len() == 0 || attr_str.to_string() == "python".to_string() {
                    //         self.code_started = true;
                    //     }
                    // }
                    Event::Start(tag)
                }
                _ => Event::Start(tag),
            },
            Event::End(tag) => match &tag {
                Tag::CodeBlock(_content) => {
                    self.code_started = false;
                    Event::End(tag)
                }
                _ => Event::End(tag),
            },
            Event::Text(txt) => {
                if self.code_started {
                    let mut doc = parse_code_string(txt.as_ref()).context("Parsing code cell")?;
                    let (placeholder, solution) = doc.split();
                    self.solution_string.push_str(&solution);
                    self.source_def.blocks.append(&mut doc.blocks);

                    Event::Text(CowStr::Boxed(placeholder.into_boxed_str()))
                } else {
                    Event::Text(txt)
                }
            }
            _ => event,
        };
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::Builder;
    use crate::config::{Document, Format};

    #[test]
    fn test_code_split() {
        let mut builder =
            Builder::new("resources/test/", vec![Box::new(CodeSplitFactory {})]).unwrap();
        let doc = Document {
            format: Format::Markdown,
            path: "resources/test/code.md".into(),
        };

        let res = builder.parse_pd(doc).unwrap();
        println!("{:?}", res);
    }
}
