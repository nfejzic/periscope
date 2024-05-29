use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{DirEntry, File},
    io::{StdoutLock, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::btor;

use self::hyperfine::Hyperfine;

mod hyperfine;
mod rotor;
mod wc;

// What I need to do:
//
// 1. Collect all btor2 files
// 2. run btormc on each of them -> collect the output
//    - we can run `wc` before running btormc to get the number of characters in the model
//    - we can also run `wc` on dump of `btormc` to get the character count without comments and
//    after `btormc` has optimized it
//    - we can run hyperfine with `--export-json /dev/stdout` to get json formatted runs
//    - we can then parse the json to get relevant information
//    - we can redirect the output of `btormc` to a temp file, which we parse to get information
//    about the model (number of steps, which bad state)

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BenchConfig {
    pub timeout: Option<u128>,
    #[serde(rename = "btormc-flags")]
    pub btormc_flags: Option<String>,
    pub files: Vec<String>,
    pub runs: HashMap<String, String>,

    #[serde(skip)]
    pub results_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Prop {
    kind: btor::PropKind,
    name: Option<String>,
    node: usize,
    idx: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum BenchResult {
    Success {
        props: Vec<Prop>,
        steps: usize,
        hyperfine: Hyperfine,
        wc_raw: usize,
        wc_btormc_dump: usize,
    },
    Failed {
        output: String,
        hyperfine: Hyperfine,
        wc_raw: usize,
        wc_btormc_dump: usize,
    },
}

/// Collects all `*.btor2` files in the given path and runs the `btormc` on them, benchmarking the
/// runs.
pub fn run_benches(
    path: PathBuf,
    bench_config: BenchConfig,
    make_target: Option<String>,
) -> anyhow::Result<()> {
    let dot_periscope = create_dot_periscope();
    let mut stdout = std::io::stdout().lock();

    if bench_config.runs.is_empty() {
        bench_file_or_dir(path, &dot_periscope, bench_config, &mut stdout)
    } else {
        run_benches_with_rotor(path, bench_config, &dot_periscope, make_target)
    }
}

fn bench_file_or_dir(
    path: PathBuf,
    dot_periscope: &Path,
    bench_config: BenchConfig,
    stdout: &mut StdoutLock,
) -> anyhow::Result<()> {
    let (mut results, results_path) =
        load_or_create_results(dot_periscope, bench_config.results_path);

    let mut paths: Vec<PathBuf> = Vec::new();

    if path.is_file() {
        paths.push(path);
    } else {
        paths.extend(
            std::fs::read_dir(&path)
                .expect("Could not open directory.")
                .flat_map(|e| e.ok().map(|e| e.path())),
        );
    }

    for path in paths {
        let bench_result = self::bench_file(
            &path,
            dot_periscope,
            stdout,
            bench_config.timeout,
            &bench_config.btormc_flags,
        )?;

        let filename = path
            .file_name()
            .and_then(OsStr::to_str)
            .map(String::from)
            .expect("Failed to get filename.");

        results.insert(filename, bench_result);
    }

    let mut results_file = File::create(results_path).unwrap();
    serde_json::to_writer_pretty(&mut results_file, &results)
        .expect("Failed serializing results into the results file.");

    Ok(())
}

fn run_benches_with_rotor(
    selfie_dir: PathBuf,
    config: BenchConfig,
    dot_periscope: &Path,
    make_target: Option<String>,
) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout().lock();

    for (name, rotor_args) in config.runs {
        println!("\nRunning '{name}':");

        // run rotor with the given config
        rotor::run_rotor(&selfie_dir, &rotor_args, &make_target)?;

        // collect filtered files
        let files: Vec<PathBuf> = std::fs::read_dir(selfie_dir.join("examples").join("symbolic"))?
            .filter_map(|entry| {
                // only files
                entry
                    .ok()
                    .and_then(|e| e.path().is_file().then(|| e.path()))
            })
            .filter(|p| {
                config
                    .files
                    .iter()
                    .any(|el| el == p.file_name().and_then(OsStr::to_str).unwrap_or_default())
            })
            .collect();

        // ensure results dir exists:
        let results_dir = dot_periscope.join("results");
        std::fs::create_dir_all(&results_dir).with_context(|| {
            format!(
                "Failed creating results directory at '{}'.",
                results_dir.display()
            )
        })?;
        // create results file
        let results_path = results_dir.join(format!("{}.json", name));
        let (mut results, results_path) = load_or_create_results(dot_periscope, Some(results_path));

        for file in files {
            let bench_result = bench_file(
                &file,
                dot_periscope,
                &mut stdout,
                config.timeout,
                &config.btormc_flags,
            )
            .with_context(|| format!("Failed benching file {}", file.display()))?;

            let filename = file
                .file_name()
                .and_then(OsStr::to_str)
                .map(String::from)
                .expect("Failed to get filename.");

            results.insert(filename, bench_result);
        }

        let mut results_file = File::create(&results_path)
            .with_context(|| format!("Failed creating '{}'", results_path.display()))?;
        serde_json::to_writer_pretty(&mut results_file, &results)
            .context("Failed serializing results into the results file.")?;
    }

    Ok(())
}

fn create_dot_periscope() -> PathBuf {
    let dot_periscope = PathBuf::from(".periscope/bench");

    if !dot_periscope.exists() || !dot_periscope.is_dir() {
        std::fs::create_dir_all(&dot_periscope)
            .unwrap_or_else(|err| panic!("Failed creating '{}': {}", dot_periscope.display(), err))
    }

    dot_periscope
}

fn load_or_create_results(
    dot_periscope: &Path,
    results_path: Option<PathBuf>,
) -> (HashMap<String, BenchResult>, PathBuf) {
    let results_path = results_path.unwrap_or_else(|| dot_periscope.join("results.json"));

    let results = File::open(&results_path)
        .context("Failed reading file.")
        .and_then(|f| serde_json::from_reader(&f).context("Falied deserializing results file."))
        .inspect_err(|err| {
            eprintln!(
                "Deserialization of '{}' failed: {}",
                results_path.display(),
                err
            )
        })
        .unwrap_or_default();

    (results, results_path)
}

fn bench_file(
    path: impl AsRef<Path>,
    dot_periscope: &Path,
    stdout: &mut std::io::StdoutLock,
    timeout: Option<u128>,
    btormc_flags: &Option<String>,
) -> anyhow::Result<BenchResult> {
    let path = path.as_ref();
    let wc_raw = wc::char_count_in_file(path)?;
    let wc_of_dump = wc::char_count_in_dump(path)?;

    debug_assert!(dot_periscope.exists());

    let file_name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();

    let hyperfine_out_path = dot_periscope.join(format!("{file_name}_hyperfine_output"));
    let hyperfine_json_path = dot_periscope.join(format!("{file_name}_hyperfine.json"));
    let hyperfine = hyperfine::run(
        path,
        &hyperfine_out_path,
        hyperfine_json_path,
        btormc_flags,
        timeout,
    )?;

    let mut props_in_steps = {
        if let Ok(witness) =
            btor::parse_btor_witness(File::open(&hyperfine_out_path)?, File::open(path).ok())
                .inspect_err(|_| {
                    let witness = std::fs::read_to_string(&hyperfine_out_path).unwrap_or_default();
                    format!("Failed parsing btor witness format: \n{}", witness);
                })
        {
            witness.props_in_steps()
        } else {
            return Ok(BenchResult::Failed {
                output: std::fs::read_to_string(hyperfine_out_path)?,
                hyperfine,
                wc_raw,
                wc_btormc_dump: wc_of_dump,
            });
        }
    };

    assert!(
        props_in_steps.len() == 1,
        "Expected only 1 frame from btor2 witness format, but found {}.",
        props_in_steps.len()
    );

    let props = props_in_steps[0]
        .0
        .inner
        .drain(..)
        .map(|mut p| {
            Ok(Prop {
                kind: p.kind,
                name: p.property.as_mut().and_then(|p| p.name.take()),
                node: p
                    .property
                    .map(|p| p.node)
                    .context("Node ID is mandatory in btor2.")?,
                idx: p.idx,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let steps = props_in_steps[0].1;

    if props_in_steps.len() == 1 {
        let _ = writeln!(
            stdout,
            "{}:\n\t{} characters, {} characters in dump.\n\tFound {} in {} steps.",
            path.file_name()
                .and_then(OsStr::to_str)
                .context("Invalid path to btor2 file.")?,
            wc_raw,
            wc_of_dump,
            props_in_steps[0].0.formatted_string(),
            props_in_steps[0].1
        );
    }

    Ok(BenchResult::Success {
        props,
        steps,
        hyperfine,
        wc_raw,
        wc_btormc_dump: wc_of_dump,
    })
}

trait IsBtor2 {
    fn is_btor2(&self) -> bool;
}

impl IsBtor2 for DirEntry {
    fn is_btor2(&self) -> bool {
        matches!(
            self.path().extension().and_then(OsStr::to_str),
            Some("btor2")
        )
    }
}
