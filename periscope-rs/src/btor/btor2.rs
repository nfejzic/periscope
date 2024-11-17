use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
};

use serde::{Deserialize, Serialize};

use crate::btor::witness_format::PropKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub node: usize,
    pub _kind: PropKind,
    pub name: Option<String>,
}

/// Extracts the names of properties found in BTOR2.
///
/// # Example
///
/// `bad` and `justice` properties are supported, such as:
/// ```btor2
/// 43011 bad 43010 core-0-illegal-instruction
/// 43012 justice 43009 core-0-illegal-instruction
/// ```
/// Then the Property `bad` will be found and stored in the HashMap. The key for the given Property
/// is the index of the property in the file. The first property that appears has index 0, second
/// has index 1 and so on.
pub(super) fn get_property_names<R: Read>(input: R) -> HashMap<u64, Property> {
    let input = BufReader::new(input);
    input
        .lines()
        .filter(|line| match line {
            Ok(line) => line
                .split(' ')
                .nth(1)
                .is_some_and(|kind| kind == "bad" || kind == "justice"),
            Err(_) => false,
        })
        .enumerate()
        .filter_map(|(idx, line)| {
            let line = line.ok()?;
            let mut iter = line.split(' ');
            let node = iter.next()?.parse().ok()?;
            let kind: PropKind = iter.next()?.parse().ok()?;
            let name = iter.nth(1).map(String::from);
            let idx = idx.try_into().ok()?;

            Some((
                idx,
                Property {
                    node,
                    _kind: kind,
                    name,
                },
            ))
        })
        .collect()
}
