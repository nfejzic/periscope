use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::bench::BenchConfig;

const SELFIE_URL: &str = "https://github.com/cksystemsteaching/selfie";
const SELFIE_COMMIT_HASH: &str = "24cc81a90f68f1d8678ef4a810db4d706483c661";

pub fn clone_selfie(dot_periscope: &Path, force_clone: bool) -> anyhow::Result<PathBuf> {
    let selfie_path = dot_periscope.join("selfie");

    if selfie_path.exists() {
        let commit_hash = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(&selfie_path)
            .output()?
            .stdout;

        let status = Command::new("git")
            .current_dir(&selfie_path)
            .arg("status")
            .arg("--short")
            .arg(".")
            .output()?
            .stdout;

        let is_dirty = !status.is_empty();

        if commit_hash.trim_ascii() == SELFIE_COMMIT_HASH.as_bytes() {
            if is_dirty && !force_clone {
                anyhow::bail!("Selfie is cloned and has correct commit, but it is modified.");
            } else if !is_dirty {
                // selfie is already cloned, correct commit is checked out.
                return Ok(selfie_path);
            }
        } else if !force_clone {
            anyhow::bail!("Selfie is cloned, but checked out commit is wrong.");
        }
    }

    if selfie_path.exists() && force_clone {
        std::fs::remove_dir_all(&selfie_path)?;
    }

    println!("Cloning selfie...");

    anyhow::ensure!(
        Command::new("git")
            .arg("clone")
            .arg(SELFIE_URL)
            .arg(&selfie_path)
            .status()?
            .success(),
        "Could not clone selfie repository."
    );

    anyhow::ensure!(
        Command::new("git")
            .arg("checkout")
            .arg(SELFIE_COMMIT_HASH)
            .current_dir(&selfie_path)
            .status()?
            .success(),
        "Could not checkout the right commit hash in selfie repository"
    );

    Ok(selfie_path)
}

pub fn collect_btor_files(selfie_dir: &Path, config: &BenchConfig) -> anyhow::Result<Vec<PathBuf>> {
    let files = std::fs::read_dir(selfie_dir.join("examples").join("symbolic"))?
        .filter_map(|entry| {
            // only files
            entry
                .ok()
                .and_then(|e| e.path().is_file().then(|| e.path()))
        })
        .filter(|path| path.extension().is_some_and(|ext| ext == "btor2"));

    Ok(config.filter_files(files))
}
