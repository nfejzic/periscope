use clap::Parser;
use periscope::Config;

fn main() -> anyhow::Result<()> {
    let config = Config::parse();
    periscope::run(config)
}
