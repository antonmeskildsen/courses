use anyhow::Context;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::document::{Document, EventContent, IteratorConfig, PreprocessError, RawContent};
use crate::processors::shortcodes::ShortCodeProcessError;
use crate::processors::{
    EventPreprocessor, EventPreprocessorConfig, MarkdownPreprocessor, PreprocessorConfig,
    PreprocessorContext,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Parser {
    pub preprocessors: Vec<Box<dyn PreprocessorConfig>>,
    pub event_processors: Vec<Box<dyn EventPreprocessorConfig>>,
    pub settings: ParserSettings,
}

/// Additional parser configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParserSettings {
    /// Include solutions for the [Exercises] preprocessor.
    #[serde(default)]
    pub solutions: bool,
    /// Include notebook outputs (from cells) in the loaded output.
    #[serde(default)]
    pub notebook_outputs: bool,
}

impl Parser {
    pub fn parse(
        &self,
        doc: &Document<RawContent>,
        template_context: &tera::Context,
        ctx: &PreprocessorContext,
    ) -> Result<Document<EventContent>, anyhow::Error> {
        let doc = self.run_preprocessors(doc, template_context, ctx)?;
        self.run_event_processors(&doc, ctx)
    }

    pub fn run_preprocessors(
        &self,
        doc: &Document<RawContent>,
        template_context: &tera::Context,
        ctx: &PreprocessorContext,
    ) -> Result<Document<RawContent>, anyhow::Error> {
        let built = self
            .preprocessors
            .iter()
            .map(|p| p.build(ctx))
            .collect::<anyhow::Result<Vec<Box<dyn MarkdownPreprocessor>>>>()?;

        let content = built.iter().fold(Ok(doc.clone()), |c, preprocessor| {
            c.and_then(|c| {
                c.preprocess(preprocessor.as_ref(), template_context)
                    .with_context(|| format!("Preprocessing error in {}", preprocessor))
            })
        })?;

        Ok(content)
    }

    pub fn run_event_processors(
        &self,
        doc: &Document<RawContent>,
        ctx: &PreprocessorContext,
    ) -> Result<Document<EventContent>, anyhow::Error> {
        let v = doc.to_events(IteratorConfig {
            include_output: doc
                .metadata
                .notebook_output
                .unwrap_or(self.settings.notebook_outputs),
            include_solutions: doc
                .metadata
                .code_solutions
                .unwrap_or(self.settings.solutions),
        });

        let built = self
            .event_processors
            .iter()
            .map(|p| p.build(ctx))
            .collect::<anyhow::Result<Vec<Box<dyn EventPreprocessor>>>>()?;

        let events = built.iter().fold(Ok(v), |c, event_processor| {
            c.and_then(|c| event_processor.process(c))
        })?;

        Ok(events)
    }
}

#[allow(unused)]
struct HeadingNode {
    id: String,
    children: Vec<HeadingNode>,
}

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("IO Error: ")]
    IoError(#[from] std::io::Error),

    #[error("Error in template")]
    TemplateError(#[from] tera::Error),

    #[error("JSON Error: ")]
    JSONError(#[from] serde_json::error::Error),

    #[error("Error parsing frontmatter: ")]
    FrontMatter(#[from] serde_yaml::Error),

    #[error(transparent)]
    Preprocess(#[from] PreprocessError),

    #[error(transparent)]
    ExtensionError(#[from] crate::processors::Error),

    #[error(transparent)]
    ShortCode(#[from] ShortCodeProcessError),

    #[error(transparent)]
    KaTeX(#[from] katex::Error),

    #[error(transparent)]
    Std(#[from] Box<dyn std::error::Error>),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_deserialization() {
    //     let config = r#"
    //         {
    //             "preprocessors": [
    //                 {
    //                     "type": "shortcodes",
    //                     "template": "tp/**",
    //                     "file_ext": ".html"
    //                 }
    //             ],
    //             "event_processors": []
    //         }
    //     "#;
    //
    //     let p: Parser = serde_json::from_str(config).unwrap();
    // }
}
