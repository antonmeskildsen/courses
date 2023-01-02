use std::fs::DirEntry;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, io};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use cdoc::config::InputFormat;
pub use iterator::*;
pub use transform::*;

pub mod config;

mod iterator;
mod transform;

/// The top-level configuration of a project's content.TTT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project<C> {
    pub project_path: PathBuf,
    pub(crate) index: ProjectItem<C>,
    pub(crate) content: Vec<Part<C>>,
}

/// A part is the highest level of content division. Each project has a series of parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part<C> {
    /// Part id (folder name)
    pub id: String,
    /// Index document
    pub index: ProjectItem<C>,
    /// Chapters (in order)
    pub chapters: Vec<Chapter<C>>,
}

/// Parts contain chapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter<C> {
    /// Chapter id (folder name)
    pub id: String,
    /// Index document
    pub index: ProjectItem<C>,
    /// Individual documents
    pub documents: Vec<ProjectItem<C>>,
    /// Other files
    pub files: Vec<PathBuf>,
}

/// Chapters contain documents. Their configuration container is called DocumentSpec. It is a generic
/// type over the inner "document". This is useful for using the configuration as a datastructure
/// for content throughout the build process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem<C> {
    /// Document id (filename excluding extension)
    pub id: String,
    /// Document source format
    pub format: InputFormat,
    /// Location
    pub path: PathBuf,
    /// Content. It is wrapped in Arc to minimise unnecessary memory copying.
    pub content: Arc<C>,
}

/// Iterates a Config.
pub struct ProjectIterator<D> {
    part_pos: usize,
    chapter_pos: usize,
    doc_pos: usize,
    config: Project<D>,
}

/// Contains necessary information for reconstructing a Config from an iterator.
#[derive(Clone)]
pub struct ItemDescriptor<D> {
    pub part_id: Option<String>,
    pub chapter_id: Option<String>,
    pub part_idx: Option<usize>,
    pub chapter_idx: Option<usize>,
    pub doc_idx: Option<usize>,
    pub doc: ProjectItem<D>,
    pub files: Option<Vec<PathBuf>>, // Temporary solution for carrying file info
}

impl<C> Project<C> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        1 + self.content.iter().map(|e| e.len()).sum::<usize>()
    }
}

impl<C> Part<C> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        1 + self.chapters.iter().map(|c| c.len()).sum::<usize>()
    }
}

impl<C> Chapter<C> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        1 + self.documents.iter().map(|_| 1).sum::<usize>()
    }
}

// impl<C> ProjectItem<C> {
//     pub fn len(&self) -> usize {
//         1
//     }
// }

impl Project<()> {
    /// Construct configuration from a directory (generally the project directory). The function
    /// finds and verifies the structure of the project.
    pub fn generate_from_directory<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content_path = path.as_ref().join("content");

        let parts = get_sorted_paths(&content_path)?
            .into_iter()
            .filter_map(|entry| {
                let m = fs::metadata(entry.path());
                m.map(|m| m.is_dir().then_some(entry)).ok()?
            })
            .map(|entry| {
                let file_path = entry.path();
                Part::new(file_path, content_path.as_path())
            })
            .collect::<anyhow::Result<Vec<Part<()>>>>()?;

        let index_doc = index_helper(&content_path, &content_path)?;

        Ok(Project {
            project_path: path.as_ref().to_path_buf(),
            index: index_doc,
            content: parts,
        })
    }
}

impl Part<()> {
    fn new<P: AsRef<Path>, PC: AsRef<Path>>(dir: P, content_path: PC) -> anyhow::Result<Self> {
        let part_folder = chapter_id(&dir).ok_or_else(|| anyhow!("Can't get part id"))?;
        // let part_dir = dir.as_ref().join(&part_folder);

        let chapters = get_sorted_paths(&dir)?
            .into_iter()
            .filter(|entry| entry.metadata().map(|meta| meta.is_dir()).unwrap())
            .map(|entry| Chapter::new(entry.path(), content_path.as_ref()))
            .collect::<anyhow::Result<Vec<Chapter<()>>>>()?;

        Ok(Part {
            id: part_folder,
            index: index_helper(&dir, &content_path)?,
            chapters,
        })
    }
}

impl Chapter<()> {
    fn new<P: AsRef<Path>, PC: AsRef<Path>>(
        chapter_dir: P,
        content_path: PC,
    ) -> anyhow::Result<Self> {
        let section_dir = chapter_dir.as_ref();

        let paths = get_sorted_paths(section_dir)?
            .into_iter()
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .filter(|e| extension_in(e.to_str().unwrap()))
                    .is_some()
            })
            .filter(|entry| !entry.file_name().to_str().unwrap().contains("index"))
            .filter(|entry| entry.metadata().map(|meta| meta.is_file()).is_ok());

        let file_paths = get_sorted_paths(section_dir)?
            .into_iter()
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .filter(|e| extension_in(e.to_str().unwrap()))
                    .is_none()
            })
            .filter(|entry| !entry.file_name().to_str().unwrap().contains("index"))
            .filter(|entry| entry.metadata().map(|meta| meta.is_file()).is_ok())
            .map(|entry| entry.path())
            .collect();

        let documents: Vec<ProjectItem<()>> = paths
            .map(|entry| ProjectItem::new(entry.path().strip_prefix(content_path.as_ref())?))
            .collect::<anyhow::Result<Vec<ProjectItem<()>>>>()?;

        let index_doc = index_helper(&chapter_dir, &content_path);

        Ok(Chapter {
            id: chapter_id(chapter_dir).ok_or_else(|| anyhow!("Can't get chapter id"))?,
            index: index_doc?,
            documents,
            files: file_paths,
        })
    }
}

