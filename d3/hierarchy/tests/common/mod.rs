//! Where d3-hierarchy's own test fixtures (`test/data/*.json`, `flare.csv`)
//! live: the `D3_HIERARCHY_DIR` checkout of d3/d3-hierarchy (v3.1.2), else
//! `$HOME/src/vendor/d3-hierarchy`.
#![allow(dead_code)]

use std::path::PathBuf;

pub fn d3_hierarchy_dir() -> PathBuf {
    match std::env::var("D3_HIERARCHY_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from(std::env::var("HOME").expect("HOME")).join("src/vendor/d3-hierarchy"),
    }
}

pub fn data(name: &str) -> PathBuf {
    d3_hierarchy_dir().join("test/data").join(name)
}
