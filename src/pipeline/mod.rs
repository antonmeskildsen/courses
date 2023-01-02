use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use tera::Tera;

use cdoc::config::OutputFormat;
use cdoc::document::Document;
use cdoc::processors::PreprocessorContext;
use cdoc::renderers::RenderResult;
use mover::{MoveContext, Mover};

use crate::generators::html::HtmlGenerator;
use crate::generators::info::InfoGenerator;
use crate::generators::notebook::CodeOutputGenerator;
use crate::generators::{Generator, GeneratorContext};
use crate::project::config::ProjectConfig;
use crate::project::{section_id, ItemDescriptor, Part, Project, ProjectItem};

mod mover;

pub struct Pipeline {
    #[allow(unused)]
    mode: String,
    project_path: PathBuf,
    project: Project<()>,
    project_config: ProjectConfig,
    base_tera: Tera,
    shortcode_tera: Tera,
    cached_contexts: HashMap<OutputFormat, GeneratorContext>,
}

pub fn print_err<T>(res: anyhow::Result<T>) -> Option<T> {
    match res {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("{} {}", style("Error:").red().bold(), e);
            e.chain()
                .skip(1)
                .for_each(|cause| eprintln!(" {} {}", style("caused by:").bold(), cause));
            None
        }
    }
}

impl Pipeline {
    pub fn new<P: AsRef<Path>>(
        project_path: P,
        mode: String,
        config: ProjectConfig,
        project: Project<()>,
    ) -> anyhow::Result<Self> {
        let path_str = project_path
            .as_ref()
            .to_str()
            .ok_or_else(|| anyhow!("Invalid path"))?;
        let pattern = path_str.to_string() + "/templates/**/*.tera.html";
        let base_tera = Tera::new(&pattern).context("Error preparing project templates")?;

        let shortcode_pattern = path_str.to_string() + "/templates/shortcodes/**/*";
        let shortcode_tera =
            Tera::new(&shortcode_pattern).context("Error preparing project templates")?;

        Ok(Pipeline {
            mode,
            project_path: project_path.as_ref().to_path_buf(),
            project,
            project_config: config,
            base_tera,
            shortcode_tera,
            cached_contexts: HashMap::new(),
        })
    }

    fn get_generator(&self, format: OutputFormat) -> Box<dyn Generator> {
        match format {
            OutputFormat::Notebook => Box::new(CodeOutputGenerator),
            OutputFormat::Html => Box::new(HtmlGenerator::new(self.base_tera.clone())),
            OutputFormat::Info => Box::new(InfoGenerator),
        }
    }

    fn get_build_path(&self, format: OutputFormat) -> PathBuf {
        match format {
            OutputFormat::Notebook => self.project_path.join("build").join("notebooks"),
            OutputFormat::Html => self.project_path.join("build").join("html"),
            OutputFormat::Info => self.project_path.join("build"),
        }
    }

    pub fn reload_shortcode_tera(&mut self) -> anyhow::Result<()> {
        Ok(self.shortcode_tera.full_reload()?)
    }

    pub fn reload_base_tera(&mut self) -> anyhow::Result<()> {
        Ok(self.base_tera.full_reload()?)
    }

    pub fn build_single(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let relpath = path.strip_prefix(self.project_path.join("content"))?;
        println!("{} {}", style("Building file").bold(), relpath.display());
        println!("{}", style("-".repeat(60)).blue());
        let item = self.doc_from_path(path)?;
        let item2 = item.clone();

        let loaded = item.map_doc(|doc| {
            let path = self.project_path.join("content").join(doc.path);
            let val = fs::read_to_string(path.as_path())
                .context(format!("Error loading document at {}", path.display()))?;
            Ok::<String, anyhow::Error>(val)
        })?;

        let mut all_errors = Vec::new();

        for format in self.project_config.outputs.clone() {
            print!("format: {}", style(format).bold());
            let output = self.process_document(&loaded.doc, format);

            match output {
                Err(e) => {
                    all_errors.push(e);
                    println!(" {}", style("error").red());
                }
                Ok(output) => {
                    if let Some(output) = output {
                        let context = self
                            .cached_contexts
                            .get(&format)
                            .ok_or_else(|| anyhow!("Cached context is missing"))?
                            .clone();

                        let context = self.update_cache(&item2, &format, &output, context.clone());

                        self.get_generator(format).generate_single(
                            output,
                            item2.clone(),
                            context,
                        )?;

                        println!(" {}", style("done").green());
                    } else {
                        println!(" {}", style("no output").yellow());
                    }
                }
            }
            // let output = print_err(output).flatten();
        }

        println!("{}", style("-".repeat(60)).blue());
        if all_errors.is_empty() {
            println!("{}", style("Success").green().bold());
        } else {
            let len = all_errors.len();
            all_errors.into_iter().for_each(|e| {
                eprintln!("{} {}", style("Error:").red().bold(), e);
                e.chain()
                    .skip(1)
                    .for_each(|cause| eprintln!(" {} {}", style("caused by:").bold(), cause));
            });
            println!("{}", style("-".repeat(60)).blue());

            println!(
                "{}",
                style(format!(
                    "File built with non-critical errors ({} total)",
                    len
                ))
                .yellow()
                .bold()
            );
        }

        Ok(())
    }

