use std::env;
use std::ffi::CStr;
use std::fs;
use std::io::Write;
use std::mem;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::ptr;
use std::slice;
use std::time::{SystemTime, UNIX_EPOCH};

use yaml::{
    yaml_alias_event_initialize, yaml_document_add_mapping, yaml_document_add_scalar,
    yaml_document_add_sequence, yaml_document_append_mapping_pair,
    yaml_document_append_sequence_item, yaml_document_delete, yaml_document_end_event_initialize,
    yaml_document_get_node, yaml_document_get_root_node, yaml_document_initialize,
    yaml_document_start_event_initialize, yaml_document_t, yaml_emitter_close, yaml_emitter_delete,
    yaml_emitter_dump, yaml_emitter_emit, yaml_emitter_flush, yaml_emitter_initialize,
    yaml_emitter_open, yaml_emitter_set_break, yaml_emitter_set_encoding, yaml_emitter_set_indent,
    yaml_emitter_set_output, yaml_emitter_set_output_string, yaml_emitter_set_unicode,
    yaml_emitter_set_width, yaml_emitter_t, yaml_event_delete, yaml_event_t,
    yaml_mapping_end_event_initialize, yaml_mapping_start_event_initialize, yaml_mapping_style_t,
    yaml_node_t, yaml_node_type_t, yaml_parser_delete, yaml_parser_initialize, yaml_parser_load,
    yaml_parser_parse, yaml_parser_set_encoding, yaml_parser_set_input,
    yaml_parser_set_input_string, yaml_parser_t, yaml_scalar_event_initialize, yaml_scalar_style_t,
    yaml_sequence_end_event_initialize, yaml_sequence_start_event_initialize,
    yaml_sequence_style_t, yaml_stream_end_event_initialize, yaml_stream_start_event_initialize,
    yaml_tag_directive_t, yaml_version_directive_t,
};

macro_rules! cstr {
    ($value:literal) => {
        unsafe { ::std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($value, "\0").as_bytes()) }
    };
}

const BUFFER_SIZE: usize = 8192;
const EXAMPLE_TAG_PREFIX: &[u8] = b"tag:example.com,2026:\0";
const EXAMPLE_TEXT_TAG: &[u8] = b"tag:example.com,2026:text\0";
const UTF8_GREETING: &[u8] = b"Hi is \xD0\x9F\xD1\x80\xD0\xB8\xD0\xB2\xD0\xB5\xD1\x82";
const UTF16LE_GREETING_NO_BOM: &[u8] = &[
    b'H', 0x00, b'i', 0x00, b' ', 0x00, b'i', 0x00, b's', 0x00, b' ', 0x00, 0x1f, 0x04, 0x40, 0x04,
    0x38, 0x04, 0x32, 0x04, 0x35, 0x04, 0x42, 0x04,
];

#[repr(C)]
struct MemoryReader {
    input: *const u8,
    size: usize,
    offset: usize,
    chunk: usize,
}

#[repr(C)]
struct MemoryWriter {
    output: *mut u8,
    capacity: usize,
    written: usize,
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

    let limit = if reader.chunk == 0 {
        size
    } else {
        size.min(reader.chunk)
    };
    let count = limit.min(remaining);
    ptr::copy_nonoverlapping(reader.input.add(reader.offset), buffer, count);
    reader.offset += count;
    *size_read = count;
    1
}

unsafe extern "C" fn memory_write_handler(
    data: *mut core::ffi::c_void,
    buffer: *mut u8,
    size: usize,
) -> i32 {
    let writer = &mut *data.cast::<MemoryWriter>();
    if writer.written.saturating_add(size) >= writer.capacity {
        return 0;
    }

    ptr::copy_nonoverlapping(buffer, writer.output.add(writer.written), size);
    writer.written += size;
    *writer.output.add(writer.written) = 0;
    1
}

