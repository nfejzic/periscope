use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// Run rotor in the provided selfie directory. Make sure that the following make targets exist:
/// * `clean`
/// * `rotor-symbolic`
///
/// Other make targets can be run by providing the corresponding CLI flag. [`Commands::Bench`] for more
/// information.
///
/// [`Commands::Bench`]: crate::Commands::Bench
pub fn run_rotor(
    selfie_dir: &Path,
    rotor_args: &str,
    make_target: &Option<String>,
) -> anyhow::Result<()> {
    // make sure we start fresh
    Command::new("make")
        .arg("clean")
        .current_dir(selfie_dir)
        .spawn()?
        .wait()?;

    if let Some(make_target) = make_target {
        Command::new("make")
            .arg(make_target)
            .arg(format!("rotor={}", rotor_args))
            .current_dir(selfie_dir)
            .spawn()?
            .wait()?;

        Ok(())
    } else {
        for file in collect_example_c_files(&selfie_dir.join("examples").join("symbolic"))? {
            let file_parent_path = file
                .strip_prefix(selfie_dir)?
                .parent()
                .map(|path| path.display().to_string() + "/")
                .unwrap_or_default();

            let Some(file_stem) = file.file_stem().map(|fs| fs.to_string_lossy()) else {
                continue;
            };

            let make_target = format!("{file_parent_path}{file_stem}-rotorized.btor2",);

            Command::new("make")
                .current_dir(selfie_dir)
                .arg(make_target)
                .arg(format!("rotor={rotor_args}"))
                .spawn()?
                .wait()?;
        }

        Ok(())
    }
}

fn collect_example_c_files(path: &Path) -> anyhow::Result<impl Iterator<Item = PathBuf>> {
    let read_dir = std::fs::read_dir(path)?;
    let filtered_files = read_dir
        .filter_map(|maybe_dir_entry| maybe_dir_entry.ok())
        .map(|dir_entry| dir_entry.path())
        .filter(|path| path.extension().unwrap_or_default() == "c");

    Ok(filtered_files)
}