    fn update_cache(
        &mut self,
        item2: &ItemDescriptor<()>,
        format: &OutputFormat,
        output: &Document<RenderResult>,
        mut context: GeneratorContext,
    ) -> GeneratorContext {
        let i3 = item2.clone();
        if let Some(part_id) = i3.part_idx {
            let part = &mut context.project.content[part_id];
            if let Some(chapter_id) = i3.chapter_idx {
                let chapter = &mut part.chapters[chapter_id];
                if let Some(doc_id) = i3.doc_idx {
                    chapter.documents[doc_id].content = Arc::new(Some(output.clone()));
                } else {
                    chapter.index.content = Arc::new(Some(output.clone()));
                }
            } else {
                part.index.content = Arc::new(Some(output.clone()));
            }
        } else {
            context.project.index.content = Arc::new(Some(output.clone()));
        }

        self.cached_contexts.insert(*format, context.clone());
        context
    }

    fn doc_from_path(&self, path: PathBuf) -> anyhow::Result<ItemDescriptor<()>> {
        let mut part_id = None;
        let mut chapter_id = None;
        let mut part_idx = None;
        let mut chapter_idx = None;
        let mut doc_idx = None;

        let doc_path = path
            .as_path()
            .strip_prefix(self.project_path.as_path().join("content"))?; // TODO: Error handling;
        let mut doc_iter = doc_path.iter();
        let el = doc_iter.next().unwrap().to_str().unwrap();

        let doc = if el.contains('.') {
            &self.project.index
        } else {
            let first_elem = doc_iter.next().unwrap().to_str().unwrap();

            // let file_name = doc_iter.next().unwrap().to_str().unwrap();
            let file_id = section_id(path.as_path()).unwrap();

            let part: &Part<()> = self
                .project
                .content
                .iter()
                .find(|e| e.id == el)
                .expect("Part not found for single document");
            let elem = part.chapters.iter().find(|c| c.id == first_elem);
            part_id = Some(part.id.clone());

            let pid = self
                .project
                .content
                .iter()
                .position(|e| e.id == el)
                .expect("Part index not found");
            part_idx = Some(pid);

            match elem {
                None => &part.index,
                Some(c) => {
                    chapter_id = Some(c.id.clone());
                    let cid = part
                        .chapters
                        .iter()
                        .position(|c| c.id == first_elem)
                        .expect("Part index not found");
                    chapter_idx = Some(cid);
                    let doc = c.documents.iter().find(|d| d.id == file_id);
                    match doc {
                        None => &c.index,
                        Some(d) => {
                            let did = c
                                .documents
                                .iter()
                                .position(|d| d.id == file_id)
                                .expect("Part index not found");
                            doc_idx = Some(did);

                            d
                        }
                    }
                }
            }
        };

        let item = ItemDescriptor {
            part_id,
            chapter_id,
            part_idx,
            chapter_idx,
            doc: doc.clone(),
            doc_idx,
            files: None,
        };

        Ok(item)
    }