#[test]
fn encoding_controls_match_upstream_test_api() {
    unsafe {
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut emitter = mem::zeroed::<yaml_emitter_t>();
        let mut parsed = mem::zeroed::<yaml_document_t>();
        let mut emitted = mem::zeroed::<yaml_document_t>();
        let mut reparsed = mem::zeroed::<yaml_document_t>();
        let mut output = [0u8; BUFFER_SIZE];
        let mut written = 0usize;

        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(
            &mut parser,
            UTF16LE_GREETING_NO_BOM.as_ptr(),
            UTF16LE_GREETING_NO_BOM.len(),
        );
        yaml_parser_set_encoding(&mut parser, yaml::yaml_encoding_t::YAML_UTF16LE_ENCODING);
        assert_eq!(
            parser.encoding,
            yaml::yaml_encoding_t::YAML_UTF16LE_ENCODING
        );
        assert_eq!(yaml_parser_load(&mut parser, &mut parsed), 1);
        assert_scalar_node(
            yaml_document_get_root_node(&mut parsed),
            cstr!("tag:yaml.org,2002:str"),
            UTF8_GREETING,
        );
        yaml_document_delete(&mut parsed);
        yaml_parser_delete(&mut parser);

        build_scalar_document(&mut emitted, UTF8_GREETING);
        assert_eq!(yaml_emitter_initialize(&mut emitter), 1);
        yaml_emitter_set_output_string(
            &mut emitter,
            output.as_mut_ptr(),
            output.len(),
            &mut written,
        );
        yaml_emitter_set_encoding(&mut emitter, yaml::yaml_encoding_t::YAML_UTF16LE_ENCODING);
        assert_eq!(
            emitter.encoding,
            yaml::yaml_encoding_t::YAML_UTF16LE_ENCODING
        );
        assert_eq!(yaml_emitter_open(&mut emitter), 1);
        assert_eq!(yaml_emitter_dump(&mut emitter, &mut emitted), 1);
        assert_eq!(yaml_emitter_close(&mut emitter), 1);
        assert_eq!(yaml_emitter_flush(&mut emitter), 1);
        yaml_emitter_delete(&mut emitter);
        assert!(written > 2);
        assert_eq!(&output[..2], &[0xFF, 0xFE]);

        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(&mut parser, output.as_ptr(), written);
        assert_eq!(yaml_parser_load(&mut parser, &mut reparsed), 1);
        assert_scalar_node(
            yaml_document_get_root_node(&mut reparsed),
            cstr!("tag:yaml.org,2002:str"),
            UTF8_GREETING,
        );
        yaml_document_delete(&mut reparsed);
        yaml_parser_delete(&mut parser);
    }
}

