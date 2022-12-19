use std::hash::Hash;
use std::sync::Arc;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::loader::{Loader, MarkdownLoader, NotebookLoader};
use crate::parser::{Parser, ParserSettings};
use crate::processors::code_split::CodeSplitConfig;
use crate::processors::katex::KaTeXPreprocessorConfig;
use crate::processors::shortcode_extender::ShortCodeProcessConfig;
use crate::renderers::html::HtmlRenderer;
use crate::renderers::markdown::MarkdownRenderer;
use crate::renderers::notebook::NotebookRenderer;
use crate::renderers::Renderer;

#[derive(Hash, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InputFormat {
    Markdown,
    Notebook,
}

#[derive(Hash, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Markdown,
    Notebook,
    Html,
    Config,
}

impl InputFormat {
    pub fn loader(&self) -> Box<dyn Loader> {
        match self {
            InputFormat::Markdown => Box::new(MarkdownLoader),
            InputFormat::Notebook => Box::new(NotebookLoader),
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            InputFormat::Markdown => "md",
            InputFormat::Notebook => "ipynb",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            InputFormat::Markdown => "markdown",
            InputFormat::Notebook => "notebook",
        }
    }

    pub fn from_extension(ext: &str) -> Result<Self, anyhow::Error> {
        match ext {
            "md" => Ok(InputFormat::Markdown),
            "ipynb" => Ok(InputFormat::Notebook),
            _ => Err(anyhow!("Invalid extension for input")),
        }
    }

    pub fn from_name(name: &str) -> Result<Self, anyhow::Error> {
        match name {
            "markdown" => Ok(InputFormat::Markdown),
            "notebook" => Ok(InputFormat::Notebook),
            _ => Err(anyhow!("Invalid format name for input")),
        }
    }
}

impl OutputFormat {
    pub fn from_extension(ext: &str) -> Result<Self, anyhow::Error> {
        match ext {
            "md" => Ok(OutputFormat::Markdown),
            "ipynb" => Ok(OutputFormat::Notebook),
            "html" => Ok(OutputFormat::Html),
            _ => Err(anyhow!("Invalid extension for output")),
        }
    }

    pub fn from_name(name: &str) -> Result<Self, anyhow::Error> {
        match name {
            "markdown" => Ok(OutputFormat::Markdown),
            "notebook" => Ok(OutputFormat::Notebook),
            "html" => Ok(OutputFormat::Html),
            "config" => Ok(OutputFormat::Config),
            _ => Err(anyhow!("Invalid format name for output")),
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            OutputFormat::Markdown => "md",
            OutputFormat::Notebook => "ipynb",
            OutputFormat::Html => "html",
            OutputFormat::Config => "yml",
        }
    }

    pub fn template_extension(&self) -> &str {
        match self {
            OutputFormat::Markdown => "md",
            OutputFormat::Notebook => "md",
            OutputFormat::Html => "html",
            OutputFormat::Config => "yml",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            OutputFormat::Markdown => "markdown",
            OutputFormat::Notebook => "notebook",
            OutputFormat::Html => "html",
            OutputFormat::Config => "config",
        }
    }

    pub fn renderer(&self) -> Option<Box<dyn Renderer>> {
        match self {
            OutputFormat::Markdown => Some(Box::new(MarkdownRenderer)),
            OutputFormat::Notebook => Some(Box::new(NotebookRenderer)),
            OutputFormat::Html => Some(Box::new(HtmlRenderer)),
            OutputFormat::Config => None,
        }
    }
}

#[allow(unused)]
fn get_default_parser(_format: OutputFormat) -> Parser {
    Parser {
        preprocessors: vec![
            Arc::new(ShortCodeProcessConfig),
            Arc::new(KaTeXPreprocessorConfig),
        ],
        event_processors: vec![Arc::new(CodeSplitConfig)],
        settings: ParserSettings {
            solutions: false,
            notebook_outputs: false,
        },
    }
}
