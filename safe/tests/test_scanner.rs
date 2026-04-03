use std::env;
use std::ffi::CStr;
use std::fs;
use std::mem;
use std::path::PathBuf;
use std::process::Command;
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};

use yaml::{
    yaml_parser_delete, yaml_parser_initialize, yaml_parser_scan, yaml_parser_set_input,
    yaml_parser_set_input_string, yaml_parser_t, yaml_token_delete, yaml_token_t,
    yaml_token_type_t,
};

#[test]
fn scans_simple_flow_sequence_tokens() {
    let tokens = scan_types(b"[item 1, item 2]\n").expect("flow sequence should scan");
    assert_eq!(
        tokens,
        vec![
            yaml_token_type_t::YAML_STREAM_START_TOKEN,
            yaml_token_type_t::YAML_FLOW_SEQUENCE_START_TOKEN,
            yaml_token_type_t::YAML_SCALAR_TOKEN,
            yaml_token_type_t::YAML_FLOW_ENTRY_TOKEN,
            yaml_token_type_t::YAML_SCALAR_TOKEN,
            yaml_token_type_t::YAML_FLOW_SEQUENCE_END_TOKEN,
            yaml_token_type_t::YAML_STREAM_END_TOKEN,
        ]
    );
}

#[test]
fn scans_simple_block_mapping_tokens() {
    let tokens = scan_types(b"key: value\n").expect("block mapping should scan");
    assert_eq!(
        tokens,
        vec![
            yaml_token_type_t::YAML_STREAM_START_TOKEN,
            yaml_token_type_t::YAML_BLOCK_MAPPING_START_TOKEN,
            yaml_token_type_t::YAML_KEY_TOKEN,
            yaml_token_type_t::YAML_SCALAR_TOKEN,
            yaml_token_type_t::YAML_VALUE_TOKEN,
            yaml_token_type_t::YAML_SCALAR_TOKEN,
            yaml_token_type_t::YAML_BLOCK_END_TOKEN,
            yaml_token_type_t::YAML_STREAM_END_TOKEN,
        ]
    );
}

#[test]
fn scans_checked_in_examples_without_failure() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for relative in [
        "compat/examples/anchors.yaml",
        "compat/examples/json.yaml",
        "compat/examples/mapping.yaml",
        "compat/examples/tags.yaml",
    ] {
        let input = fs::read(manifest_dir.join(relative)).expect("failed to read example");
        let tokens = scan_types(&input).unwrap_or_else(|error| {
            panic!("{relative}: scanner failed: {error}");
        });
        assert!(
            !tokens.is_empty(),
            "{relative}: expected at least a stream-start token"
        );
        assert_eq!(
            tokens.last().copied(),
            Some(yaml_token_type_t::YAML_STREAM_END_TOKEN),
            "{relative}"
        );
    }
}

#[test]
fn chunked_multibyte_simple_keys_use_character_indexes() {
    let mut input = Vec::with_capacity(513 * 2 + 8);
    for _ in 0..513 {
        input.extend_from_slice(b"\xD1\x8F");
    }
    input.extend_from_slice(b": value\n");

    let tokens =
        scan_types_with_chunk(&input, 1).expect("chunked multibyte simple key should scan");
    assert_eq!(
        tokens,
        vec![
            yaml_token_type_t::YAML_STREAM_START_TOKEN,
            yaml_token_type_t::YAML_BLOCK_MAPPING_START_TOKEN,
            yaml_token_type_t::YAML_KEY_TOKEN,
            yaml_token_type_t::YAML_SCALAR_TOKEN,
            yaml_token_type_t::YAML_VALUE_TOKEN,
            yaml_token_type_t::YAML_SCALAR_TOKEN,
            yaml_token_type_t::YAML_BLOCK_END_TOKEN,
            yaml_token_type_t::YAML_STREAM_END_TOKEN,
        ]
    );
}