impl<I> ProjectItem<I> {
    fn transform_parents_helper<F, O>(
        &self,
        part: Option<&Part<I>>,
        chapter: Option<&Chapter<I>>,
        f: &F,
    ) -> ProjectItem<O>
    where
        F: Fn(&ProjectItem<I>, Option<&Part<I>>, Option<&Chapter<I>>) -> O,
    {
        ProjectItem {
            id: self.id.clone(),
            format: self.format,
            path: self.path.clone(),
            content: Arc::new(f(self, part, chapter)),
        }
    }
}

impl<C> ProjectItem<C> {
    pub fn map<O, F>(self, f: F) -> ProjectItem<O>
    where
        F: Fn(&C) -> O,
    {
        ProjectItem {
            id: self.id,
            format: self.format,
            path: self.path,
            content: Arc::new(f(self.content.deref())),
        }
    }
}

/// Extract a section_id (folder name) from a full path.
pub fn section_id<P: AsRef<Path>>(path: P) -> Option<String> {
    Some(
        path.as_ref()
            .file_name()?
            .to_str()?
            .split('.')
            .into_iter()
            .next()?
            .to_string(),
    )
}

/// Extract a chapter_id (folder name) from a full path.
fn chapter_id<P: AsRef<Path>>(path: P) -> Option<String> {
    Some(path.as_ref().file_name()?.to_str().unwrap().to_string())
}

impl ProjectItem<()> {
    fn new<P: AsRef<Path>>(section_path: P) -> anyhow::Result<Self> {
        Ok(ProjectItem {
            id: section_id(section_path.as_ref())
                .ok_or_else(|| anyhow!("Could not get raw file name"))?,
            path: section_path.as_ref().to_path_buf(),
            format: InputFormat::from_extension(
                section_path.as_ref().extension().unwrap().to_str().unwrap(),
            )?,
            content: Arc::new(()),
        })
    }
}

const EXT: [&str; 2] = ["md", "ipynb"];

fn extension_in(extension: &str) -> bool {
    EXT.iter().any(|e| e == &extension)
}

fn index_helper<P: AsRef<Path>, PC: AsRef<Path>>(
    chapter_dir: &P,
    content_path: &PC,
) -> anyhow::Result<ProjectItem<()>> {
    let chapter_index_md = chapter_dir.as_ref().join("index.md");
    let chapter_index_ipynb = chapter_dir.as_ref().join("index.ipynb");
    let chapter_index = if chapter_index_md.is_file() {
        chapter_index_md
    } else {
        chapter_index_ipynb
    };

    ProjectItem::new(chapter_index.strip_prefix(content_path.as_ref())?)
}

fn get_sorted_paths<P: AsRef<Path>>(path: P) -> io::Result<Vec<DirEntry>> {
    let mut paths: Vec<_> = fs::read_dir(&path)?.filter_map(|r| r.ok()).collect();
    paths.sort_by_key(|p| p.path());
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use std::iter::zip;

    use super::*;

    #[test]
    fn gen_config_from_dir() {
        let cfg =
            Project::generate_from_directory("resources/test").expect("Could not read config");
        assert_eq!(cfg.content.len(), 1); // 1 part
        assert_eq!(cfg.content[0].chapters.len(), 4); // 4 chapters in part 1
        assert_eq!(cfg.content[0].chapters[0].id, "01_getting_started");
        assert_eq!(cfg.content[0].chapters[1].id, "02_project_organisation");
        assert_eq!(cfg.content[0].chapters[2].id, "03_shortcodes");
        assert_eq!(cfg.content[0].chapters[3].id, "04_exercise_tools");
    }

    #[test]
    fn test_iteration_collect() {
        let doc = ProjectItem {
            id: "doc".to_string(),
            format: InputFormat::Markdown,
            path: Default::default(),
            content: Arc::new(()),
        };

        let cfg = Project {
            project_path: Default::default(),
            index: doc.clone(),
            content: vec![
                Part {
                    id: "part1".to_string(),
                    index: doc.clone(),
                    chapters: vec![
                        Chapter {
                            id: "chapter1".to_string(),
                            index: doc.clone(),
                            documents: vec![doc.clone(), doc.clone()],
                            files: vec![PathBuf::new()],
                        },
                        Chapter {
                            id: "chapter2".to_string(),
                            index: doc.clone(),
                            documents: vec![doc.clone(), doc.clone()],
                            files: vec![PathBuf::new()],
                        },
                    ],
                },
                Part {
                    id: "part2".to_string(),
                    index: doc,
                    chapters: vec![],
                },
            ],
        };

        let cfg_mapped: Project<()> = cfg.clone().into_iter().collect();

        for (p1, p2) in zip(cfg.content, cfg_mapped.content) {
            assert_eq!(p1.id, p2.id);
        }
    }
}
