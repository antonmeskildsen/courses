use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use cdoc::config::OutputFormat;
use cdoc::parser::Parser;

/// Refers to a configuration.yml file in the project that specifies a variety
/// of options for the project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub url_prefix: String,
    #[serde(default = "default_config")]
    pub build: HashMap<String, BuildConfig>,
    #[serde(default)]
    pub repository: RepositoryConfig,
    pub outputs: Vec<OutputFormat>,
    pub parsers: HashMap<OutputFormat, Parser>,
    pub custom: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepositoryConfig {
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfigSet {
    pub dev: BuildConfig,
    pub release: BuildConfig,
}

fn default_config() -> HashMap<String, BuildConfig> {
    let mut map = HashMap::new();
    map.insert(
        "dev".to_string(),
        BuildConfig {
            katex_output: false,
        },
    );
    map.insert("release".to_string(), BuildConfig { katex_output: true });
    map
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub katex_output: bool,
}
