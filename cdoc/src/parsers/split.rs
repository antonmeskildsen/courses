use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;

use crate::parsers::split_types::{
    Block, CodeTaskDefinition, Content, Inner, SolutionBlock, Value,
};

#[derive(Parser)]
#[grammar = "parsers/split.pest"]
pub struct TaskParser;

pub fn parse_markup_block(pair: Pair<Rule>) -> Result<String, Box<pest::error::Error<Rule>>> {
    Ok(pair
        .into_inner()
        .map(|p| Ok(p.as_str().parse().expect("String parse error")))
        .collect::<Result<Vec<String>, pest::error::Error<Rule>>>()?
        .join("\n"))
}

pub fn parse_src_block(pair: Pair<Rule>) -> Result<Content, Box<pest::error::Error<Rule>>> {
    Ok(match pair.as_rule() {
        Rule::source_code_block => Content::Code {
            code: pair.as_str().parse().expect("String parse error"),
        },
        Rule::markup_block => Content::Markup {
            markup: parse_markup_block(pair)?,
        },
        _ => unreachable!(),
    })
}

fn parse_source_comment(pair: Pair<Rule>) -> Result<String, Box<pest::error::Error<Rule>>> {
    Ok(match pair.as_rule() {
        Rule::source_comment => {
            let mut inner = pair.into_inner();
            let spacing = inner.next().expect("Unexpected end of iterator");
            let str = inner.next().expect("Unexpected end of iterator");
            let spacing: String = spacing.as_str().parse().expect("String parse error");
            let str: String = str.as_str().parse().expect("String parse error");

            format!("{}{}", spacing, str)
        }
        _ => unreachable!(),
    })
}

pub fn parse_code_placeholder_block(
    pair: Pair<Rule>,
) -> Result<Content, Box<pest::error::Error<Rule>>> {
    Ok(match pair.as_rule() {
        Rule::source_comment_block => Content::Code {
            code: pair
                .into_inner()
                .map(parse_source_comment)
                .collect::<anyhow::Result<Vec<String>, Box<pest::error::Error<Rule>>>>()?
                .join("\n"),
        },
        Rule::markup_block => Content::Markup {
            markup: parse_markup_block(pair)?,
        },
        _ => unreachable!(),
    })
}

pub fn parse_inner_block(pair: Pair<Rule>) -> Result<Inner, Box<pest::error::Error<Rule>>> {
    let r = pair.as_rule();
    Ok(match r {
        Rule::code_block => {
            let mut block_segments = pair.into_inner();
            let placeholder_pair = block_segments.next().expect("Unexpected end of iterator");
            let solution_pair = block_segments.next().expect("Unexpected end of iterator");

            let placeholder: Vec<Content> = placeholder_pair
                .into_inner()
                .map(parse_code_placeholder_block)
                .collect::<anyhow::Result<Vec<Content>, Box<pest::error::Error<Rule>>>>()?;

            let solution: Vec<Content> =
                solution_pair
                    .into_inner()
                    .map(parse_src_block)
                    .collect::<anyhow::Result<Vec<Content>, Box<pest::error::Error<Rule>>>>()?;

            Inner::SolutionBlock(SolutionBlock {
                placeholder,
                solution,
            })
        }
        Rule::source_code_block | Rule::markup_block => Inner::SrcBlock(parse_src_block(pair)?),
        _ => unreachable!(),
    })
}

pub fn parse_attribute(
    pair: Pair<Rule>,
) -> Result<(String, String), Box<pest::error::Error<Rule>>> {
    Ok(match pair.as_rule() {
        Rule::attr => {
            let mut segments = pair.into_inner();
            let name = segments
                .next()
                .expect("Unexpected end of iterator")
                .as_str()
                .parse()
                .expect("String parse error");
            let value = segments
                .next()
                .expect("Unexpected end of iterator")
                .as_str()
                .parse()
                .expect("String parse error");

            (name, value)
        }
        _ => unreachable!(),
    })
}

