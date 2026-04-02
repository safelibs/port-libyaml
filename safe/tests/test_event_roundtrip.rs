use std::env;
use std::ffi::CStr;
use std::fs::{self, File};
use std::mem;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::ptr;
use std::slice;
use std::time::{SystemTime, UNIX_EPOCH};

use yaml::{
    yaml_document_end_event_initialize, yaml_document_start_event_initialize, yaml_emitter_delete,
    yaml_emitter_emit, yaml_emitter_flush, yaml_emitter_initialize, yaml_emitter_set_break,
    yaml_emitter_set_encoding, yaml_emitter_set_indent, yaml_emitter_set_output,
    yaml_emitter_set_output_string, yaml_emitter_set_unicode, yaml_emitter_set_width,
    yaml_emitter_t, yaml_error_type_t, yaml_event_delete, yaml_event_t, yaml_mapping_end_event_initialize,
    yaml_mapping_start_event_initialize, yaml_mapping_style_t, yaml_parser_delete,
    yaml_parser_initialize, yaml_parser_parse, yaml_parser_set_input_string, yaml_parser_t,
    yaml_scalar_event_initialize, yaml_scalar_style_t, yaml_stream_end_event_initialize,
    yaml_stream_start_event_initialize,
};

const UTF8_GREETING: &[u8] = "Hi is Привет".as_bytes();

#[repr(C)]
struct CallbackWriter {
    output: Vec<u8>,
    capacity: usize,
}

unsafe extern "C" fn callback_write_handler(
    data: *mut core::ffi::c_void,
    buffer: *mut u8,
    size: usize,
) -> i32 {
    let writer = &mut *data.cast::<CallbackWriter>();
    if writer.output.len().saturating_add(size) > writer.capacity {
        return 0;
    }
    writer
        .output
        .extend_from_slice(slice::from_raw_parts(buffer.cast_const(), size));
    1
}

unsafe fn emit_scalar_mapping_document(emitter: *mut yaml_emitter_t, value: &[u8]) -> i32 {
    let key = b"message";
    let mut event = mem::zeroed::<yaml_event_t>();

    if yaml_stream_start_event_initialize(
        &mut event,
        yaml::yaml_encoding_t::YAML_UTF8_ENCODING,
    ) == 0
        || yaml_emitter_emit(emitter, &mut event) == 0
    {
        return 0;
    }

    if yaml_document_start_event_initialize(&mut event, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), 0) == 0
        || yaml_emitter_emit(emitter, &mut event) == 0
    {
        return 0;
    }

    if yaml_mapping_start_event_initialize(
        &mut event,
        ptr::null(),
        ptr::null(),
        1,
        yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE,
    ) == 0
        || yaml_emitter_emit(emitter, &mut event) == 0
    {
        return 0;
    }

    if yaml_scalar_event_initialize(
        &mut event,
        ptr::null(),
        ptr::null(),
        key.as_ptr(),
        key.len() as i32,
        1,
        1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    ) == 0
        || yaml_emitter_emit(emitter, &mut event) == 0
    {
        return 0;
    }

    if yaml_scalar_event_initialize(
        &mut event,
        ptr::null(),
        ptr::null(),
        value.as_ptr(),
        value.len() as i32,
        1,
        1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    ) == 0
        || yaml_emitter_emit(emitter, &mut event) == 0
    {
        return 0;
    }

    if yaml_mapping_end_event_initialize(&mut event) == 0 || yaml_emitter_emit(emitter, &mut event) == 0 {
        return 0;
    }

    if yaml_document_end_event_initialize(&mut event, 0) == 0 || yaml_emitter_emit(emitter, &mut event) == 0 {
        return 0;
    }

    if yaml_stream_end_event_initialize(&mut event) == 0 || yaml_emitter_emit(emitter, &mut event) == 0 {
        return 0;
    }

    1
}

