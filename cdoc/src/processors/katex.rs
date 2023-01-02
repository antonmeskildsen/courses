use katex::Opts;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;

use crate::processors::{MarkdownPreprocessor, PreprocessorConfig, PreprocessorContext};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KaTeXConfig;

#[typetag::serde(name = "katex")]
impl PreprocessorConfig for KaTeXConfig {
    fn build(&self, _ctx: &PreprocessorContext) -> anyhow::Result<Box<dyn MarkdownPreprocessor>> {
        Ok(Box::new(KaTeX))
    }
}

#[derive(Error, Debug)]
pub enum KaTeXPreprocessorError {}

#[derive(Debug)]
pub struct KaTeX;

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

impl MarkdownPreprocessor for KaTeX {
    fn name(&self) -> String {
        "KaTeX preprocessor".to_string()
    }

    fn process(&self, input: &str, _ctx: &tera::Context) -> Result<String, anyhow::Error> {
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

impl Display for KaTeX {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> String {
        let context = tera::Context::new();
        let processor = KaTeX;
        processor
            .process(input, &context)
            .expect("KaTeX parse error")
    }

    #[test]
    fn inline_mode() {
        let input = r#"some input $\frac{2}{3}$"#;
        let output = parse(input);
        let expected = r#"some input <span class="katex"><span class="katex-mathml"><math xmlns="http://www.w3.org/1998/Math/MathML"><semantics><mrow><mfrac><mn>2</mn><mn>3</mn></mfrac></mrow><annotation encoding="application/x-tex">\frac{2}{3}</annotation></semantics></math></span><span class="katex-html" aria-hidden="true"><span class="base"><span class="strut" style="height:1.1901em;vertical-align:-0.345em;"></span><span class="mord"><span class="mopen nulldelimiter"></span><span class="mfrac"><span class="vlist-t vlist-t2"><span class="vlist-r"><span class="vlist" style="height:0.8451em;"><span style="top:-2.655em;"><span class="pstrut" style="height:3em;"></span><span class="sizing reset-size6 size3 mtight"><span class="mord mtight"><span class="mord mtight">3</span></span></span></span><span style="top:-3.23em;"><span class="pstrut" style="height:3em;"></span><span class="frac-line" style="border-bottom-width:0.04em;"></span></span><span style="top:-3.394em;"><span class="pstrut" style="height:3em;"></span><span class="sizing reset-size6 size3 mtight"><span class="mord mtight"><span class="mord mtight">2</span></span></span></span></span><span class="vlist-s">​</span></span><span class="vlist-r"><span class="vlist" style="height:0.345em;"><span></span></span></span></span></span><span class="mclose nulldelimiter"></span></span></span></span></span>"#;

        assert_eq!(
            output, expected,
            "KaTeX should always produce the same output."
        )
    }

    #[test]
    fn display_mode() {
        let input = r#"some input $$\begin{array}{cc}a&b\\c&d\end{array}$$"#;
        let output = parse(input);
        let expected = r#"some input <span class="katex-display"><span class="katex"><span class="katex-mathml"><math xmlns="http://www.w3.org/1998/Math/MathML" display="block"><semantics><mtable rowspacing="0.16em" columnalign="center center" columnspacing="1em"><mtr><mtd><mstyle scriptlevel="0" displaystyle="false"><mi>a</mi></mstyle></mtd><mtd><mstyle scriptlevel="0" displaystyle="false"><mi>b</mi></mstyle></mtd></mtr><mtr><mtd><mstyle scriptlevel="0" displaystyle="false"><mi>c</mi></mstyle></mtd><mtd><mstyle scriptlevel="0" displaystyle="false"><mi>d</mi></mstyle></mtd></mtr></mtable><annotation encoding="application/x-tex">\begin{array}{cc}a&amp;b\\c&amp;d\end{array}</annotation></semantics></math></span><span class="katex-html" aria-hidden="true"><span class="base"><span class="strut" style="height:2.4em;vertical-align:-0.95em;"></span><span class="mord"><span class="mtable"><span class="arraycolsep" style="width:0.5em;"></span><span class="col-align-c"><span class="vlist-t vlist-t2"><span class="vlist-r"><span class="vlist" style="height:1.45em;"><span style="top:-3.61em;"><span class="pstrut" style="height:3em;"></span><span class="mord"><span class="mord mathnormal">a</span></span></span><span style="top:-2.41em;"><span class="pstrut" style="height:3em;"></span><span class="mord"><span class="mord mathnormal">c</span></span></span></span><span class="vlist-s">​</span></span><span class="vlist-r"><span class="vlist" style="height:0.95em;"><span></span></span></span></span></span><span class="arraycolsep" style="width:0.5em;"></span><span class="arraycolsep" style="width:0.5em;"></span><span class="col-align-c"><span class="vlist-t vlist-t2"><span class="vlist-r"><span class="vlist" style="height:1.45em;"><span style="top:-3.61em;"><span class="pstrut" style="height:3em;"></span><span class="mord"><span class="mord mathnormal">b</span></span></span><span style="top:-2.41em;"><span class="pstrut" style="height:3em;"></span><span class="mord"><span class="mord mathnormal">d</span></span></span></span><span class="vlist-s">​</span></span><span class="vlist-r"><span class="vlist" style="height:0.95em;"><span></span></span></span></span></span><span class="arraycolsep" style="width:0.5em;"></span></span></span></span></span></span></span>"#;

        assert_eq!(
            output, expected,
            "KaTeX should always produce the same output."
        )
    }
}