pub fn parse_value(pair: Pair<Rule>) -> Result<Value, Box<pest::error::Error<Rule>>> {
    Ok(match pair.as_rule() {
        Rule::block => {
            let mut block_segments = pair.into_inner();
            let keyword = block_segments
                .next()
                .expect("Unexpected end of iterator")
                .as_str()
                .parse()
                .expect("String parse error");
            let attribute_pairs = block_segments.next().expect("Unexpected end of iterator");
            let content_pairs = block_segments.next().expect("Unexpected end of iterator");

            let attributes_vec = attribute_pairs
                .into_inner()
                .map(parse_attribute)
                .collect::<anyhow::Result<Vec<(String, String)>, Box<pest::error::Error<Rule>>>>(
                )?;
            let content = content_pairs
                .into_inner()
                .map(parse_inner_block)
                .collect::<anyhow::Result<Vec<Inner>, Box<pest::error::Error<Rule>>>>()?;

            Value::Block {
                block: Block {
                    keyword,
                    attributes: HashMap::from_iter(attributes_vec),
                    inner: content,
                },
            }
        }
        Rule::source_code_block | Rule::markup_block => Value::SrcBlock {
            content: parse_src_block(pair)?,
        },
        Rule::comment_def => Value::SrcBlock {
            content: Content::Markup {
                markup: pair.as_str().parse().expect("String parse error"),
            },
        },
        Rule::code_block => {
            let mut block_segments = pair.into_inner();
            let placeholder_pair = block_segments.next().expect("Unexpected end of iterator");
            let solution_pair = block_segments.next().expect("Unexpected end of iterator");

            let placeholder: Vec<Content> = placeholder_pair
                .into_inner()
                .map(parse_code_placeholder_block)
                .collect::<anyhow::Result<Vec<Content>, Box<pest::error::Error<Rule>>>>()?;

            let solution: Vec<Content> =
                solution_pair
                    .into_inner()
                    .map(parse_src_block)
                    .collect::<anyhow::Result<Vec<Content>, Box<pest::error::Error<Rule>>>>()?;

            Value::SolutionBlock(SolutionBlock {
                placeholder,
                solution,
            })
        }
        _ => unreachable!(),
    })
}

pub fn parse_code_string(
    content: &str,
) -> Result<CodeTaskDefinition, Box<pest::error::Error<Rule>>> {
    let mut padded = content.to_string();
    padded.push('\n');
    let p = TaskParser::parse(Rule::doc, &padded)?;

    let vals = p
        .into_iter()
        .map(parse_value)
        .collect::<anyhow::Result<Vec<Value>, Box<pest::error::Error<Rule>>>>()?;
    Ok(CodeTaskDefinition { blocks: vals })
}

pub fn human_errors(error: pest::error::Error<Rule>) -> Box<pest::error::Error<Rule>> {
    Box::new(error.renamed_rules(|rule| match *rule {
        Rule::source_code_block => "code".to_owned(),
        Rule::code_block => "placeholder/solution block".to_owned(),
        Rule::block => "arbitrary block".to_owned(),
        Rule::attr => "block attributes".to_owned(),
        Rule::source_comment => "code comment".to_owned(),
        Rule::source_comment_block => "code comment".to_owned(),
        Rule::markup_block => "markup lines".to_owned(),
        _ => "Unknown".to_owned(),
    }))
}

pub fn format_pest_err(error: pest::error::Error<Rule>) -> String {
    let error = human_errors(error);
    // format!(
    //     r#"
    // line: {:?}, col: {:?},
    // details: {}
    // "#,
    //     error.location, error.line_col, error.variant
    // )
    format!("{}", error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let str = include_str!("../../resources/test/sample.py");
        let _doc = parse_code_string(str).unwrap();
    }

    // #[test]
    // fn test_output() {
    //     let str = include_str!("../../../resources/test/sample.rs");
    //     let doc = parse_code_string(str).unwrap();
    //
    //     let _output_solution = doc.write_string(true);
    //     let _output_placeholder = doc.write_string(false);
    // }

    #[test]
    fn test_serialize() {
        let str = include_str!("../../resources/test/sample.rs");
        let doc = parse_code_string(str).unwrap();

        let _res = serde_json::to_string(&doc).unwrap();
    }
}
