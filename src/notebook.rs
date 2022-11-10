use crate::parser::FrontMatter;
use crate::parsers::split::parse_code_string;
use crate::parsers::split_types::Output;
use anyhow::Context;
use base64;
use pulldown_cmark::CodeBlockKind::Fenced;
use pulldown_cmark::Tag::CodeBlock;
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use serde_with::EnumMap;
use std::collections::HashMap;
use std::iter::FlatMap;
use std::slice::Iter;
use std::vec::IntoIter;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Notebook {
    pub(crate) metadata: NotebookMeta,
    pub(crate) nbformat: i64,
    pub(crate) nbformat_minor: i64,
    pub(crate) cells: Vec<Cell>,
}

type Dict = HashMap<String, Value>;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NotebookMeta {
    pub(crate) kernelspec: Option<HashMap<String, Value>>,
    #[serde(flatten)]
    pub(crate) optional: Dict,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CellMeta {
    collapsed: Option<bool>,
    autoscroll: Option<Value>,
    deletable: Option<bool>,
    format: Option<String>,
    name: Option<String>,
    tags: Option<Vec<String>>,
    #[serde(flatten)]
    additional: Dict,
}

fn concatenate_deserialize<'de, D>(input: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let base: Vec<String> = Deserialize::deserialize(input)?;
    let source = base.into_iter().collect();
    Ok(escape_string_deserialize(source))
}

fn concatenate_serialize<S>(value: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_seq(value.split("\n"))
}

fn escape_string_deserialize(source: String) -> String {
    let escaped = source
        .chars()
        .flat_map(|c| match c {
            '\\' => r#"\\"#.chars().collect(),
            // '\'' => vec!['\\', '\''],
            // '\"' => vec!['\\', '\"'],
            // '±' => vec!['±'],
            _ => vec![c],
        })
        .collect::<String>();
    escaped
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CellCommon {
    pub metadata: CellMeta,
    #[serde(
        deserialize_with = "concatenate_deserialize",
        serialize_with = "concatenate_serialize"
    )]
    pub source: String,
}

fn deserialize_png<'de, D>(input: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let base: String = Deserialize::deserialize(input)?;
    let bytes = base64::decode(base).map_err(|e| D::Error::custom(e.to_string()))?;
    // let source = load_from_memory(&bytes).map_err(|e| D::Error::custom(e.to_string()))?;
    Ok(bytes)
}

fn serialize_png<S>(value: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // serializer.collect_str(&base64::encode(value.as_bytes()))
    serializer.collect_str(&base64::encode(value))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OutputValue {
    #[serde(rename = "text/plain")]
    Plain(
        #[serde(
            deserialize_with = "concatenate_deserialize",
            serialize_with = "concatenate_serialize"
        )]
        String,
    ),
    #[serde(rename = "image/png")]
    Image(String),
    #[serde(rename = "image/svg+xml")]
    Svg(String),
    #[serde(rename = "application/json")]
    Json(HashMap<String, Value>),
    #[serde(rename = "text/html")]
    Html(
        #[serde(
            deserialize_with = "concatenate_deserialize",
            serialize_with = "concatenate_serialize"
        )]
        String,
    ),
    #[serde(rename = "application/javascript")]
    Javascript(String),
}

