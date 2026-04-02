use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

const SONAME: &str = "libyaml-0.so.2";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=compat/upstream/libyaml-0-2.symbols");
    println!("cargo:rerun-if-changed=compat/upstream/exported-symbols-phase-01.txt");

    let manifest_path = Path::new("compat/upstream/libyaml-0-2.symbols");
    let phase_symbols_path = Path::new("compat/upstream/exported-symbols-phase-01.txt");

    let upstream_symbols = parse_debian_symbols(&fs::read_to_string(manifest_path)?)?;
    let phase_symbols = parse_symbol_list(&fs::read_to_string(phase_symbols_path)?);

    validate_phase_subset(&upstream_symbols, &phase_symbols)?;
    emit_linker_args();

    Ok(())
}

fn parse_debian_symbols(contents: &str) -> Result<Vec<String>, io::Error> {
    let mut symbols = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('*') || trimmed.starts_with("libyaml-0.so.2") {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let symbol_with_version = match parts.next() {
            Some(value) => value,
            None => continue,
        };
        let symbol = symbol_with_version.split('@').next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "symbol line missing symbol name",
            )
        })?;
        if !symbol.is_empty() {
            symbols.push(symbol.to_owned());
        }
    }
    Ok(symbols)
}

fn parse_symbol_list(contents: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            symbols.push(trimmed.to_owned());
        }
    }
    symbols
}

fn validate_phase_subset(
    upstream_symbols: &[String],
    phase_symbols: &[String],
) -> Result<(), io::Error> {
    let upstream_set: BTreeSet<&str> = upstream_symbols.iter().map(String::as_str).collect();
    let mut seen = BTreeSet::new();
    for symbol in phase_symbols {
        let symbol_name = symbol.as_str();
        if !upstream_set.contains(symbol_name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("phase export `{symbol_name}` is not present in vendored Debian symbols"),
            ));
        }
        if !seen.insert(symbol_name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("phase export `{symbol_name}` is listed more than once"),
            ));
        }
    }
    Ok(())
}

fn emit_linker_args() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,{SONAME}");
}
