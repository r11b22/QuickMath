//! Compiles the practice levels in `src/practice/lessons.toml` into Rust.
//!
//! Doing this at build time keeps a TOML parser out of the program and turns
//! mistakes in the file into build errors that name the offending level.

#[path = "build/lesson_tables.rs"]
mod lesson_tables;

use std::path::Path;
use std::{env, fs, process};

const LESSONS_FILE: &str = "src/practice/lessons.toml";
const GENERATED_FILE: &str = "lesson_tables.rs";

fn main() {
    println!("cargo::rerun-if-changed={LESSONS_FILE}");
    println!("cargo::rerun-if-changed=build/lesson_tables.rs");
    let source = fs::read_to_string(LESSONS_FILE)
        .unwrap_or_else(|error| fail(&format!("cannot read {LESSONS_FILE}: {error}")));
    let tables = lesson_tables::generate(&source)
        .unwrap_or_else(|problem| fail(&format!("{LESSONS_FILE}: {problem}")));
    let out_dir = env::var("OUT_DIR").expect("cargo sets OUT_DIR for build scripts");
    let path = Path::new(&out_dir).join(GENERATED_FILE);
    fs::write(&path, tables)
        .unwrap_or_else(|error| fail(&format!("cannot write {}: {error}", path.display())));
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    process::exit(1);
}