    pub fn build_all(&mut self, remove_existing: bool) -> Result<(), anyhow::Error> {
        let build_path = self.project_path.join("build");

        if remove_existing && build_path.exists() {
            fs::remove_dir_all(build_path)?;
        }

        let loaded = self.load_all()?;

        println!("{}", style("=".repeat(60)).blue());
        println!(
            "{} ({} files)",
            style("Building project").bold(),
            loaded.len()
        );
        println!("{}", style("-".repeat(60)).blue());

        let mut all_errs = Vec::new();

        for format in &self.project_config.outputs {
            print!(
                "{}{}",
                style(format).bold(),
                " ".repeat(10 - format.to_string().len())
            );
            let mut format_errs = Vec::new();
            let (output, mut errs) = self.process_all(loaded.clone(), *format);
            format_errs.append(&mut errs);
            let context = GeneratorContext {
                root: self.project_path.to_path_buf(),
                project: output,
                config: self.project_config.clone(),
                build_dir: self.get_build_path(*format),
            };
            self.cached_contexts.insert(*format, context.clone());

            // print!("[generating output");

            let res = self
                .get_generator(*format)
                .generate(context.clone())
                .with_context(|| format!("Could not generate {}", format));

            match res {
                Err(e) => format_errs.push(e),
                Ok(_) => {
                    // println!("   output generation \t{}", style("success").green());
                    self.cached_contexts.insert(*format, context);
                }
            }

            // Move extra files
            if let Some(parser) = self.project_config.parsers.get(format) {
                // print!(", copying additional files");
                let move_ctx = MoveContext {
                    project_path: self.project_path.to_path_buf(),
                    build_dir: self.get_build_path(*format),
                    settings: parser.settings.clone(),
                };

                Mover::traverse_dir(self.project_path.join("content").to_path_buf(), &move_ctx)?;
            }

            // Error display
            if format_errs.is_empty() {
                println!("{}", style("success").green());
            } else {
                println!("{}", style(format!("({} errors)", format_errs.len())).red());
            }

            all_errs.append(&mut format_errs);
        }

        println!("{}", style("-".repeat(60)).blue());
        if all_errs.is_empty() {
            println!("{}", style("Project built successfully").green().bold());
        } else {
            let len = all_errs.len();
            all_errs.into_iter().for_each(|e| {
                eprintln!("{} {}", style("Error:").red().bold(), e);
                e.chain()
                    .skip(1)
                    .for_each(|cause| eprintln!(" {} {}", style("caused by:").bold(), cause));
            });
            println!("{}", style("-".repeat(60)).blue());

            println!(
                "{}",
                style(format!(
                    "Project built with non-critical errors ({} total)",
                    len
                ))
                .yellow()
                .bold()
            );
        }
        println!("{}", style("=".repeat(60)).blue());

        Ok(())
    }

    fn load_all(&self) -> Result<Project<String>, anyhow::Error> {
        self.project
            .clone()
            .into_iter()
            .map(|item| {
                item.map_doc(|doc| {
                    let path = self.project_path.join("content").join(doc.path);
                    let val = fs::read_to_string(path.as_path())
                        .context(format!("Error loading document {}", path.display()))?;

                    Ok(val)
                })
            })
            .collect::<Result<Project<String>, anyhow::Error>>()
    }

    // fn load_single(&self, )

    fn process_all(
        &self,
        project: Project<String>,
        format: OutputFormat,
    ) -> (Project<Option<Document<RenderResult>>>, Vec<anyhow::Error>) {
        let spinner = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
        let pb = ProgressBar::new(0);
        pb.set_style(spinner);
        // pb.set_prefix(format!("[{}/?]", i + 1));

        let mut errs = Vec::new();

        let res = project
            .into_iter()
            .map(|i| {
                pb.set_message(format!("{}", i.doc.path.display()));
                pb.inc(1);

                let res = self.process_document(&i.doc, format).with_context(|| {
                    format!(
                        "Failed to process document – {}",
                        style(format!("content/{}", i.doc.path.display())).italic()
                    )
                });

                let res = match res {
                    Ok(good) => good,
                    Err(e) => {
                        errs.push(e);
                        None
                    }
                };
                // let res = print_err(res);

                ItemDescriptor {
                    part_id: i.part_id,
                    chapter_id: i.chapter_id,
                    part_idx: i.part_idx,
                    chapter_idx: i.chapter_idx,
                    doc_idx: i.doc_idx,
                    doc: ProjectItem {
                        id: i.doc.id,
                        format: i.doc.format,
                        path: i.doc.path,
                        content: Arc::new(res),
                    },
                    files: i.files,
                }
            })
            .collect::<Project<Option<Document<RenderResult>>>>();

        pb.finish_and_clear();

        // pb.finish_with_message(format!("Done"));

        (res, errs)
    }

    fn process_document(
        &self,
        item: &ProjectItem<String>,
        format: OutputFormat,
    ) -> anyhow::Result<Option<Document<RenderResult>>> {
        let doc = item.format.loader().load(&item.content)?;
        // println!("doc {}", item.path.display());
        if format.no_parse() {
            Ok(Some(Document {
                content: "".to_string(),
                metadata: doc.metadata,
                variables: doc.variables,
            }))
        } else if doc.metadata.outputs.contains(&format) {
            let processor_ctx = PreprocessorContext {
                tera: self.shortcode_tera.clone(),
                output_format: format,
            };

            let mut meta = tera::Context::new();
            meta.insert("project", &self.project_config);

            let res = self
                .project_config
                .parsers
                .get(&format)
                .ok_or_else(|| anyhow!("Invalid format"))?
                .parse(&doc, &meta, &processor_ctx)?;

            // let res = print_err(res)?;
            if let Some(renderer) = format.renderer() {
                Ok(Some(renderer.render(&res)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}
