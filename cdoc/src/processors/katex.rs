use katex::Opts;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::processors::Preprocessor;
use crate::Context;

#[derive(Error, Debug)]
pub enum KaTeXPreprocessorError {}

#[derive(Serialize, Deserialize, Debug)]
pub struct KaTeXPreprocessor;

fn find_block(input: &str) -> Option<(usize, usize, usize)> {
    let begin = input.find('$')?;
    let end_delim = if &input[(begin + 1)..(begin + 2)] == "$" {
        "$$"
    } else {
        "$"
    };

    let end = begin + end_delim.len() + input[begin + end_delim.len()..].find(end_delim)?;

    Some((begin, end, end_delim.len()))
}

#[typetag::serde(name = "katex")]
impl Preprocessor for KaTeXPreprocessor {
    fn name(&self) -> String {
        "KaTeX preprocessor".to_string()
    }

    fn process(&self, input: &str, ctx: &Context) -> Result<String, Box<dyn std::error::Error>> {
        let mut rest = input;
        let mut res = String::new();

        while !rest.is_empty() {
            match find_block(rest) {
                Some((begin, end, delim_len)) => {
                    let pre = &rest[..begin];
                    let post = &rest[(end + delim_len)..];

                    let source = &rest[(begin + delim_len)..end];

                    let opts = Opts::builder().display_mode(delim_len == 2).build()?;
                    let ktex = katex::render_with_opts(source, opts)?;

                    res.push_str(pre);
                    res.push_str(&ktex);

                    rest = post;
                }
                None => {
                    res.push_str(rest);
                    rest = ""
                }
            }
        }

        Ok(res)
    }
}
