use std::env;
use std::ffi::CStr;
use std::fs;
use std::io::Write;
use std::mem;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::slice;
use std::time::{SystemTime, UNIX_EPOCH};

use yaml::{
    yaml_alias_event_initialize, yaml_document_end_event_initialize,
    yaml_document_start_event_initialize, yaml_event_delete, yaml_event_t, yaml_event_type_t,
    yaml_mapping_end_event_initialize, yaml_mapping_start_event_initialize, yaml_mapping_style_t,
    yaml_parser_delete, yaml_parser_initialize, yaml_parser_parse, yaml_parser_set_input_string,
    yaml_parser_t, yaml_scalar_event_initialize, yaml_scalar_style_t,
    yaml_sequence_end_event_initialize, yaml_sequence_start_event_initialize,
    yaml_sequence_style_t, yaml_stream_end_event_initialize, yaml_stream_start_event_initialize,
    yaml_tag_directive_t, yaml_version_directive_t,
};

macro_rules! cstr {
    ($value:literal) => {
        unsafe { ::std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($value, "\0").as_bytes()) }
    };
}

#[test]
fn event_initializers_deep_copy_and_delete_zeroes_memory() {
    unsafe {
        let mut event = mem::zeroed::<yaml_event_t>();

        assert_eq!(
            yaml_stream_start_event_initialize(
                &mut event,
                yaml::yaml_encoding_t::YAML_UTF8_ENCODING
            ),
            1
        );
        assert_eq!(event.r#type, yaml_event_type_t::YAML_STREAM_START_EVENT);
        assert_eq!(
            event.data.stream_start.encoding,
            yaml::yaml_encoding_t::YAML_UTF8_ENCODING
        );
        yaml_event_delete(&mut event);
        assert_zeroed_event(&event);

        let mut version = yaml_version_directive_t { major: 1, minor: 2 };
        let mut handle = b"!e!\0".to_vec();
        let mut prefix = b"tag:example.com,2026:\0".to_vec();
        let mut tags = [yaml_tag_directive_t {
            handle: handle.as_mut_ptr(),
            prefix: prefix.as_mut_ptr(),
        }];
        assert_eq!(
            yaml_document_start_event_initialize(
                &mut event,
                &mut version,
                tags.as_mut_ptr(),
                tags.as_mut_ptr().add(1),
                0
            ),
            1
        );
        assert_eq!(event.r#type, yaml_event_type_t::YAML_DOCUMENT_START_EVENT);
        assert_ne!(
            event.data.document_start.version_directive,
            &mut version as *mut yaml_version_directive_t
        );
        assert_eq!(
            event
                .data
                .document_start
                .version_directive
                .as_ref()
                .unwrap()
                .major,
            1
        );
        assert_eq!(
            event
                .data
                .document_start
                .tag_directives
                .end
                .offset_from(event.data.document_start.tag_directives.start),
            1
        );
        assert_ne!(
            event.data.document_start.tag_directives.start,
            tags.as_mut_ptr()
        );
        version.major = 9;
        handle[0] = b'X';
        prefix[0] = b'X';
        assert_eq!(
            event
                .data
                .document_start
                .version_directive
                .as_ref()
                .unwrap()
                .major,
            1
        );
        assert_eq!(
            CStr::from_ptr(
                event
                    .data
                    .document_start
                    .tag_directives
                    .start
                    .read()
                    .handle
                    .cast()
            ),
            cstr!("!e!")
        );
        assert_eq!(
            CStr::from_ptr(
                event
                    .data
                    .document_start
                    .tag_directives
                    .start
                    .read()
                    .prefix
                    .cast()
            ),
            cstr!("tag:example.com,2026:")
        );
        yaml_event_delete(&mut event);
        assert_zeroed_event(&event);

        assert_eq!(yaml_document_end_event_initialize(&mut event, 0), 1);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_DOCUMENT_END_EVENT);
        assert_eq!(event.data.document_end.implicit, 0);
        yaml_event_delete(&mut event);
        assert_zeroed_event(&event);

        let mut alias_anchor = b"item\0".to_vec();
        assert_eq!(
            yaml_alias_event_initialize(&mut event, alias_anchor.as_ptr()),
            1
        );
        assert_eq!(event.r#type, yaml_event_type_t::YAML_ALIAS_EVENT);
        assert_ne!(event.data.alias.anchor, alias_anchor.as_mut_ptr());
        alias_anchor[0] = b'X';
        assert_eq!(
            CStr::from_ptr(event.data.alias.anchor.cast()),
            cstr!("item")
        );
        yaml_event_delete(&mut event);
        assert_zeroed_event(&event);

        let mut scalar_anchor = b"item\0".to_vec();
        let mut scalar_tag = b"tag:example.com,2026:text\0".to_vec();
        let mut scalar_value = b"value\0".to_vec();
        assert_eq!(
            yaml_scalar_event_initialize(
                &mut event,
                scalar_anchor.as_ptr(),
                scalar_tag.as_ptr(),
                scalar_value.as_ptr(),
                5,
                0,
                0,
                yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE
            ),
            1
        );
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_ne!(event.data.scalar.anchor, scalar_anchor.as_mut_ptr());
        assert_ne!(event.data.scalar.tag, scalar_tag.as_mut_ptr());
        assert_ne!(event.data.scalar.value, scalar_value.as_mut_ptr());
        scalar_anchor[0] = b'X';
        scalar_tag[0] = b'X';
        scalar_value[0] = b'X';
        assert_eq!(
            CStr::from_ptr(event.data.scalar.anchor.cast()),
            cstr!("item")
        );
        assert_eq!(
            CStr::from_ptr(event.data.scalar.tag.cast()),
            cstr!("tag:example.com,2026:text")
        );
        assert_eq!(
            slice::from_raw_parts(event.data.scalar.value, event.data.scalar.length),
            b"value"
        );
        yaml_event_delete(&mut event);
        assert_zeroed_event(&event);

        let mut seq_anchor = b"seq\0".to_vec();
        let mut seq_tag = b"tag:yaml.org,2002:seq\0".to_vec();
        assert_eq!(
            yaml_sequence_start_event_initialize(
                &mut event,
                seq_anchor.as_ptr(),
                seq_tag.as_ptr(),
                1,
                yaml_sequence_style_t::YAML_FLOW_SEQUENCE_STYLE
            ),
            1
        );
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SEQUENCE_START_EVENT);
        assert_ne!(event.data.sequence_start.anchor, seq_anchor.as_mut_ptr());
        assert_ne!(event.data.sequence_start.tag, seq_tag.as_mut_ptr());
        yaml_event_delete(&mut event);
        assert_zeroed_event(&event);

        assert_eq!(yaml_sequence_end_event_initialize(&mut event), 1);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SEQUENCE_END_EVENT);
        yaml_event_delete(&mut event);
        assert_zeroed_event(&event);

        let mut map_anchor = b"map\0".to_vec();
        let mut map_tag = b"tag:yaml.org,2002:map\0".to_vec();
        assert_eq!(
            yaml_mapping_start_event_initialize(
                &mut event,
                map_anchor.as_ptr(),
                map_tag.as_ptr(),
                1,
                yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE
            ),
            1
        );
        assert_eq!(event.r#type, yaml_event_type_t::YAML_MAPPING_START_EVENT);
        assert_ne!(event.data.mapping_start.anchor, map_anchor.as_mut_ptr());
        assert_ne!(event.data.mapping_start.tag, map_tag.as_mut_ptr());
        yaml_event_delete(&mut event);
        assert_zeroed_event(&event);

        assert_eq!(yaml_mapping_end_event_initialize(&mut event), 1);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_MAPPING_END_EVENT);
        yaml_event_delete(&mut event);
        assert_zeroed_event(&event);

        assert_eq!(yaml_stream_end_event_initialize(&mut event), 1);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_STREAM_END_EVENT);
        yaml_event_delete(&mut event);
        assert_zeroed_event(&event);
    }
}

