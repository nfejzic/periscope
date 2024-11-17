use std::{collections::HashSet, ffi::OsStr, io::Read, path::PathBuf};

use anyhow::Context;
use bench::BenchConfig;
use clap::{Parser, Subcommand};

pub mod bench;
pub mod btor;
mod selfie;

#[derive(Debug, Clone, Parser)]
#[clap(long_about)]
pub struct Config {
    /// Parse witness format of btormc generated from btor2 model. Parses from stdin if path to
    /// file is not provided.
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Subcommand)]
#[command(long_about)]
pub enum Commands {
    ParseWitness {
        /// Path to the witness file.
        file: Option<PathBuf>,

        /// Path to the BTOR2 model file, typically ends with '.btor2' extension.
        #[arg(short, long)]
        btor2: Option<PathBuf>,
    },

    Bench {
        /// Path to the results file where the benchmark results will be stored in JSON format.
        /// By default, the results will be stored in the '.periscope/bench/results.json' file.
        ///
        /// If 'run-rotor' flag is provided, then the results are stored in
        /// '.periscope/bench/results/{run-name}.json' regardless of this option.
        #[arg(long)]
        results_path: Option<PathBuf>,

        /// Whether to run rotor to generate files first.
        #[arg(short = 'r', long = "run-rotor")]
        run_rotor: bool,

        /// Files that should be benchmarked. Files that do not match the provided names will be
        /// ignored.
        ///
        /// The 'filter-files' option has priority if both 'filter-files' and 'filter-config' are
        /// provided.
        #[arg(short, long, requires = "run_rotor")]
        filter_files: Vec<String>,

        /// Config file that should be used for filtering. This is an alternative to using the
        /// 'filter-files' option. The file can be in JSON or YAML format.
        ///
        /// The 'filter-files' option has priority if both 'filter-files' and 'filter-config' are
        /// provided.
        ///
        /// # Example:
        ///
        /// ```yaml
        /// # timeout in seconds
        /// timeout: 300 # 5m = (5 * 60) s = 300 seconds
        /// files:
        ///   - "file1.btor2"
        ///   - "file2.btor2"
        ///   - "file3.btor3"
        ///
        /// runs:
        ///   8-bit-codeword-size: "0 -codewordsize 8"
        ///   16-bit-codeword-size: "0 -codewordsize 16"
        /// ```
        #[arg(short = 'c', long, requires = "run_rotor", verbatim_doc_comment)]
        bench_config: Option<PathBuf>,

        /// Path to the directory that contains selfie and rotor. You can clone selfie from
        /// [selfie's Github repository](https://www.github.com/cksystemsteaching/selfie).
        #[arg(short = 's', long = "selfie-dir")]
        selfie_dir: Option<PathBuf>,

        #[arg(long = "force-clone-selfie", default_value = "false")]
        force_clone_selfie: bool,

        /// Path to folder containing BTOR2 files. All BTOR2 files should have the ".btor2"
        /// extension. Alternatively, path to a single BTOR2 file can be provided for single
        /// benchmark.
        #[arg(required_unless_present("run_rotor"))]
        path: Option<PathBuf>,

        /// Target for runing `make` inside of the selfie directory.
        #[arg(short = 'm', long = "make-target")]
        make_target: Option<String>,

        /// Number of parallel benchmarks to run. By default benchmarks are run sequentially.
        /// However, if you have multiple CPU cores, you can spin-up multiple benchmarks in
        /// parallel. Maximum value is 255.
        #[arg(short = 'j', long = "jobs", default_value = "1")]
        jobs: u8,
    },
}

pub fn run(config: Config) -> anyhow::Result<()> {
    match config.command {
        Commands::ParseWitness { file, btor2 } => {
            let witness: &mut dyn Read = match file {
                Some(path) => &mut std::fs::File::open(path).unwrap(),
                None => &mut std::io::stdin(),
            };

            let btor2 = btor2.and_then(|path| {
                std::fs::File::open(path)
                    .inspect_err(|err| {
                        println!("Could not open provided btor2 file: {}", err);
                    })
                    .ok()
            });

            let witness = btor::parse_btor_witness(witness, btor2)?;

            witness.analyze_and_report();
        }
        Commands::Bench {
            path,
            run_rotor,
            results_path,
            filter_files,
            bench_config,
            selfie_dir,
            force_clone_selfie: clone_selfie,
            make_target,
            jobs,
        } => {
            let dot_periscope = crate::create_dot_periscope();

            let btor_files = if run_rotor {
                match selfie_dir {
                    Some(selfie_dir) => selfie_dir,
                    None => selfie::clone_selfie(&dot_periscope, clone_selfie)
                        .context("No selfie dir is provided, and cloning selfie failed.")?,
                }
            } else {
                path.context(
                    "Path to a BTOR2 file or directory containing BTOR2 files is required.",
                )?
            };

            let filter_files = HashSet::from_iter(filter_files);
            let config = prepare_bench_config(run_rotor, filter_files, bench_config, results_path)?;

            bench::run_benches(btor_files, &dot_periscope, config, make_target, jobs)?;
        }
    };

    Ok(())
}

/// Reads and deserializes the configuration file for benchmarking. If no file is provided, default
/// configuration values are used.
fn prepare_bench_config(
    run_rotor: bool,
    filter_files: HashSet<String>,
    bench_config: Option<PathBuf>,
    results_path: Option<PathBuf>,
) -> anyhow::Result<BenchConfig> {
    let mut config = BenchConfig::default();

    if run_rotor {
        config = bench_config
            .map(|path| {
                let file = std::fs::File::open(&path)
                    .map_err(|err| anyhow::format_err!("Could not open config file: {err}"))?;

                match path.extension().and_then(OsStr::to_str) {
                    Some("json") => {
                        serde_json::from_reader(file).context("Config has invalid JSON format.")
                    }
                    Some("yaml") => {
                        serde_yaml::from_reader(file).context("Config has invalid YAML format.")
                    }
                    _ => anyhow::bail!("Config file must be in JSON or YAML format."),
                }
            })
            .transpose()?
            .unwrap_or_default();

        if !filter_files.is_empty() {
            config.files = filter_files;
        }
    }

    config.results_path = results_path;

    Ok(config)
}

/// Creates the `.periscope` directory for temporary data generated by the `periscope` command.
fn create_dot_periscope() -> PathBuf {
    let dot_periscope = PathBuf::from(".periscope/bench");

    if !dot_periscope.exists() || !dot_periscope.is_dir() {
        std::fs::create_dir_all(&dot_periscope)
            .unwrap_or_else(|err| panic!("Failed creating '{}': {}", dot_periscope.display(), err))
    }

    dot_periscope
}