#[test]
fn document_api_roundtrip_matches_upstream_test_api() {
    unsafe {
        let mut emitted = mem::zeroed::<yaml_document_t>();
        let mut actual = mem::zeroed::<yaml_document_t>();
        let mut expected = mem::zeroed::<yaml_document_t>();
        let mut end = mem::zeroed::<yaml_document_t>();
        let mut emitter = mem::zeroed::<yaml_emitter_t>();
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut output = [0u8; BUFFER_SIZE];
        let mut written = 0usize;
        let mut reader = MemoryReader {
            input: output.as_ptr(),
            size: 0,
            offset: 0,
            chunk: 5,
        };

        build_roundtrip_document(&mut emitted);
        assert_eq!(yaml_emitter_initialize(&mut emitter), 1);
        yaml_emitter_set_output_string(
            &mut emitter,
            output.as_mut_ptr(),
            output.len(),
            &mut written,
        );
        yaml_emitter_set_indent(&mut emitter, 3);
        yaml_emitter_set_width(&mut emitter, 48);
        yaml_emitter_set_unicode(&mut emitter, 1);
        yaml_emitter_set_break(&mut emitter, yaml::yaml_break_t::YAML_CRLN_BREAK);
        assert_eq!(emitter.best_indent, 3);
        assert_eq!(emitter.best_width, 48);
        assert_eq!(emitter.unicode, 1);
        assert_eq!(emitter.line_break, yaml::yaml_break_t::YAML_CRLN_BREAK);
        assert_eq!(yaml_emitter_open(&mut emitter), 1);
        assert_eq!(yaml_emitter_dump(&mut emitter, &mut emitted), 1);
        assert_eq!(yaml_emitter_close(&mut emitter), 1);
        assert_eq!(yaml_emitter_flush(&mut emitter), 1);
        yaml_emitter_delete(&mut emitter);

        let emitted_text = String::from_utf8_lossy(&output[..written]);
        assert!(
            buffer_contains(&output[..written], b"%YAML 1.1\r\n"),
            "{emitted_text}"
        );
        assert!(
            buffer_contains(&output[..written], b"%TAG !e! tag:example.com,2026:\r\n"),
            "{emitted_text}"
        );
        assert!(
            buffer_contains(&output[..written], b"message: "),
            "{emitted_text}"
        );
        assert!(
            buffer_contains(&output[..written], b"meta:"),
            "{emitted_text}"
        );
        assert!(
            buffer_contains(&output[..written], b"count"),
            "{emitted_text}"
        );
        assert_crlf_line_breaks(&output[..written]);

        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        reader.size = written;
        reader.offset = 0;
        yaml_parser_set_input(
            &mut parser,
            Some(memory_read_handler),
            (&mut reader as *mut MemoryReader).cast(),
        );
        assert_eq!(yaml_parser_load(&mut parser, &mut actual), 1);
        assert_ne!(reader.offset, 0);

        build_roundtrip_document(&mut expected);
        assert!(compare_documents(&expected, &actual));
        assert!(yaml_document_get_node(&mut actual, 999).is_null());

        let root = yaml_document_get_root_node(&mut actual);
        let items = lookup_mapping_value(&mut actual, root, b"items").expect("items");
        let meta = lookup_mapping_value(&mut actual, root, b"meta").expect("meta");
        assert_eq!((*items).r#type, yaml_node_type_t::YAML_SEQUENCE_NODE);
        assert_eq!(
            (*items)
                .data
                .sequence
                .items
                .top
                .offset_from((*items).data.sequence.items.start),
            2
        );
        assert_eq!((*meta).r#type, yaml_node_type_t::YAML_MAPPING_NODE);
        assert!(lookup_mapping_value(&mut actual, root, b"missing").is_none());

        yaml_document_delete(&mut expected);
        yaml_document_delete(&mut actual);
        assert_eq!(yaml_parser_load(&mut parser, &mut end), 1);
        assert!(yaml_document_get_root_node(&mut end).is_null());
        yaml_document_delete(&mut end);
        yaml_parser_delete(&mut parser);
    }
}

#[test]
fn event_api_roundtrip_matches_upstream_test_api() {
    unsafe {
        let mut emitter = mem::zeroed::<yaml_emitter_t>();
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut event = mem::zeroed::<yaml_event_t>();
        let mut output = [0u8; BUFFER_SIZE];
        let mut writer = MemoryWriter {
            output: output.as_mut_ptr(),
            capacity: output.len(),
            written: 0,
        };
        let mut version = yaml_version_directive_t { major: 1, minor: 2 };
        let mut tags = [yaml_tag_directive_t {
            handle: cstr!("!e!").as_ptr().cast::<u8>().cast_mut(),
            prefix: EXAMPLE_TAG_PREFIX.as_ptr().cast_mut(),
        }];

        assert_eq!(yaml_emitter_initialize(&mut emitter), 1);
        yaml_emitter_set_output(
            &mut emitter,
            Some(memory_write_handler),
            (&mut writer as *mut MemoryWriter).cast(),
        );
        yaml_emitter_set_width(&mut emitter, -1);
        yaml_emitter_set_break(&mut emitter, yaml::yaml_break_t::YAML_LN_BREAK);
        yaml_emitter_set_unicode(&mut emitter, 1);
        assert_eq!(emitter.best_width, -1);
        assert_eq!(emitter.line_break, yaml::yaml_break_t::YAML_LN_BREAK);
        assert_eq!(emitter.unicode, 1);

        emit_ok(
            &mut emitter,
            yaml_stream_start_event_initialize(
                &mut event,
                yaml::yaml_encoding_t::YAML_UTF8_ENCODING,
            ),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_document_start_event_initialize(
                &mut event,
                &mut version,
                tags.as_mut_ptr(),
                tags.as_mut_ptr().add(1),
                0,
            ),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_mapping_start_event_initialize(
                &mut event,
                cstr!("root").as_ptr().cast(),
                cstr!("tag:yaml.org,2002:map").as_ptr().cast(),
                1,
                yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE,
            ),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_scalar_event_initialize(
                &mut event,
                ptr::null(),
                ptr::null(),
                cstr!("shared").as_ptr().cast(),
                -1,
                1,
                1,
                yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
            ),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_scalar_event_initialize(
                &mut event,
                cstr!("item").as_ptr().cast(),
                EXAMPLE_TEXT_TAG.as_ptr(),
                cstr!("value").as_ptr().cast(),
                -1,
                0,
                0,
                yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
            ),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_scalar_event_initialize(
                &mut event,
                ptr::null(),
                ptr::null(),
                cstr!("alias").as_ptr().cast(),
                -1,
                1,
                1,
                yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
            ),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_alias_event_initialize(&mut event, cstr!("item").as_ptr().cast()),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_scalar_event_initialize(
                &mut event,
                ptr::null(),
                ptr::null(),
                cstr!("seq").as_ptr().cast(),
                -1,
                1,
                1,
                yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
            ),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_sequence_start_event_initialize(
                &mut event,
                ptr::null(),
                cstr!("tag:yaml.org,2002:seq").as_ptr().cast(),
                1,
                yaml_sequence_style_t::YAML_FLOW_SEQUENCE_STYLE,
            ),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_scalar_event_initialize(
                &mut event,
                ptr::null(),
                ptr::null(),
                cstr!("a").as_ptr().cast(),
                -1,
                1,
                1,
                yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
            ),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_scalar_event_initialize(
                &mut event,
                ptr::null(),
                ptr::null(),
                cstr!("b").as_ptr().cast(),
                -1,
                1,
                1,
                yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
            ),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_sequence_end_event_initialize(&mut event),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_mapping_end_event_initialize(&mut event),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_document_end_event_initialize(&mut event, 0),
            &mut event,
        );
        emit_ok(
            &mut emitter,
            yaml_stream_end_event_initialize(&mut event),
            &mut event,
        );
        assert_eq!(yaml_emitter_flush(&mut emitter), 1);
        yaml_emitter_delete(&mut emitter);

        assert!(buffer_contains(&output[..writer.written], b"%YAML 1.2\n"));
        assert!(buffer_contains(
            &output[..writer.written],
            b"%TAG !e! tag:example.com,2026:\n"
        ));
        assert!(buffer_contains(&output[..writer.written], b"&item"));
        assert!(buffer_contains(&output[..writer.written], b"*item"));
        assert!(buffer_contains(&output[..writer.written], b"[a, b]"));

        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(&mut parser, output.as_ptr(), writer.written);

        parse_ok(&mut parser, &mut event);
        assert_eq!(
            event.r#type,
            yaml::yaml_event_type_t::YAML_STREAM_START_EVENT
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(
            event.r#type,
            yaml::yaml_event_type_t::YAML_DOCUMENT_START_EVENT
        );
        assert_eq!((*event.data.document_start.version_directive).major, 1);
        assert_eq!((*event.data.document_start.version_directive).minor, 2);
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
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(
            event.r#type,
            yaml::yaml_event_type_t::YAML_MAPPING_START_EVENT
        );
        assert_eq!(
            CStr::from_ptr(event.data.mapping_start.anchor.cast()),
            cstr!("root")
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml::yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_eq!(scalar_bytes(&event), b"shared");
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml::yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_eq!(
            CStr::from_ptr(event.data.scalar.anchor.cast()),
            cstr!("item")
        );
        assert_eq!(
            CStr::from_ptr(event.data.scalar.tag.cast()),
            cstr!("tag:example.com,2026:text")
        );
        assert_eq!(scalar_bytes(&event), b"value");
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml::yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_eq!(scalar_bytes(&event), b"alias");
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml::yaml_event_type_t::YAML_ALIAS_EVENT);
        assert_eq!(
            CStr::from_ptr(event.data.alias.anchor.cast()),
            cstr!("item")
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml::yaml_event_type_t::YAML_SCALAR_EVENT);
        assert_eq!(scalar_bytes(&event), b"seq");
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(
            event.r#type,
            yaml::yaml_event_type_t::YAML_SEQUENCE_START_EVENT
        );
        assert_eq!(
            event.data.sequence_start.style,
            yaml_sequence_style_t::YAML_FLOW_SEQUENCE_STYLE
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(scalar_bytes(&event), b"a");
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(scalar_bytes(&event), b"b");
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(
            event.r#type,
            yaml::yaml_event_type_t::YAML_SEQUENCE_END_EVENT
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(
            event.r#type,
            yaml::yaml_event_type_t::YAML_MAPPING_END_EVENT
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(
            event.r#type,
            yaml::yaml_event_type_t::YAML_DOCUMENT_END_EVENT
        );
        yaml_event_delete(&mut event);

        parse_ok(&mut parser, &mut event);
        assert_eq!(event.r#type, yaml::yaml_event_type_t::YAML_STREAM_END_EVENT);
        yaml_event_delete(&mut event);
        yaml_parser_delete(&mut parser);
    }
}

#[test]
fn native_phase6_examples_run() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let run_dumper_output = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(manifest_dir.join("Cargo.toml"))
        .arg("--offline")
        .arg("--example")
        .arg("run_dumper")
        .arg(manifest_dir.join("compat/examples/anchors.yaml"))
        .arg(manifest_dir.join("compat/examples/json.yaml"))
        .output()
        .expect("failed to run Rust run_dumper example");
    assert!(
        run_dumper_output.status.success(),
        "run_dumper failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_dumper_output.stdout),
        String::from_utf8_lossy(&run_dumper_output.stderr)
    );
    let run_dumper_stdout =
        String::from_utf8(run_dumper_output.stdout).expect("run_dumper stdout utf8");
    assert_eq!(
        run_dumper_stdout.matches("PASSED (length:").count(),
        2,
        "{run_dumper_stdout}"
    );

    let reformatter_output = run_cargo_example_with_stdin(
        &manifest_dir,
        "example_reformatter_alt",
        b"foo: [bar, {x: y}]\n",
    );
    let reformatter_stdout =
        String::from_utf8(reformatter_output.stdout).expect("reformatter utf8");
    assert!(reformatter_stdout.contains("foo:"), "{reformatter_stdout}");
    assert!(reformatter_stdout.contains("bar"), "{reformatter_stdout}");

    let deconstructor_output =
        run_cargo_example_with_stdin(&manifest_dir, "example_deconstructor_alt", b"foo: bar\n");
    let deconstructor_stdout =
        String::from_utf8(deconstructor_output.stdout).expect("deconstructor utf8");
    assert!(
        deconstructor_stdout.contains("STREAM-START"),
        "{deconstructor_stdout}"
    );
    assert!(
        deconstructor_stdout.contains("SCALAR"),
        "{deconstructor_stdout}"
    );
}

#[test]
fn staged_install_runs_phase6_c_probes_and_tools() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stage_root = temp_dir("stage-root-phase-6");
    let arch = multiarch();
    let stage_lib_dir = stage_root.join("usr/lib").join(&arch);
    let compiler = compiler();

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/stage-install.sh"))
            .arg(&stage_root),
        "stage-install",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/verify-exported-symbols.sh"))
            .arg(&stage_root),
        "verify-exported-symbols",
    );

    let test_api_binary = temp_dir("test-api-safe").join("test-api-safe");
    compile_vendored_tool(
        &compiler,
        stage_root.join("usr/include"),
        &stage_lib_dir,
        manifest_dir.join("compat/original-tests/test-api.c"),
        &test_api_binary,
        "compile vendored test-api.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&test_api_binary),
        "assert staged loader for test-api",
    );
    run_command(&mut Command::new(&test_api_binary), "run vendored test-api");

    let run_dumper_binary = temp_dir("run-dumper-safe").join("run-dumper-safe");
    compile_vendored_tool(
        &compiler,
        stage_root.join("usr/include"),
        &stage_lib_dir,
        manifest_dir.join("compat/original-tests/run-dumper.c"),
        &run_dumper_binary,
        "compile vendored run-dumper.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&run_dumper_binary),
        "assert staged loader for run-dumper",
    );
    let run_dumper_output = Command::new(&run_dumper_binary)
        .arg(manifest_dir.join("compat/examples/anchors.yaml"))
        .arg(manifest_dir.join("compat/examples/json.yaml"))
        .output()
        .expect("failed to run vendored run-dumper");
    assert!(
        run_dumper_output.status.success(),
        "run-dumper failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_dumper_output.stdout),
        String::from_utf8_lossy(&run_dumper_output.stderr)
    );
    let run_dumper_stdout =
        String::from_utf8(run_dumper_output.stdout).expect("run-dumper stdout utf8");
    assert!(!run_dumper_stdout.contains("FAILED"), "{run_dumper_stdout}");
    assert_eq!(
        run_dumper_stdout.matches("PASSED (length:").count(),
        2,
        "{run_dumper_stdout}"
    );

    let reformatter_binary =
        temp_dir("example-reformatter-alt-safe").join("example-reformatter-alt-safe");
    compile_vendored_tool(
        &compiler,
        stage_root.join("usr/include"),
        &stage_lib_dir,
        manifest_dir.join("compat/original-tests/example-reformatter-alt.c"),
        &reformatter_binary,
        "compile vendored example-reformatter-alt.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&reformatter_binary),
        "assert staged loader for example-reformatter-alt",
    );
    let mut reformatter = Command::new(&reformatter_binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn vendored example-reformatter-alt");
    reformatter
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(b"foo: [bar, baz]\n")
        .expect("failed to write reformatter input");
    let reformatter_output = reformatter
        .wait_with_output()
        .expect("failed to collect reformatter output");
    assert!(
        reformatter_output.status.success(),
        "example-reformatter-alt failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&reformatter_output.stdout),
        String::from_utf8_lossy(&reformatter_output.stderr)
    );
    let reformatter_stdout =
        String::from_utf8(reformatter_output.stdout).expect("reformatter stdout utf8");
    assert!(reformatter_stdout.contains("foo:"), "{reformatter_stdout}");
    assert!(reformatter_stdout.contains("baz"), "{reformatter_stdout}");

    let deconstructor_binary =
        temp_dir("example-deconstructor-alt-safe").join("example-deconstructor-alt-safe");
    compile_vendored_tool(
        &compiler,
        stage_root.join("usr/include"),
        &stage_lib_dir,
        manifest_dir.join("compat/original-tests/example-deconstructor-alt.c"),
        &deconstructor_binary,
        "compile vendored example-deconstructor-alt.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&deconstructor_binary),
        "assert staged loader for example-deconstructor-alt",
    );
    let mut deconstructor = Command::new(&deconstructor_binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn vendored example-deconstructor-alt");
    deconstructor
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(b"foo: bar\n")
        .expect("failed to write deconstructor input");
    let deconstructor_output = deconstructor
        .wait_with_output()
        .expect("failed to collect deconstructor output");
    assert!(
        deconstructor_output.status.success(),
        "example-deconstructor-alt failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&deconstructor_output.stdout),
        String::from_utf8_lossy(&deconstructor_output.stderr)
    );
    let deconstructor_stdout =
        String::from_utf8(deconstructor_output.stdout).expect("deconstructor stdout utf8");
    assert!(
        deconstructor_stdout.contains("STREAM-START"),
        "{deconstructor_stdout}"
    );
    assert!(
        deconstructor_stdout.contains("STREAM-END"),
        "{deconstructor_stdout}"
    );
}

unsafe fn build_scalar_document(document: *mut yaml_document_t, value: &[u8]) {
    assert_eq!(
        yaml_document_initialize(
            document,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            1,
            1
        ),
        1
    );
    assert_eq!(
        yaml_document_add_scalar(
            document,
            cstr!("tag:yaml.org,2002:str").as_ptr().cast(),
            value.as_ptr(),
            value.len() as i32,
            yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
        ),
        1
    );
}

unsafe fn build_roundtrip_document(document: *mut yaml_document_t) {
    let mut version = yaml_version_directive_t { major: 1, minor: 1 };
    let mut handle = b"!e!\0".to_vec();
    let mut prefix = EXAMPLE_TAG_PREFIX.to_vec();
    let mut tags = [yaml_tag_directive_t {
        handle: handle.as_mut_ptr(),
        prefix: prefix.as_mut_ptr(),
    }];

    assert_eq!(
        yaml_document_initialize(
            document,
            &mut version,
            tags.as_mut_ptr(),
            tags.as_mut_ptr().add(1),
            0,
            0,
        ),
        1
    );

    let root = yaml_document_add_mapping(
        document,
        cstr!("tag:yaml.org,2002:map").as_ptr().cast(),
        yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE,
    );
    let message_key = yaml_document_add_scalar(
        document,
        cstr!("tag:yaml.org,2002:str").as_ptr().cast(),
        cstr!("message").as_ptr().cast(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let message_value = yaml_document_add_scalar(
        document,
        EXAMPLE_TEXT_TAG.as_ptr(),
        UTF8_GREETING.as_ptr(),
        UTF8_GREETING.len() as i32,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    assert_eq!(
        yaml_document_append_mapping_pair(document, root, message_key, message_value),
        1
    );

    let items_key = yaml_document_add_scalar(
        document,
        cstr!("tag:yaml.org,2002:str").as_ptr().cast(),
        cstr!("items").as_ptr().cast(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let items_value = yaml_document_add_sequence(
        document,
        cstr!("tag:yaml.org,2002:seq").as_ptr().cast(),
        yaml_sequence_style_t::YAML_BLOCK_SEQUENCE_STYLE,
    );
    let first_item = yaml_document_add_scalar(
        document,
        cstr!("tag:yaml.org,2002:str").as_ptr().cast(),
        cstr!("one").as_ptr().cast(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let second_item = yaml_document_add_scalar(
        document,
        cstr!("tag:yaml.org,2002:str").as_ptr().cast(),
        cstr!("two").as_ptr().cast(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    assert_eq!(
        yaml_document_append_sequence_item(document, items_value, first_item),
        1
    );
    assert_eq!(
        yaml_document_append_sequence_item(document, items_value, second_item),
        1
    );
    assert_eq!(
        yaml_document_append_mapping_pair(document, root, items_key, items_value),
        1
    );

    let meta_key = yaml_document_add_scalar(
        document,
        cstr!("tag:yaml.org,2002:str").as_ptr().cast(),
        cstr!("meta").as_ptr().cast(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let meta_value = yaml_document_add_mapping(
        document,
        cstr!("tag:yaml.org,2002:map").as_ptr().cast(),
        yaml_mapping_style_t::YAML_FLOW_MAPPING_STYLE,
    );
    let count_key = yaml_document_add_scalar(
        document,
        cstr!("tag:yaml.org,2002:str").as_ptr().cast(),
        cstr!("count").as_ptr().cast(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let count_value = yaml_document_add_scalar(
        document,
        cstr!("tag:yaml.org,2002:int").as_ptr().cast(),
        cstr!("2").as_ptr().cast(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    assert_eq!(
        yaml_document_append_mapping_pair(document, meta_value, count_key, count_value),
        1
    );
    assert_eq!(
        yaml_document_append_mapping_pair(document, root, meta_key, meta_value),
        1
    );
}

unsafe fn assert_scalar_node(node: *mut yaml_node_t, tag: &CStr, value: &[u8]) {
    assert!(!node.is_null());
    assert_eq!((*node).r#type, yaml_node_type_t::YAML_SCALAR_NODE);
    assert_eq!(CStr::from_ptr((*node).tag.cast()), tag);
    assert_eq!((*node).data.scalar.length, value.len());
    assert_eq!(
        slice::from_raw_parts((*node).data.scalar.value, value.len()),
        value
    );
}

unsafe fn lookup_mapping_value(
    document: *mut yaml_document_t,
    mapping: *mut yaml_node_t,
    key: &[u8],
) -> Option<*mut yaml_node_t> {
    let mut pair = (*mapping).data.mapping.pairs.start;
    while pair < (*mapping).data.mapping.pairs.top {
        let key_node = yaml_document_get_node(document, (*pair).key);
        if !key_node.is_null()
            && (*key_node).r#type == yaml_node_type_t::YAML_SCALAR_NODE
            && slice::from_raw_parts(
                (*key_node).data.scalar.value,
                (*key_node).data.scalar.length,
            ) == key
        {
            return Some(yaml_document_get_node(document, (*pair).value));
        }
        pair = pair.add(1);
    }
    None
}

unsafe fn compare_documents(lhs: &yaml_document_t, rhs: &yaml_document_t) -> bool {
    if lhs.start_implicit != rhs.start_implicit || lhs.end_implicit != rhs.end_implicit {
        return false;
    }

    if !same_optional_version(lhs.version_directive, rhs.version_directive) {
        return false;
    }

    let lhs_tags = lhs.tag_directives.end.offset_from(lhs.tag_directives.start);
    let rhs_tags = rhs.tag_directives.end.offset_from(rhs.tag_directives.start);
    if lhs_tags != rhs_tags {
        return false;
    }
    for index in 0..lhs_tags {
        let lhs_tag = *lhs.tag_directives.start.offset(index);
        let rhs_tag = *rhs.tag_directives.start.offset(index);
        if CStr::from_ptr(lhs_tag.handle.cast()) != CStr::from_ptr(rhs_tag.handle.cast())
            || CStr::from_ptr(lhs_tag.prefix.cast()) != CStr::from_ptr(rhs_tag.prefix.cast())
        {
            return false;
        }
    }

    let lhs_nodes = lhs.nodes.top.offset_from(lhs.nodes.start);
    let rhs_nodes = rhs.nodes.top.offset_from(rhs.nodes.start);
    lhs_nodes == rhs_nodes
        && (lhs_nodes == 0
            || compare_nodes(
                lhs as *const _ as *mut _,
                1,
                rhs as *const _ as *mut _,
                1,
                0,
            ))
}

unsafe fn compare_nodes(
    lhs_document: *mut yaml_document_t,
    lhs_index: i32,
    rhs_document: *mut yaml_document_t,
    rhs_index: i32,
    level: i32,
) -> bool {
    if level > 1000 {
        return false;
    }

    let lhs = yaml_document_get_node(lhs_document, lhs_index);
    let rhs = yaml_document_get_node(rhs_document, rhs_index);
    if lhs.is_null() || rhs.is_null() || (*lhs).r#type != (*rhs).r#type {
        return false;
    }
    if CStr::from_ptr((*lhs).tag.cast()) != CStr::from_ptr((*rhs).tag.cast()) {
        return false;
    }

    match (*lhs).r#type {
        yaml_node_type_t::YAML_SCALAR_NODE => {
            (*lhs).data.scalar.length == (*rhs).data.scalar.length
                && slice::from_raw_parts((*lhs).data.scalar.value, (*lhs).data.scalar.length)
                    == slice::from_raw_parts((*rhs).data.scalar.value, (*rhs).data.scalar.length)
        }
        yaml_node_type_t::YAML_SEQUENCE_NODE => {
            let lhs_len = (*lhs)
                .data
                .sequence
                .items
                .top
                .offset_from((*lhs).data.sequence.items.start);
            let rhs_len = (*rhs)
                .data
                .sequence
                .items
                .top
                .offset_from((*rhs).data.sequence.items.start);
            if lhs_len != rhs_len {
                return false;
            }
            for index in 0..lhs_len {
                if !compare_nodes(
                    lhs_document,
                    *(*lhs).data.sequence.items.start.offset(index),
                    rhs_document,
                    *(*rhs).data.sequence.items.start.offset(index),
                    level + 1,
                ) {
                    return false;
                }
            }
            true
        }
        yaml_node_type_t::YAML_MAPPING_NODE => {
            let lhs_len = (*lhs)
                .data
                .mapping
                .pairs
                .top
                .offset_from((*lhs).data.mapping.pairs.start);
            let rhs_len = (*rhs)
                .data
                .mapping
                .pairs
                .top
                .offset_from((*rhs).data.mapping.pairs.start);
            if lhs_len != rhs_len {
                return false;
            }
            for index in 0..lhs_len {
                let lhs_pair = *(*lhs).data.mapping.pairs.start.offset(index);
                let rhs_pair = *(*rhs).data.mapping.pairs.start.offset(index);
                if !compare_nodes(
                    lhs_document,
                    lhs_pair.key,
                    rhs_document,
                    rhs_pair.key,
                    level + 1,
                ) || !compare_nodes(
                    lhs_document,
                    lhs_pair.value,
                    rhs_document,
                    rhs_pair.value,
                    level + 1,
                ) {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

unsafe fn same_optional_version(
    lhs: *mut yaml::yaml_version_directive_t,
    rhs: *mut yaml::yaml_version_directive_t,
) -> bool {
    match (lhs.is_null(), rhs.is_null()) {
        (true, true) => true,
        (false, false) => (*lhs).major == (*rhs).major && (*lhs).minor == (*rhs).minor,
        _ => false,
    }
}

unsafe fn emit_ok(emitter: *mut yaml_emitter_t, init_ok: i32, event: *mut yaml_event_t) {
    assert_eq!(init_ok, 1);
    let result = yaml_emitter_emit(emitter, event);
    let problem = if (*emitter).problem.is_null() {
        "<null>".to_owned()
    } else {
        CStr::from_ptr((*emitter).problem)
            .to_string_lossy()
            .into_owned()
    };
    assert_eq!(
        result,
        1,
        "event={:?} error={:?} problem={problem}",
        (*event).r#type,
        (*emitter).error
    );
}

unsafe fn parse_ok(parser: *mut yaml_parser_t, event: *mut yaml_event_t) {
    assert_eq!(yaml_parser_parse(parser, event), 1, "{:?}", (*parser).error);
}

unsafe fn scalar_bytes<'a>(event: &'a yaml_event_t) -> &'a [u8] {
    slice::from_raw_parts(event.data.scalar.value, event.data.scalar.length)
}

fn buffer_contains(buffer: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && buffer.windows(needle.len()).any(|window| window == needle)
}

fn assert_crlf_line_breaks(buffer: &[u8]) {
    for (index, value) in buffer.iter().enumerate() {
        if *value == b'\n' {
            assert!(index > 0);
            assert_eq!(buffer[index - 1], b'\r');
        }
    }
}

fn run_cargo_example_with_stdin(
    manifest_dir: &PathBuf,
    example: &str,
    stdin_bytes: &[u8],
) -> std::process::Output {
    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("--manifest-path")
        .arg(manifest_dir.join("Cargo.toml"))
        .arg("--offline")
        .arg("--example")
        .arg(example)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("failed to spawn cargo example");
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(stdin_bytes)
        .expect("failed to write example input");
    let output = child
        .wait_with_output()
        .expect("failed to collect example output");
    assert!(
        output.status.success(),
        "{example} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn compile_vendored_tool(
    compiler: &str,
    include_dir: PathBuf,
    stage_lib_dir: &PathBuf,
    source: PathBuf,
    output: &PathBuf,
    label: &str,
) {
    run_command(
        Command::new(compiler)
            .arg("-std=c11")
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
        if let Ok(output) = Command::new(candidate).arg("-print-multiarch").output() {
            if output.status.success() {
                let arch = String::from_utf8_lossy(&output.stdout).trim().to_owned();
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
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}
