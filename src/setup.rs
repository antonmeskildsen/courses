use std::io::Write;
use std::path::PathBuf;
use std::{fs, io};

use fs_extra::dir::CopyOptions;
use penguin::hyper::body::Bytes;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "resources/bundle/"]
struct Asset;

pub(crate) fn setup(dir: PathBuf, repository: String) -> anyhow::Result<()> {
    let mut temp_file = tempfile::tempfile()?;
    let temp_dir = tempfile::tempdir()?;

    // Download and write template zip
    let res: Bytes = reqwest::blocking::get(repository)?.bytes()?;
    temp_file.write_all(&res)?;
    // Create and extract zip archive
    let mut z = zip::ZipArchive::new(temp_file)?;
    z.extract(temp_dir.path())?;

    // Find repository directory in zip file
    let from_dir = fs::read_dir(temp_dir.path())?
        .into_iter()
        .find(|e| {
            e.as_ref()
                .map(|e| Ok::<bool, io::Error>(e.metadata()?.is_dir()))
                .is_ok()
        })
        .unwrap()?;
    // Copy zip contents to the final destination (without the enclosing folder)
    let mut opts = CopyOptions::new();
    opts.content_only = true;
    fs_extra::dir::copy(from_dir.path(), dir, &opts)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_default() {
        let tmpdir = tempfile::tempdir().unwrap();

        setup(
            tmpdir.path().to_path_buf(),
            "https://github.com/coursesproject/courses-template-default/archive/main.zip"
                .to_string(),
        )
        .expect("Setup failed");

        let index_path = tmpdir.path().join("content/index.md");
        let tp_index_path = tmpdir.path().join("templates/index.tera.html");
        let tp_section_path = tmpdir.path().join("templates/section.tera.html");
        let tp_nav_path = tmpdir.path().join("templates/nav.tera.html");
        assert!(index_path.is_file(), "Missing index.md");
        assert!(tp_index_path.is_file(), "Missing index.tera.html");
        assert!(tp_section_path.is_file(), "Missing section.tera.html");
        assert!(tp_nav_path.is_file(), "Missing nav.tera.html");
    }
}