unsafe fn assert_output_contains_scalar(output: &[u8], expected: &[u8]) {
    let mut parser = mem::zeroed::<yaml_parser_t>();
    let mut event = mem::zeroed::<yaml_event_t>();
    let mut found = false;

    assert_eq!(yaml_parser_initialize(&mut parser), 1);
    yaml_parser_set_input_string(&mut parser, output.as_ptr(), output.len());

    loop {
        assert_eq!(yaml_parser_parse(&mut parser, &mut event), 1);
        if event.r#type == yaml::yaml_event_type_t::YAML_SCALAR_EVENT
            && slice::from_raw_parts(event.data.scalar.value, event.data.scalar.length) == expected
        {
            found = true;
        }
        let done = event.r#type == yaml::yaml_event_type_t::YAML_STREAM_END_EVENT;
        yaml_event_delete(&mut event);
        if done {
            break;
        }
    }

    yaml_parser_delete(&mut parser);
    assert!(found, "expected scalar {:?} not found in emitted stream", expected);
}

#[test]
fn emitter_string_output_roundtrips_and_applies_public_setters() {
    unsafe {
        let mut emitter = mem::zeroed::<yaml_emitter_t>();
        let mut output = [0u8; 512];
        let mut written = usize::MAX;

        assert_eq!(yaml_emitter_initialize(&mut emitter), 1);
        yaml_emitter_set_output_string(&mut emitter, output.as_mut_ptr(), output.len(), &mut written);
        yaml_emitter_set_encoding(&mut emitter, yaml::yaml_encoding_t::YAML_UTF8_ENCODING);
        yaml_emitter_set_indent(&mut emitter, 1);
        assert_eq!(emitter.best_indent, 2);
        yaml_emitter_set_indent(&mut emitter, 4);
        assert_eq!(emitter.best_indent, 4);
        yaml_emitter_set_width(&mut emitter, -7);
        assert_eq!(emitter.best_width, -1);
        yaml_emitter_set_width(&mut emitter, 24);
        assert_eq!(emitter.best_width, 24);
        yaml_emitter_set_unicode(&mut emitter, 9);
        assert_eq!(emitter.unicode, 1);
        yaml_emitter_set_break(&mut emitter, yaml::yaml_break_t::YAML_CRLN_BREAK);
        assert_eq!(emitter.line_break, yaml::yaml_break_t::YAML_CRLN_BREAK);

        assert_eq!(emit_scalar_mapping_document(&mut emitter, b"hello"), 1);
        assert_eq!(yaml_emitter_flush(&mut emitter), 1);
        yaml_emitter_delete(&mut emitter);

        let emitted = &output[..written];
        assert!(emitted.windows(2).any(|pair| pair == b"\r\n"));
        assert!(String::from_utf8_lossy(emitted).contains("message: hello"));
        assert_output_contains_scalar(emitted, b"hello");
    }
}

#[test]
fn emitter_callback_output_supports_utf16le_recode_and_flush() {
    unsafe {
        let mut emitter = mem::zeroed::<yaml_emitter_t>();
        let mut writer = CallbackWriter {
            output: Vec::new(),
            capacity: 4096,
        };

        assert_eq!(yaml_emitter_initialize(&mut emitter), 1);
        yaml_emitter_set_output(
            &mut emitter,
            Some(callback_write_handler),
            (&mut writer as *mut CallbackWriter).cast(),
        );
        yaml_emitter_set_encoding(&mut emitter, yaml::yaml_encoding_t::YAML_UTF16LE_ENCODING);
        yaml_emitter_set_width(&mut emitter, -32);
        assert_eq!(emitter.best_width, -1);
        yaml_emitter_set_unicode(&mut emitter, 1);

        assert_eq!(emit_scalar_mapping_document(&mut emitter, UTF8_GREETING), 1);
        assert_eq!(yaml_emitter_flush(&mut emitter), 1);
        yaml_emitter_delete(&mut emitter);

        assert!(writer.output.starts_with(&[0xFF, 0xFE]), "{:?}", writer.output);
        assert_output_contains_scalar(&writer.output, UTF8_GREETING);
    }
}