#[test]
fn staged_install_runs_phase2_c_probes_and_upstream_run_scanner() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stage_root = temp_dir("stage-root-phase-2");
    let arch = multiarch();
    let stage_lib_dir = stage_root.join("usr/lib").join(&arch);
    let compiler = compiler();

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/stage-install.sh"))
            .arg(&stage_root),
        "stage-install",
    );

    let parser_input_safe = temp_dir("parser-input-api-safe").join("parser-input-api-safe");
    run_command(
        Command::new(&compiler)
            .arg("-I")
            .arg(stage_root.join("usr/include"))
            .arg(manifest_dir.join("tests/fixtures/parser_input_api.c"))
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&parser_input_safe),
        "compile staged parser_input_api.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&parser_input_safe),
        "assert staged loader for parser_input_api staged-header mode",
    );
    run_command(
        &mut Command::new(&parser_input_safe),
        "run parser_input_api staged-header mode",
    );

    let parser_input_object =
        temp_dir("parser-input-api-link-safe").join("parser-input-api-safe.o");
    run_command(
        Command::new(&compiler)
            .arg("-c")
            .arg("-I")
            .arg(manifest_dir.join("include"))
            .arg(manifest_dir.join("tests/fixtures/parser_input_api.c"))
            .arg("-o")
            .arg(&parser_input_object),
        "compile parser_input_api.c against vendored header",
    );
    let parser_input_link =
        temp_dir("parser-input-api-link-safe").join("parser-input-api-link-safe");
    run_command(
        Command::new(&compiler)
            .arg(&parser_input_object)
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&parser_input_link),
        "link parser_input_api object against staged library",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&parser_input_link),
        "assert staged loader for parser_input_api object-link mode",
    );
    run_command(
        &mut Command::new(&parser_input_link),
        "run parser_input_api object-link mode",
    );

    let private_exports_safe =
        temp_dir("private-parser-exports-safe").join("private-parser-exports-safe");
    run_command(
        Command::new(&compiler)
            .arg("-I")
            .arg(stage_root.join("usr/include"))
            .arg(manifest_dir.join("tests/fixtures/private_parser_exports.c"))
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&private_exports_safe),
        "compile staged private_parser_exports.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&private_exports_safe),
        "assert staged loader for private_parser_exports staged-header mode",
    );
    run_command(
        &mut Command::new(&private_exports_safe),
        "run private_parser_exports staged-header mode",
    );

    let private_exports_object =
        temp_dir("private-parser-exports-link-safe").join("private-parser-exports-safe.o");
    run_command(
        Command::new(&compiler)
            .arg("-c")
            .arg("-I")
            .arg(manifest_dir.join("include"))
            .arg(manifest_dir.join("tests/fixtures/private_parser_exports.c"))
            .arg("-o")
            .arg(&private_exports_object),
        "compile private_parser_exports.c against vendored header",
    );
    let private_exports_link =
        temp_dir("private-parser-exports-link-safe").join("private-parser-exports-link-safe");
    run_command(
        Command::new(&compiler)
            .arg(&private_exports_object)
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&private_exports_link),
        "link private_parser_exports object against staged library",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&private_exports_link),
        "assert staged loader for private_parser_exports object-link mode",
    );
    run_command(
        &mut Command::new(&private_exports_link),
        "run private_parser_exports object-link mode",
    );

    let run_scanner_binary = temp_dir("run-scanner-safe").join("run-scanner-safe");
    run_command(
        Command::new(&compiler)
            .arg("-I")
            .arg(stage_root.join("usr/include"))
            .arg(manifest_dir.join("compat/original-tests/run-scanner.c"))
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&run_scanner_binary),
        "compile upstream run-scanner.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&run_scanner_binary),
        "assert staged loader for run-scanner",
    );
    let run_scanner_output = Command::new(&run_scanner_binary)
        .arg(manifest_dir.join("compat/examples/anchors.yaml"))
        .arg(manifest_dir.join("compat/examples/json.yaml"))
        .arg(manifest_dir.join("compat/examples/mapping.yaml"))
        .arg(manifest_dir.join("compat/examples/tags.yaml"))
        .output()
        .expect("failed to run upstream run-scanner");
    assert!(
        run_scanner_output.status.success(),
        "upstream run-scanner exited with failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_scanner_output.stdout),
        String::from_utf8_lossy(&run_scanner_output.stderr)
    );
    let stdout =
        String::from_utf8(run_scanner_output.stdout).expect("run-scanner emitted invalid UTF-8");
    assert!(!stdout.contains("FAILURE"), "{stdout}");
    assert_eq!(stdout.matches("SUCCESS").count(), 4, "{stdout}");
}