#[test]
fn parser_parse_matches_expected_event_semantics() {
    let input = b"%YAML 1.2\n%TAG !e! tag:example.com,2026:\n---\nroot: &item !e!text \"value\"\nalias: *item\nseq: [a, b]\n...\n";

    unsafe {
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut event = mem::zeroed::<yaml_event_t>();
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_STREAM_START_EVENT);
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_DOCUMENT_START_EVENT);
        assert_eq!(event.data.document_start.implicit, 0);
        assert_eq!(
            event
                .data
                .document_start
                .version_directive
                .as_ref()
                .unwrap()
                .major,
            1
        );
        assert_eq!(
            event
                .data
                .document_start
                .version_directive
                .as_ref()
                .unwrap()
                .minor,
            2
        );
        assert_eq!(
            event
                .data
                .document_start
                .tag_directives
                .end
                .offset_from(event.data.document_start.tag_directives.start),
            1
        );
        assert_eq!(
            CStr::from_ptr(
                event
                    .data
                    .document_start
                    .tag_directives
                    .start
                    .read()
                    .handle
                    .cast()
            ),
            cstr!("!e!")
        );
        assert_eq!(
            CStr::from_ptr(
                event
                    .data
                    .document_start
                    .tag_directives
                    .start
                    .read()
                    .prefix
                    .cast()
            ),
            cstr!("tag:example.com,2026:")
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_MAPPING_START_EVENT);
        assert_eq!(
            event.data.mapping_start.style,
            yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_eq!(
            slice::from_raw_parts(event.data.scalar.value, event.data.scalar.length),
            b"root"
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_eq!(
            CStr::from_ptr(event.data.scalar.anchor.cast()),
            cstr!("item")
        );
        assert_eq!(
            CStr::from_ptr(event.data.scalar.tag.cast()),
            cstr!("tag:example.com,2026:text")
        );
        assert_eq!(
            slice::from_raw_parts(event.data.scalar.value, event.data.scalar.length),
            b"value"
        );
        assert_eq!(
            event.data.scalar.style,
            yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE
        );
        assert_eq!(event.data.scalar.plain_implicit, 0);
        assert_eq!(event.data.scalar.quoted_implicit, 0);
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_eq!(
            slice::from_raw_parts(event.data.scalar.value, event.data.scalar.length),
            b"alias"
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_ALIAS_EVENT);
        assert_eq!(
            CStr::from_ptr(event.data.alias.anchor.cast()),
            cstr!("item")
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_eq!(
            slice::from_raw_parts(event.data.scalar.value, event.data.scalar.length),
            b"seq"
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SEQUENCE_START_EVENT);
        assert_eq!(
            event.data.sequence_start.style,
            yaml_sequence_style_t::YAML_FLOW_SEQUENCE_STYLE
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_eq!(
            slice::from_raw_parts(event.data.scalar.value, event.data.scalar.length),
            b"a"
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_eq!(
            slice::from_raw_parts(event.data.scalar.value, event.data.scalar.length),
            b"b"
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_SEQUENCE_END_EVENT);
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_MAPPING_END_EVENT);
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_DOCUMENT_END_EVENT);
        assert_eq!(event.data.document_end.implicit, 0);
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml_event_type_t::YAML_STREAM_END_EVENT);
        yaml_event_delete(&mut event);

        yaml_parser_delete(&mut parser);
    }
}

