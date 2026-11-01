//! Compiles the math vocabulary in `src/math/syntax.toml` into Rust tables.
//!
//! Doing this at build time keeps a TOML parser out of the program and turns
//! mistakes in the file into build errors that name the offending entry.

#[path = "build/syntax_tables.rs"]
mod syntax_tables;

use std::path::Path;
use std::{env, fs, process};

const SYNTAX_FILE: &str = "src/math/syntax.toml";
const GENERATED_FILE: &str = "syntax_tables.rs";

fn main() {
    println!("cargo::rerun-if-changed={SYNTAX_FILE}");
    println!("cargo::rerun-if-changed=build/syntax_tables.rs");
    let source = fs::read_to_string(SYNTAX_FILE)
        .unwrap_or_else(|error| fail(&format!("cannot read {SYNTAX_FILE}: {error}")));
    let tables = syntax_tables::generate(&source)
        .unwrap_or_else(|problem| fail(&format!("{SYNTAX_FILE}: {problem}")));
    let out_dir = env::var("OUT_DIR").expect("cargo sets OUT_DIR for build scripts");
    let path = Path::new(&out_dir).join(GENERATED_FILE);
    fs::write(&path, tables)
        .unwrap_or_else(|error| fail(&format!("cannot write {}: {error}", path.display())));
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    process::exit(1);
}
