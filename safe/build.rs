use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const SONAME: &str = "libyaml-0.so.2";
const VERSION_NODE: &str = "LIBYAML_0_2";
const PHASE_SYMBOLS_PATH: &str = "compat/upstream/exported-symbols-phase-01.txt";
const HIDDEN_PREFIX: &str = "phase2_hidden_";

const HIDDEN_API_SYMBOLS: &[&str] = &[
    "yaml_get_version_string",
    "yaml_get_version",
    "yaml_malloc",
    "yaml_realloc",
    "yaml_free",
    "yaml_strdup",
    "yaml_string_extend",
    "yaml_string_join",
    "yaml_stack_extend",
    "yaml_queue_extend",
    "yaml_parser_initialize",
    "yaml_parser_delete",
    "yaml_parser_set_input_string",
    "yaml_parser_set_input_file",
    "yaml_parser_set_input",
    "yaml_parser_set_encoding",
    "yaml_emitter_initialize",
    "yaml_emitter_delete",
    "yaml_emitter_set_output_string",
    "yaml_emitter_set_output_file",
    "yaml_emitter_set_output",
    "yaml_emitter_set_encoding",
    "yaml_emitter_set_canonical",
    "yaml_emitter_set_indent",
    "yaml_emitter_set_width",
    "yaml_emitter_set_unicode",
    "yaml_emitter_set_break",
    "yaml_token_delete",
    "yaml_stream_start_event_initialize",
    "yaml_stream_end_event_initialize",
    "yaml_document_start_event_initialize",
    "yaml_document_end_event_initialize",
    "yaml_alias_event_initialize",
    "yaml_scalar_event_initialize",
    "yaml_sequence_start_event_initialize",
    "yaml_sequence_end_event_initialize",
    "yaml_mapping_start_event_initialize",
    "yaml_mapping_end_event_initialize",
    "yaml_event_delete",
    "yaml_document_initialize",
    "yaml_document_delete",
    "yaml_document_get_node",
    "yaml_document_get_root_node",
    "yaml_document_add_scalar",
    "yaml_document_add_sequence",
    "yaml_document_add_mapping",
    "yaml_document_append_sequence_item",
    "yaml_document_append_mapping_pair",
    "yaml_parser_update_buffer",
    "yaml_parser_fetch_more_tokens",
    "yaml_parser_scan",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=compat/upstream/libyaml-0-2.symbols");
    println!("cargo:rerun-if-changed={PHASE_SYMBOLS_PATH}");
    println!("cargo:rerun-if-changed=../original/include/yaml.h");
    println!("cargo:rerun-if-changed=../original/src/yaml_private.h");
    println!("cargo:rerun-if-changed=../original/src/api.c");
    println!("cargo:rerun-if-changed=../original/src/reader.c");
    println!("cargo:rerun-if-changed=../original/src/scanner.c");

    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "CARGO_MANIFEST_DIR is not set")
        })?);
    let out_dir = PathBuf::from(
        env::var_os("OUT_DIR")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "OUT_DIR is not set"))?,
    );

    let manifest_path = manifest_dir.join("compat/upstream/libyaml-0-2.symbols");
    let phase_symbols_path = manifest_dir.join(PHASE_SYMBOLS_PATH);

    let upstream_symbols = parse_debian_symbols(&fs::read_to_string(&manifest_path)?)?;
    let phase_symbols = parse_symbol_list(&fs::read_to_string(&phase_symbols_path)?);

    validate_phase_subset(&upstream_symbols, &phase_symbols)?;
    compile_hidden_runtime(&manifest_dir, &out_dir)?;
    emit_version_script(&out_dir, &phase_symbols)?;
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

fn compile_hidden_runtime(manifest_dir: &Path, out_dir: &Path) -> Result<(), io::Error> {
    let repo_root = manifest_dir.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "safe crate should have a parent repository directory",
        )
    })?;
    let include_dir = repo_root.join("original/include");
    let source_dir = repo_root.join("original/src");
    let archive_path = out_dir.join("libyaml_phase2_hidden.a");
    let common_renames = out_dir.join("phase2-hidden-renames.h");
    let reader_renames = out_dir.join("phase2-hidden-reader-renames.h");
    let reader_wrapper = out_dir.join("phase2-hidden-reader-wrapper.c");

    fs::write(&common_renames, generate_rename_header(false))?;
    fs::write(&reader_renames, generate_rename_header(true))?;
    fs::write(&reader_wrapper, generate_reader_wrapper_source())?;

    let compiler = env::var("CC").unwrap_or_else(|_| String::from("cc"));
    let archiver = env::var("AR").unwrap_or_else(|_| String::from("ar"));

    let api_object = out_dir.join("phase2-hidden-api.o");
    let reader_object = out_dir.join("phase2-hidden-reader.o");
    let scanner_object = out_dir.join("phase2-hidden-scanner.o");
    let reader_wrapper_object = out_dir.join("phase2-hidden-reader-wrapper.o");

    compile_c_source(
        &compiler,
        &source_dir.join("api.c"),
        &api_object,
        &[include_dir.as_path(), source_dir.as_path()],
        Some(&common_renames),
    )?;
    compile_c_source(
        &compiler,
        &source_dir.join("reader.c"),
        &reader_object,
        &[include_dir.as_path(), source_dir.as_path()],
        Some(&reader_renames),
    )?;
    compile_c_source(
        &compiler,
        &source_dir.join("scanner.c"),
        &scanner_object,
        &[include_dir.as_path(), source_dir.as_path()],
        Some(&common_renames),
    )?;
    compile_c_source(
        &compiler,
        &reader_wrapper,
        &reader_wrapper_object,
        &[include_dir.as_path()],
        None,
    )?;

    run_command(
        Command::new(&archiver)
            .arg("rcs")
            .arg(&archive_path)
            .arg(&api_object)
            .arg(&reader_object)
            .arg(&scanner_object)
            .arg(&reader_wrapper_object),
        "archive hidden libyaml runtime",
    )?;

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=yaml_phase2_hidden");

    Ok(())
}