// #[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
// pub enum OutputType {
//     #[serde(rename = "text/plain")]
//     Plain,
//     #[serde(rename = "image/png")]
//     Image,
//     #[serde(rename = "application/json")]
//     Json,
// }

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "output_type")]
pub enum CellOutput {
    #[serde(rename = "stream")]
    Stream {
        name: String,
        #[serde(
            deserialize_with = "concatenate_deserialize",
            serialize_with = "concatenate_serialize"
        )]
        text: String,
    },
    #[serde(rename = "display_data", alias = "execute_result")]
    Data {
        execution_count: Option<i64>,
        #[serde_as(as = "EnumMap")]
        data: Vec<OutputValue>,
        metadata: HashMap<String, Value>,
    },
    // #[serde(rename = "execute_result")]
    // Result {
    //     execution_count: i64,
    //     data: HashMap<String, Data>,
    //     metadata: HashMap<String, Value>,
    // },
    #[serde(rename = "error")]
    Error {
        ename: String,
        evalue: String,
        traceback: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cell_type")]
pub enum Cell {
    #[serde(rename = "markdown")]
    Markdown {
        #[serde(flatten)]
        common: CellCommon,
    },
    #[serde(rename = "code")]
    Code {
        #[serde(flatten)]
        common: CellCommon,

        execution_count: Option<i64>,

        outputs: Vec<CellOutput>,
    },
    #[serde(rename = "raw")]
    Raw {
        #[serde(flatten)]
        common: CellCommon,
    },
}

pub enum CellEventIterator<'a, 'b> {
    Markdown {
        cell: &'a Cell,
        parser: Parser<'a, 'b>,
    },
    Code {
        cell: &'a Cell,
        events: IntoIter<Event<'a>>,
    },
    Raw {
        cell: &'a Cell,
    },
}

impl CellOutput {
    pub fn to_events(&self) -> Vec<Event> {
        match self {
            CellOutput::Stream { name, text } => {
                vec![Event::Html(CowStr::Boxed(
                    format!(
                        r#"
                <div class="alert alert-info">
                    <p>{}</p>
                </div>
                "#,
                        text
                    )
                    .into_boxed_str(),
                ))]
            }
            CellOutput::Data {
                data,
                metadata,
                execution_count,
            } => data
                .into_iter()
                .flat_map(|value| match value {
                    OutputValue::Plain(v) => {
                        let block = Tag::CodeBlock(Fenced(CowStr::Boxed(
                            "plaintext".to_string().into_boxed_str(),
                        )));
                        vec![
                            Event::Start(block.clone()),
                            Event::Text(CowStr::Borrowed(v)),
                            Event::End(block),
                        ]
                    }
                    OutputValue::Image(v) => {
                        vec![Event::Html(CowStr::Boxed(
                            format!("<img src=\"data:image/png;base64,{}\"></img>", v)
                                .into_boxed_str(),
                        ))]
                    }
                    OutputValue::Svg(v) => {
                        vec![Event::Html(CowStr::Boxed(
                            format!(
                                "<img><svg width=\"640px\" height=\"480px\">{}</svg></img>",
                                v
                            )
                            .into_boxed_str(),
                        ))]
                    }
                    OutputValue::Json(v) => {
                        vec![Event::Text(CowStr::Boxed(
                            format!("{:?}", v).into_boxed_str(),
                        ))]
                    }
                    OutputValue::Html(v) => {
                        vec![Event::Html(CowStr::Boxed(v.to_string().into_boxed_str()))]
                    }
                    OutputValue::Javascript(v) => {
                        vec![Event::Html(CowStr::Boxed(
                            format!("<script>{}</script>", v).into_boxed_str(),
                        ))]
                    }
                })
                .collect(),
            CellOutput::Error { .. } => {
                vec![Event::Text(CowStr::Boxed(
                    "Error".to_string().into_boxed_str(),
                ))]
            }
        }
    }
}

impl<'a> IntoIterator for &'a Cell {
    type Item = Event<'a>;
    type IntoIter = CellEventIterator<'a, 'a>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Cell::Markdown { common } => CellEventIterator::Markdown {
                cell: &self,
                parser: Parser::new_ext(&common.source, Options::all()),
            },
            Cell::Code {
                common, outputs, ..
            } => {
                let cblock = CodeBlock(Fenced(CowStr::Boxed("python".into())));
                let source = &common.source;
                let mut events = vec![
                    Event::Start(cblock.clone()),
                    Event::Text(CowStr::Borrowed(&common.source)),
                    Event::End(cblock),
                ];
                outputs
                    .into_iter()
                    .for_each(|o| events.append(&mut o.to_events()));
                CellEventIterator::Code {
                    cell: &self,
                    events: events.into_iter(),
                }
            }
            Cell::Raw { .. } => CellEventIterator::Raw { cell: &self },
        }
    }
}

