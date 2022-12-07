use crate::extensions::Preprocessor;
use crate::parsers::shortcodes::{parse_shortcode, Rule};
use pulldown_cmark::html::push_html;
use pulldown_cmark::{Options, Parser};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use tera::Tera;
use thiserror::Error;
use crate::cfg::ProjectConfig;

pub enum OutputFormat {
    Markdown,
    Html,
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Markdown => write!(f, "md"),
            OutputFormat::Html => write!(f, "html"),
        }
    }
}

enum ShortcodeInfo {
    Inline(usize, usize),
    Block {
        def: (usize, usize),
        end: (usize, usize),
    },
}

fn extract_block(start: usize, input: &str) -> Option<ShortcodeInfo> {
    let end = start + input[start..].find("%}")?;

    let end_block = end + (&input[end..]).find("{% end %}")?;

    Some(ShortcodeInfo::Block {
        def: (start, end),
        end: (end_block, end_block + 7),
    })
}

fn extract_inline(start: usize, input: &str) -> Option<ShortcodeInfo> {
    let end = start + 2 + input[(start + 2)..].find("}}")?;
    Some(ShortcodeInfo::Inline(start, end))
}

fn find_all_blocks(input: &str) -> Vec<(usize, usize)> {
    let mut rest = input;
    let mut offset = 0;

    let mut res = Vec::new();
    loop {
        let next = find_next_block(rest);
        match next {
            None => return res,
            Some((start, end)) => {
                res.push((offset + start, offset + end));
                rest = &rest[(end)..];
                offset += end;
            }
        }
    }
}

fn find_next_block(input: &str) -> Option<(usize, usize)> {
    let start = input.find("`")?;
    let end_delim = if input[(start + 1)..].len() > 2 && &input[(start + 1)..(start + 3)] == "``" {
        "```"
    } else {
        "`"
    };

    let end = start + 1 + input[(start + 1)..].find(end_delim)? + end_delim.len();
    Some((start, end))
}

fn find_shortcode(input: &str) -> Option<ShortcodeInfo> {
    let start_inline = input.find("{{");
    let start_block = input.find("{%");

    match start_inline {
        None => start_block.and_then(|start| extract_block(start, input)),
        Some(inline_start_idx) => match start_block {
            None => extract_inline(inline_start_idx, input),
            Some(block_start_idx) => {
                if inline_start_idx < block_start_idx {
                    extract_inline(inline_start_idx, input)
                } else {
                    extract_block(block_start_idx, input)
                }
            }
        },
    }
}

#[derive(Error, Debug)]
pub enum ShortCodeProcessError {
    // #[error("shortcode template error: {:#}", .source)]
    Tera {
        #[from]
        source: tera::Error,
    },
    // #[error("shortcode syntax error: {}", .0)]
    Pest(#[from] pest::error::Error<Rule>),
}

impl Display for ShortCodeProcessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ShortCodeProcessError::Tera { source } => {
                Display::fmt(&source, f)?;
                let mut e = source.source();
                while let Some(next) = e {
                    Display::fmt(&next, f)?;

                    e = next.source();
                }
                Ok(())
            }
            ShortCodeProcessError::Pest(inner) => Display::fmt(&inner, f),
        }
    }
}

pub struct ShortCodeProcessor {
    tera: Tera,
    project_config: ProjectConfig,
    file_ext: String,
}

impl ShortCodeProcessor {
    pub fn new(tera: Tera, file_ext: String, project_config: ProjectConfig) -> Self {
        ShortCodeProcessor { tera, file_ext, project_config }
    }

    fn render_inline_template(&self, shortcode: &str) -> Result<String, ShortCodeProcessError> {
        let code = parse_shortcode(shortcode)?;
        let mut context = tera::Context::new();
        let name = format!("{}/{}.tera.{}", self.file_ext, code.name, self.file_ext);

        context.insert("project", &self.project_config);
        for (k, v) in code.parameters {
            context.insert(k, &v);
        }
        Ok(self.tera.render(&name, &context)?)
    }

    fn render_block_template(
        &self,
        shortcode: &str,
        body: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let code = parse_shortcode(shortcode)?;
        let mut context = tera::Context::new();
        let name = format!("{}/{}.tera.{}", self.file_ext, code.name, self.file_ext);
        for (k, v) in code.parameters {
            context.insert(k, &v);
        }

        let processed =
            ShortCodeProcessor::new(self.tera.clone(), self.file_ext.clone(), self.project_config.clone()).process(&body)?;
        let body_final = if self.file_ext == "html" {
            let parser = Parser::new_ext(&processed, Options::all());
            let mut html = String::new();
            push_html(&mut html, parser);
            html
        } else {
            processed
        };

        context.insert("body", &body_final);
        Ok(self.tera.render(&name, &context)?)
    }
}

impl Preprocessor for ShortCodeProcessor {
    fn process(&self, input: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut rest = input;
        let mut offset = 0;

        let mut result = String::new();

        let blocks = find_all_blocks(input);

        while rest.len() > 0 {
            match find_shortcode(rest) {
                None => {
                    result.push_str(rest);
                    rest = "";
                }

                Some(info) => {
                    match info {
                        ShortcodeInfo::Inline(start, end) => {
                            match (&blocks)
                                .into_iter()
                                .filter(|(bs, be)| bs < &(start + offset) && be >= &(end + offset))
                                .next()
                            {
                                None => {
                                    let pre = &rest[..start];
                                    let post = &rest[(end + 2)..];
                                    let tmp_name = (&rest[(start + 2)..(end - 1)]).trim();

                                    let res = self.render_inline_template(tmp_name)?;

                                    result.push_str(pre);
                                    result.push_str(&res);

                                    rest = post; // Start next round after the current shortcode position
                                    offset += end + 2;
                                }
                                Some((_, block_end)) => {
                                    let relative = *block_end - offset;
                                    let pre = &rest[..relative];
                                    result.push_str(pre);
                                    rest = &rest[relative..];
                                    offset += relative;
                                }
                            }
                        }
                        ShortcodeInfo::Block { def, end } => {
                            match (&blocks)
                                .into_iter()
                                .filter(|(bs, be)| bs < &(def.1 + offset) && be > &(end.0 + offset))
                                .next()
                            {
                                None => {
                                    let pre = &rest[..def.0];
                                    let post = &rest[(end.1 + 2)..];

                                    let tmp_name = (&rest[(def.0 + 2)..(def.1 - 1)]).trim();
                                    let body = (&rest[(def.1 + 2)..end.0]).trim();

                                    let res = self.render_block_template(tmp_name, body)?;

                                    result.push_str(pre);
                                    result.push_str(&res);
                                    result.push('\n');

                                    rest = post; // Start next round after the current shortcode position
                                    offset += end.1 + 2;
                                }

                                Some((_, block_end)) => {
                                    let relative = *block_end - offset;
                                    let pre = &rest[..relative];
                                    result.push_str(pre);
                                    rest = &rest[relative..];
                                    offset += relative;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}
