use crate::document::{DocPos, EventDocument};
use crate::renderers::notebook::heading_num;
use crate::renderers::Renderer;
use pulldown_cmark::{CodeBlockKind, Event, Tag};
use std::fmt::Write;

struct MarkdownRenderer;

impl Renderer for MarkdownRenderer {
    fn render(&self, doc: &EventDocument) -> String {
        render_markdown(doc.to_events_with_pos())
    }
}

struct MarkdownWriter<I> {
    iter: I,
    source: String,
    list_order_num: Option<u64>,
}

impl<'a, I> MarkdownWriter<I>
where
    I: Iterator<Item = (Event<'a>, DocPos)>,
{
    fn new(iter: I) -> Self {
        MarkdownWriter {
            iter,
            source: String::new(),
            list_order_num: None,
        }
    }

    fn start_tag(&mut self, tag: Tag<'a>) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading(level, _, _) => {
                let mut prefix = "#".repeat(heading_num(level));
                prefix.push(' ');
                self.source.push_str(&prefix);
            }
            Tag::BlockQuote => {}
            Tag::CodeBlock(kind) => match kind {
                CodeBlockKind::Indented => {
                    self.source.push_str("```plain\n");
                }
                CodeBlockKind::Fenced(cls) => {
                    let s = cls.into_string();
                    writeln!(self.source, "```{}", s).expect("Invalid format");
                }
            },
            Tag::List(i) => {
                self.list_order_num = i;
            }
            Tag::Item => match self.list_order_num {
                None => self.source.push_str("- "),
                Some(i) => {
                    write!(self.source, "{}. ", i).expect("Invalid format");
                    self.list_order_num = self.list_order_num.map(|i| i + 1);
                }
            },
            Tag::FootnoteDefinition(_) => {}
            Tag::Table(_) => {}
            Tag::TableHead => {}
            Tag::TableRow => {}
            Tag::TableCell => {}
            Tag::Emphasis => self.source.push('*'),
            Tag::Strong => self.source.push_str("__"),
            Tag::Strikethrough => {}
            Tag::Link(_, _, _) => self.source.push('['),
            Tag::Image(_, _, _) => {}
        }
    }

    fn end_tag(&mut self, tag: Tag<'a>) {
        match tag {
            Tag::CodeBlock(_) => self.source.push_str("\n```\n"),
            Tag::Paragraph => self.source.push('\n'),
            Tag::Heading(_, _, _) => self.source.push_str("\n\n"),
            Tag::BlockQuote => {}
            Tag::List(_) => self.source.push('\n'),
            Tag::Item => self.source.push('\n'),
            Tag::FootnoteDefinition(_) => {}
            Tag::Table(_) => {}
            Tag::TableHead => {}
            Tag::TableRow => {}
            Tag::TableCell => {}
            Tag::Emphasis => self.source.push('*'),
            Tag::Strong => self.source.push_str("__"),
            Tag::Strikethrough => {}
            Tag::Link(_type, dest, title) => {
                write!(self.source, "]({} {})", dest, title).expect("Invalid format");
            }
            Tag::Image(_, _, _) => {}
        }
    }

    fn run(mut self) -> String {
        while let Some((event, _range)) = self.iter.next() {
            match event {
                Event::Start(tag) => self.start_tag(tag),
                Event::End(tag) => self.end_tag(tag),
                Event::Text(text) => {
                    let ts = text.into_string();
                    if &ts == "\\" {
                        self.source.push_str("\\\\");
                    } else {
                        self.source.push_str(&ts)
                    }
                }
                Event::Code(_) => {}
                Event::Html(text) => self.source.push_str(&text.into_string()),
                Event::FootnoteReference(_) => {}
                Event::SoftBreak => self.source.push('\n'),
                Event::HardBreak => self.source.push_str("\n\n"),
                Event::Rule => {}
                Event::TaskListMarker(_) => {}
            };
        }

        self.source
    }
}

pub fn render_markdown<'a, I>(iter: I) -> String
where
    I: Iterator<Item = (Event<'a>, DocPos)>,
{
    MarkdownWriter::new(iter).run()
}