fn compile_c_source(
    compiler: &str,
    source: &Path,
    object: &Path,
    include_paths: &[&Path],
    forced_header: Option<&Path>,
) -> Result<(), io::Error> {
    let mut command = Command::new(compiler);
    command.arg("-std=c11");
    command.arg("-fPIC");
    command.arg("-c");
    command.arg(source);
    command.arg("-o");
    command.arg(object);
    command.arg("-DYAML_VERSION_MAJOR=0");
    command.arg("-DYAML_VERSION_MINOR=2");
    command.arg("-DYAML_VERSION_PATCH=5");
    command.arg("-DYAML_VERSION_STRING=\"0.2.5\"");

    for include_path in include_paths {
        command.arg("-I");
        command.arg(include_path);
    }

    if let Some(header) = forced_header {
        command.arg("-include");
        command.arg(header);
    }

    run_command(
        &mut command,
        &format!("compile hidden runtime source `{}`", source.display()),
    )
}

fn run_command(command: &mut Command, label: &str) -> Result<(), io::Error> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        format!(
            "{label} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    ))
}

fn generate_rename_header(reader_impl: bool) -> String {
    let mut header = String::new();
    header.push_str("/* generated by safe/build.rs */\n");
    for symbol in HIDDEN_API_SYMBOLS {
        let renamed = if reader_impl && *symbol == "yaml_parser_update_buffer" {
            format!("{HIDDEN_PREFIX}{symbol}_impl")
        } else {
            format!("{HIDDEN_PREFIX}{symbol}")
        };
        header.push_str("#define ");
        header.push_str(symbol);
        header.push(' ');
        header.push_str(&renamed);
        header.push('\n');
    }
    header
}

fn generate_reader_wrapper_source() -> String {
    format!(
        "/* generated by safe/build.rs */\n\
         #include <stddef.h>\n\
         #include <yaml.h>\n\
         \n\
         #define MAX_FILE_SIZE (~(size_t)0 / 2)\n\
         \n\
         int {prefix}yaml_parser_update_buffer_impl(yaml_parser_t *parser, size_t length);\n\
         \n\
         static int {prefix}set_input_too_long(yaml_parser_t *parser)\n\
         {{\n\
             if (!parser) {{\n\
                 return 0;\n\
             }}\n\
             parser->error = YAML_READER_ERROR;\n\
             parser->problem = \"input is too long\";\n\
             parser->problem_offset = parser->offset;\n\
             parser->problem_value = -1;\n\
             return 0;\n\
         }}\n\
         \n\
         int {prefix}yaml_parser_update_buffer(yaml_parser_t *parser, size_t length)\n\
         {{\n\
             int ok;\n\
             if (!parser) {{\n\
                 return 0;\n\
             }}\n\
             if (parser->offset >= MAX_FILE_SIZE) {{\n\
                 return {prefix}set_input_too_long(parser);\n\
             }}\n\
             ok = {prefix}yaml_parser_update_buffer_impl(parser, length);\n\
             if (ok && parser->offset >= MAX_FILE_SIZE) {{\n\
                 return {prefix}set_input_too_long(parser);\n\
             }}\n\
             return ok;\n\
         }}\n",
        prefix = HIDDEN_PREFIX
    )
}

fn emit_version_script(out_dir: &Path, phase_symbols: &[String]) -> Result<(), io::Error> {
    let script_path = out_dir.join("libyaml-phase.map");

    let mut version_script = String::new();
    version_script.push_str(VERSION_NODE);
    version_script.push_str(" {\n    global:\n");
    for symbol in phase_symbols {
        version_script.push_str("        ");
        version_script.push_str(symbol);
        version_script.push_str(";\n");
    }
    version_script.push_str("    local:\n        *;\n};\n");

    fs::write(&script_path, version_script)?;
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--version-script={}",
        script_path.display()
    );

    Ok(())
}

fn emit_linker_args() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,{SONAME}");
}
