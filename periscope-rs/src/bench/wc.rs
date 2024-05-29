use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;

pub fn char_count_in_file(file: impl AsRef<Path>) -> anyhow::Result<usize> {
    let wc = Command::new("wc")
        .arg("-c")
        .arg(file.as_ref())
        .output()?
        .stdout;

    parse_wc_output(&wc)
}

pub fn char_count_in_dump(path: impl AsRef<Path>) -> anyhow::Result<usize> {
    let s = Command::new("btormc")
        .arg("-d")
        .arg(path.as_ref())
        .output()?
        .stdout;

    let mut wc_child = Command::new("wc")
        .arg("-c")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = wc_child.stdin.as_mut() {
        stdin.write_all(&s)?;
    }

    let wc = wc_child.wait_with_output()?.stdout;

    parse_wc_output(&wc)
}

fn parse_wc_output(output: &[u8]) -> anyhow::Result<usize> {
    let idx = output
        .iter()
        .position(|c| c == &b' ')
        .context("Bad output from 'wc' command.")?;

    let wc = std::str::from_utf8(output[..idx].as_ref())?;

    wc.parse()
        .context("Could not parse output of 'wc' command.")
}