fn scan_types(input: &[u8]) -> Result<Vec<yaml_token_type_t>, String> {
    scan_types_with_chunk(input, 0)
}

fn scan_types_with_chunk(input: &[u8], chunk: usize) -> Result<Vec<yaml_token_type_t>, String> {
    let mut parser = unsafe { mem::zeroed::<yaml_parser_t>() };
    let mut reader = MemoryReader {
        input: input.as_ptr(),
        size: input.len(),
        offset: 0,
        chunk,
    };
    unsafe {
        if yaml_parser_initialize(&mut parser) == 0 {
            return Err(String::from("yaml_parser_initialize failed"));
        }
        if chunk == 0 {
            yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());
        } else {
            yaml_parser_set_input(
                &mut parser,
                Some(memory_read_handler),
                (&mut reader as *mut MemoryReader).cast(),
            );
        }

        let mut tokens = Vec::new();
        loop {
            let mut token = mem::zeroed::<yaml_token_t>();
            if yaml_parser_scan(&mut parser, &mut token) == 0 {
                let problem = if parser.problem.is_null() {
                    String::from("(no problem)")
                } else {
                    CStr::from_ptr(parser.problem)
                        .to_string_lossy()
                        .into_owned()
                };
                let message = format!(
                    "error={:?} problem={problem} line={} column={}",
                    parser.error, parser.problem_mark.line, parser.problem_mark.column
                );
                yaml_parser_delete(&mut parser);
                return Err(message);
            }

            let token_type = token.r#type;
            tokens.push(token_type);
            yaml_token_delete(&mut token);
            if token_type == yaml_token_type_t::YAML_STREAM_END_TOKEN {
                break;
            }
        }

        yaml_parser_delete(&mut parser);
        Ok(tokens)
    }
}

#[repr(C)]
struct MemoryReader {
    input: *const u8,
    size: usize,
    offset: usize,
    chunk: usize,
}

unsafe extern "C" fn memory_read_handler(
    data: *mut core::ffi::c_void,
    buffer: *mut u8,
    size: usize,
    size_read: *mut usize,
) -> i32 {
    let reader = &mut *data.cast::<MemoryReader>();
    let remaining = reader.size.saturating_sub(reader.offset);
    if remaining == 0 {
        *size_read = 0;
        return 1;
    }

    let mut limit = size;
    if reader.chunk != 0 && limit > reader.chunk {
        limit = reader.chunk;
    }
    let count = limit.min(remaining);
    ptr::copy_nonoverlapping(reader.input.add(reader.offset), buffer, count);
    reader.offset += count;
    *size_read = count;
    1
}

fn compiler() -> String {
    match env::var("CC") {
        Ok(value) if !value.is_empty() => value,
        _ => String::from("cc"),
    }
}

fn multiarch() -> String {
    for candidate in ["cc", "gcc"] {
        let output = Command::new(candidate).arg("-print-multiarch").output();
        if let Ok(value) = output {
            if value.status.success() {
                let arch = String::from_utf8_lossy(&value.stdout).trim().to_owned();
                if !arch.is_empty() {
                    return arch;
                }
            }
        }
    }

    format!("{}-linux-gnu", env::consts::ARCH)
}

fn run_command(command: &mut Command, label: &str) {
    let output = command.output().expect("failed to spawn command");
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temp_dir(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-artifacts")
        .join(format!("{prefix}-{nonce}"));
    fs::create_dir_all(&dir).expect("failed to create temp directory");
    dir
}