impl<'a, 'b> Iterator for CellEventIterator<'a, 'b> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            CellEventIterator::Markdown { parser, .. } => parser.next(),
            CellEventIterator::Code { events, .. } => events.next(),
            CellEventIterator::Raw { .. } => None,
        }
    }
}

pub struct NotebookIterator<'a, 'b> {
    iter: FlatMap<
        Iter<'a, Cell>,
        CellEventIterator<'a, 'b>,
        fn(&'a Cell) -> CellEventIterator<'a, 'b>,
    >,
}

impl<'a> IntoIterator for &'a Notebook {
    type Item = Event<'a>;
    type IntoIter = NotebookIterator<'a, 'a>;

    fn into_iter(self) -> Self::IntoIter {
        NotebookIterator {
            iter: self.cells.iter().flat_map(|c| c.into_iter()),
        }
    }
}

impl<'a, 'b> Iterator for NotebookIterator<'a, 'b> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl Notebook {
    pub fn get_front_matter(&self) -> anyhow::Result<Option<FrontMatter>> {
        match &self.cells[0] {
            Cell::Raw { common } => Ok(Some(serde_yaml::from_str(&common.source)?)),
            Cell::Code { common, .. } => {
                let r = common
                    .metadata
                    .additional
                    .get("vscode")
                    .and_then(|d| match d {
                        Value::Object(o) => o.get("languageId"),
                        _ => None,
                    })
                    .filter(|val| match val {
                        Value::String(s) => s == "yaml",
                        _ => false,
                    })
                    .and_then(|_| Some(serde_yaml::from_str(&common.source)));
                r.map_or(Ok(None), |v| v.map(Some).context("Yaml"))
            }
            _ => Ok(None),
        }
    }

    pub fn map_cell(&self, f: fn(&Cell) -> anyhow::Result<Cell>) -> anyhow::Result<Notebook> {
        let cells = self.cells.iter().map(f);
        Ok(Notebook {
            metadata: self.metadata.clone(),
            nbformat: self.nbformat,
            nbformat_minor: self.nbformat_minor,
            cells: cells.collect::<anyhow::Result<Vec<Cell>>>()?,
        })
    }

    pub fn placeholder_notebook(&self) -> anyhow::Result<Notebook> {
        self.map_cell(|c| match c {
            Cell::Code {
                common,
                execution_count,
                outputs,
            } => {
                let def = parse_code_string(&common.source)?;
                let placeholder = def.write_string(false);
                Ok(Cell::Code {
                    common: CellCommon {
                        source: placeholder,
                        metadata: common.metadata.clone(),
                    },
                    execution_count: *execution_count,
                    outputs: Vec::new(),
                })
            }
            c => Ok(c.clone()),
        })
    }
}

//
// impl<'a> IntoIterator for &'a Cell {
//     type Item = Event<'a>;
//     type IntoIter = Parser<'a, 'a>;
//
//     fn into_iter(self) -> Self::IntoIter {
//         match self {
//             Cell::Markdown { common } => {
//                 Parser::new(&common.source)
//             }
//             Cell::Code { common, .. } => {
//
//             }
//             _ => Parser::new("")
//         }
//     }
// }

#[cfg(test)]
mod tests {

    use crate::notebook::Notebook;
    use pulldown_cmark::html;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::PathBuf;

    #[test]
    fn deserialize() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test/test.ipynb");
        let bf = BufReader::new(File::open(d).expect("Could not open file"));
        let nb: Notebook = serde_json::from_reader(bf).expect("Deserialization failed");

        println!("Done");
    }

    #[test]
    fn html_out() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test/test.ipynb");

        let bf = BufReader::new(File::open(d).expect("Could not open file"));
        let nb: Notebook = serde_json::from_reader(bf).expect("Deserialization failed");

        let mut html_output = String::new();
        html::push_html(&mut html_output, nb.into_iter());

        // println!("{}", html_output);
    }
}