#[test]
fn parser_reports_undefined_tag_handle_with_context() {
    unsafe {
        let input = b"!e!text value\n";
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut event = mem::zeroed::<yaml_event_t>();

        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());

        parse_ok(&mut parser, &mut event);
        yaml_event_delete(&mut event);
        parse_ok(&mut parser, &mut event);
        yaml_event_delete(&mut event);

        assert_eq!(yaml_parser_parse(&mut parser, &mut event), 0);
        assert_eq!(parser.error, yaml::yaml_error_type_t::YAML_PARSER_ERROR);
        assert_eq!(
            CStr::from_ptr(parser.context),
            cstr!("while parsing a node")
        );
        assert_eq!(
            CStr::from_ptr(parser.problem),
            cstr!("found undefined tag handle")
        );
        assert_eq!(parser.problem_mark.line, 0);
        assert_eq!(parser.context_mark.line, 0);
        assert_zeroed_event(&event);

        yaml_parser_delete(&mut parser);
    }
}

#[test]
fn staged_install_runs_phase3_c_probes_and_upstream_parser_tools() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stage_root = temp_dir("stage-root-phase-3");
    let arch = multiarch();
    let stage_lib_dir = stage_root.join("usr/lib").join(&arch);
    let compiler = compiler();

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/stage-install.sh"))
            .arg(&stage_root),
        "stage-install",
    );

    let event_api_safe = temp_dir("event-api-exports-safe").join("event-api-exports-safe");
    run_command(
        Command::new(&compiler)
            .arg("-I")
            .arg(stage_root.join("usr/include"))
            .arg(manifest_dir.join("tests/fixtures/event_api_exports.c"))
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&event_api_safe),
        "compile staged event_api_exports.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&event_api_safe),
        "assert staged loader for event_api_exports staged-header mode",
    );
    run_command(
        &mut Command::new(&event_api_safe),
        "run event_api_exports staged-header mode",
    );

    let event_api_object = temp_dir("event-api-exports-link-safe").join("event-api-exports-safe.o");
    run_command(
        Command::new(&compiler)
            .arg("-c")
            .arg("-I")
            .arg(manifest_dir.join("include"))
            .arg(manifest_dir.join("tests/fixtures/event_api_exports.c"))
            .arg("-o")
            .arg(&event_api_object),
        "compile event_api_exports.c against vendored header",
    );
    let event_api_link =
        temp_dir("event-api-exports-link-safe").join("event-api-exports-link-safe");
    run_command(
        Command::new(&compiler)
            .arg(&event_api_object)
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&event_api_link),
        "link event_api_exports object against staged library",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&event_api_link),
        "assert staged loader for event_api_exports object-link mode",
    );
    run_command(
        &mut Command::new(&event_api_link),
        "run event_api_exports object-link mode",
    );

    let run_parser_binary = temp_dir("run-parser-safe").join("run-parser-safe");
    run_command(
        Command::new(&compiler)
            .arg("-I")
            .arg(stage_root.join("usr/include"))
            .arg(manifest_dir.join("compat/original-tests/run-parser.c"))
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&run_parser_binary),
        "compile upstream run-parser.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&run_parser_binary),
        "assert staged loader for run-parser",
    );
    let run_parser_output = Command::new(&run_parser_binary)
        .arg(manifest_dir.join("compat/examples/anchors.yaml"))
        .arg(manifest_dir.join("compat/examples/json.yaml"))
        .arg(manifest_dir.join("compat/examples/mapping.yaml"))
        .arg(manifest_dir.join("compat/examples/tags.yaml"))
        .output()
        .expect("failed to run upstream run-parser");
    assert!(
        run_parser_output.status.success(),
        "run-parser exited with failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_parser_output.stdout),
        String::from_utf8_lossy(&run_parser_output.stderr)
    );
    let run_parser_stdout =
        String::from_utf8(run_parser_output.stdout).expect("run-parser emitted invalid UTF-8");
    assert!(
        !run_parser_stdout.contains("FAILURE"),
        "{run_parser_stdout}"
    );
    assert_eq!(
        run_parser_stdout.matches("SUCCESS").count(),
        4,
        "{run_parser_stdout}"
    );

    let run_parser_test_suite =
        temp_dir("run-parser-test-suite-safe").join("run-parser-test-suite-safe");
    run_command(
        Command::new(&compiler)
            .arg("-I")
            .arg(stage_root.join("usr/include"))
            .arg(manifest_dir.join("compat/original-tests/run-parser-test-suite.c"))
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&run_parser_test_suite),
        "compile upstream run-parser-test-suite.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&run_parser_test_suite),
        "assert staged loader for run-parser-test-suite",
    );

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&run_parser_test_suite),
        "assert staged loader for run-parser-test-suite block input",
    );
    let mut block_command = Command::new(&run_parser_test_suite);
    let block_output = run_command_with_input(
        &mut block_command,
        b"foo: bar\n",
        "run run-parser-test-suite block input",
    );
    assert_eq!(
        String::from_utf8(block_output.stdout).expect("block stdout should be utf8"),
        "+STR\n+DOC\n+MAP\n=VAL :foo\n=VAL :bar\n-MAP\n-DOC\n-STR\n"
    );

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&run_parser_test_suite),
        "assert staged loader for run-parser-test-suite flow input",
    );
    let mut flow_command = Command::new(&run_parser_test_suite);
    flow_command.arg("--flow").arg("keep");
    let flow_output = run_command_with_input(
        &mut flow_command,
        b"{ foo: bar }\n",
        "run run-parser-test-suite flow input",
    );
    assert_eq!(
        String::from_utf8(flow_output.stdout).expect("flow stdout should be utf8"),
        "+STR\n+DOC\n+MAP {}\n=VAL :foo\n=VAL :bar\n-MAP\n-DOC\n-STR\n"
    );
}

unsafe fn parse_ok(parser: *mut yaml_parser_t, event: *mut yaml_event_t) {
    assert_eq!(
        yaml_parser_parse(parser, event),
        1,
        "parser error={:?} context={:?} problem={:?}",
        (*parser).error,
        if (*parser).context.is_null() {
            None
        } else {
            Some(
                CStr::from_ptr((*parser).context)
                    .to_string_lossy()
                    .into_owned(),
            )
        },
        if (*parser).problem.is_null() {
            None
        } else {
            Some(
                CStr::from_ptr((*parser).problem)
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    );
}

unsafe fn assert_zeroed_event(event: &yaml_event_t) {
    let bytes = slice::from_raw_parts(
        (event as *const yaml_event_t).cast::<u8>(),
        mem::size_of::<yaml_event_t>(),
    );
    assert!(
        bytes.iter().all(|value| *value == 0),
        "event was not zeroed"
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

fn run_command_with_input(
    command: &mut Command,
    input: &[u8],
    label: &str,
) -> std::process::Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn command with stdin");
    child
        .stdin
        .as_mut()
        .expect("stdin should be available")
        .write_all(input)
        .expect("failed to write stdin");
    let output = child.wait_with_output().expect("failed to collect output");
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
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
