use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use anyhow::{anyhow, Context};
use thiserror::Error;

use crate::document::{Document, DocumentMetadata, RawContent};
use crate::notebook::Notebook;

#[derive(Error, Debug)]
pub enum LoaderError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    Std(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// A loader creates an initial [Document] from a raw input string loaded from a content file.
#[typetag::serde(tag = "type")]
pub trait Loader: Debug {
    /// Perform any parsing/conversion necessary.
    fn load(&self, input: &str) -> anyhow::Result<Document<RawContent>>;
}

/// Parses a Jupyter Notebook file (.ipynb).
#[derive(Serialize, Deserialize, Debug)]
pub struct NotebookLoader;

#[typetag::serde(name = "notebook_loader")]
impl Loader for NotebookLoader {
    fn load(&self, input: &str) -> anyhow::Result<Document<RawContent>> {
        let nb: Notebook = serde_json::from_str(input)?;
        let meta = nb
            .get_front_matter()
            .context("Failed to read front matter")?;
        Ok(Document::new(nb, meta))
    }
}

/// Loads a markdown document. It reads the yml frontmatter and creates the document from the remaining input.
#[derive(Serialize, Deserialize, Debug)]
pub struct MarkdownLoader;

#[typetag::serde(name = "markdown_loader")]
impl Loader for MarkdownLoader {
    fn load(&self, input: &str) -> anyhow::Result<Document<RawContent>> {
        // let yml: yaml_front_matter::Document<DocumentMetadata> =
        //     // YamlFrontMatter::parse(input).map_err(|_e| anyhow!("Could not parse front matter"))?;
        //     YamlFrontMatter::parse(input)?;
        let start = input
            .find("---")
            .ok_or_else(|| anyhow!("Missing frontmatter specifier"))?;
        let end = start
            + 3
            + input[start + 3..]
                .find("---")
                .ok_or_else(|| anyhow!("Missing frontmatter specifier"))?;

        let meta: DocumentMetadata =
            serde_yaml::from_str(&input[start + 3..end]).context("Could not parse frontmatter")?;
        Ok(Document::new(input[end + 3..].to_string(), meta))
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    //
    // #[test]
    // fn test_deserialization() {
    //     let config = r#"
    //         {
    //             ".md": {"type": "markdown_loader"},
    //             ".ipynb": {"type": "notebook_loader"}
    //         }
    //     "#;
    //
    //     let p: LoaderConfig = serde_json::from_str(config).unwrap();
    // }
}