#[test]
fn emitter_callback_failures_report_writer_errors() {
    unsafe {
        let mut emitter = mem::zeroed::<yaml_emitter_t>();
        let mut writer = CallbackWriter {
            output: Vec::new(),
            capacity: 8,
        };

        assert_eq!(yaml_emitter_initialize(&mut emitter), 1);
        yaml_emitter_set_output(
            &mut emitter,
            Some(callback_write_handler),
            (&mut writer as *mut CallbackWriter).cast(),
        );
        assert_eq!(emit_scalar_mapping_document(&mut emitter, b"hello"), 0);
        assert_eq!(emitter.error, yaml_error_type_t::YAML_WRITER_ERROR);
        assert_eq!(CStr::from_ptr(emitter.problem), c"write error");
        yaml_emitter_delete(&mut emitter);
    }
}

#[test]
fn staged_install_runs_phase5_c_probe_and_upstream_emitter_tools() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .expect("safe crate should have a parent repository directory");
    let stage_root = temp_dir("stage-root-phase-5");
    let arch = multiarch();
    let stage_lib_dir = stage_root.join("usr/lib").join(&arch);
    let compiler = compiler();

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/stage-install.sh"))
            .arg(&stage_root),
        "stage-install",
    );

    let emitter_api_safe = temp_dir("emitter-api-exports-safe").join("emitter-api-exports-safe");
    run_command(
        Command::new(&compiler)
            .arg("-I")
            .arg(stage_root.join("usr/include"))
            .arg(manifest_dir.join("tests/fixtures/emitter_api_exports.c"))
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&emitter_api_safe),
        "compile staged emitter_api_exports.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&emitter_api_safe),
        "assert staged loader for emitter_api_exports staged-header mode",
    );
    run_command(
        &mut Command::new(&emitter_api_safe),
        "run emitter_api_exports staged-header mode",
    );

    let emitter_api_object = temp_dir("emitter-api-exports-link-safe").join("emitter-api-exports-safe.o");
    run_command(
        Command::new(&compiler)
            .arg("-c")
            .arg("-I")
            .arg(repo_root.join("original/include"))
            .arg(manifest_dir.join("tests/fixtures/emitter_api_exports.c"))
            .arg("-o")
            .arg(&emitter_api_object),
        "compile emitter_api_exports.c against original header",
    );
    let emitter_api_link = temp_dir("emitter-api-exports-link-safe").join("emitter-api-exports-link-safe");
    run_command(
        Command::new(&compiler)
            .arg(&emitter_api_object)
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&emitter_api_link),
        "link emitter_api_exports object against staged library",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&emitter_api_link),
        "assert staged loader for emitter_api_exports object-link mode",
    );
    run_command(
        &mut Command::new(&emitter_api_link),
        "run emitter_api_exports object-link mode",
    );

    let yaml_input = repo_root.join("original/examples/mapping.yaml");
    let run_emitter_binary = temp_dir("run-emitter-safe").join("run-emitter-safe");
    compile_upstream_tool(
        &compiler,
        stage_root.join("usr/include"),
        &stage_lib_dir,
        repo_root.join("original/tests/run-emitter.c"),
        &run_emitter_binary,
        "compile upstream run-emitter.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&run_emitter_binary),
        "assert staged loader for run-emitter",
    );
    let run_emitter_output = Command::new(&run_emitter_binary)
        .arg(&yaml_input)
        .output()
        .expect("failed to run upstream run-emitter");
    assert!(
        run_emitter_output.status.success(),
        "run-emitter exited with failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_emitter_output.stdout),
        String::from_utf8_lossy(&run_emitter_output.stderr)
    );
    let run_emitter_stdout =
        String::from_utf8(run_emitter_output.stdout).expect("run-emitter emitted invalid UTF-8");
    assert!(run_emitter_stdout.contains("PASSED"), "{run_emitter_stdout}");
    assert!(!run_emitter_stdout.contains("FAILED"), "{run_emitter_stdout}");

    let suite_input = temp_dir("run-emitter-suite-input").join("suite.events");
    fs::write(
        &suite_input,
        b"+STR\n+DOC ---\n+MAP\n=VAL :message\n=VAL :hello\n-MAP\n-DOC ...\n-STR\n",
    )
    .expect("failed to write emitter test-suite input");
    let run_emitter_suite_binary =
        temp_dir("run-emitter-test-suite-safe").join("run-emitter-test-suite-safe");
    compile_upstream_tool(
        &compiler,
        stage_root.join("usr/include"),
        &stage_lib_dir,
        repo_root.join("original/tests/run-emitter-test-suite.c"),
        &run_emitter_suite_binary,
        "compile upstream run-emitter-test-suite.c",
    );
    let run_emitter_suite_output = Command::new(&run_emitter_suite_binary)
        .arg(&suite_input)
        .output()
        .expect("failed to run upstream run-emitter-test-suite");
    assert!(
        run_emitter_suite_output.status.success(),
        "run-emitter-test-suite exited with failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_emitter_suite_output.stdout),
        String::from_utf8_lossy(&run_emitter_suite_output.stderr)
    );
    let run_emitter_suite_stdout = String::from_utf8(run_emitter_suite_output.stdout)
        .expect("run-emitter-test-suite emitted invalid UTF-8");
    assert!(run_emitter_suite_stdout.contains("message: hello"));

    let reformatter_binary = temp_dir("example-reformatter-safe").join("example-reformatter-safe");
    compile_upstream_tool(
        &compiler,
        stage_root.join("usr/include"),
        &stage_lib_dir,
        repo_root.join("original/tests/example-reformatter.c"),
        &reformatter_binary,
        "compile upstream example-reformatter.c",
    );
    let reformatter_output = Command::new(&reformatter_binary)
        .stdin(Stdio::from(File::open(&yaml_input).expect("failed to open YAML input")))
        .output()
        .expect("failed to run upstream example-reformatter");
    assert!(
        reformatter_output.status.success(),
        "example-reformatter exited with failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&reformatter_output.stdout),
        String::from_utf8_lossy(&reformatter_output.stderr)
    );
    let reformatter_stdout = String::from_utf8(reformatter_output.stdout)
        .expect("example-reformatter emitted invalid UTF-8");
    assert!(reformatter_stdout.contains("key: value"));

    let deconstructor_binary =
        temp_dir("example-deconstructor-safe").join("example-deconstructor-safe");
    compile_upstream_tool(
        &compiler,
        stage_root.join("usr/include"),
        &stage_lib_dir,
        repo_root.join("original/tests/example-deconstructor.c"),
        &deconstructor_binary,
        "compile upstream example-deconstructor.c",
    );
    let deconstructor_output = Command::new(&deconstructor_binary)
        .stdin(Stdio::from(File::open(&yaml_input).expect("failed to open YAML input")))
        .output()
        .expect("failed to run upstream example-deconstructor");
    assert!(
        deconstructor_output.status.success(),
        "example-deconstructor exited with failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&deconstructor_output.stdout),
        String::from_utf8_lossy(&deconstructor_output.stderr)
    );
    let deconstructor_stdout = String::from_utf8(deconstructor_output.stdout)
        .expect("example-deconstructor emitted invalid UTF-8");
    assert!(deconstructor_stdout.contains("STREAM-START"), "{deconstructor_stdout}");
    assert!(deconstructor_stdout.contains("SCALAR"), "{deconstructor_stdout}");
}

fn compile_upstream_tool(
    compiler: &str,
    include_dir: PathBuf,
    stage_lib_dir: &PathBuf,
    source: PathBuf,
    output: &PathBuf,
    label: &str,
) {
    run_command(
        Command::new(compiler)
            .arg("-I")
            .arg(include_dir)
            .arg(source)
            .arg("-L")
            .arg(stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(output),
        label,
    );
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
