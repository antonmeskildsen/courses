mod html;
mod markdown;
mod notebook;

use crate::document::EventDocument;
use std::collections::HashMap;
use std::ops::Deref;

pub trait Renderer {
    fn render(&self, doc: &EventDocument) -> String;
}

pub struct RenderExtensionConfiguration {
    mapping: HashMap<String, Box<dyn Renderer>>,
}

impl RenderExtensionConfiguration {
    pub fn add_mapping(&mut self, extension: &str, parser: Box<dyn Renderer>) {
        self.mapping.insert(extension.to_string(), parser);
    }

    pub fn get_parser(&self, extension: &str) -> Option<&dyn Renderer> {
        self.mapping.get(extension).map(|b| b.deref())
    }
}
