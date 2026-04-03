use std::env;
use std::ffi::CStr;
use std::fs;
use std::mem;
use std::path::PathBuf;
use std::process::Command;
use std::ptr;
use std::slice;
use std::time::{SystemTime, UNIX_EPOCH};

use yaml::{
    yaml_document_add_mapping, yaml_document_add_scalar, yaml_document_add_sequence,
    yaml_document_append_mapping_pair, yaml_document_append_sequence_item, yaml_document_delete,
    yaml_document_get_node, yaml_document_get_root_node, yaml_document_initialize, yaml_document_t,
    yaml_error_type_t, yaml_mapping_style_t, yaml_node_t, yaml_node_type_t, yaml_parser_delete,
    yaml_parser_initialize, yaml_parser_load, yaml_parser_set_input, yaml_parser_set_input_string,
    yaml_parser_t, yaml_scalar_style_t, yaml_sequence_style_t, yaml_tag_directive_t,
    yaml_version_directive_t,
};

macro_rules! cstr {
    ($value:literal) => {
        unsafe { ::std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($value, "\0").as_bytes()) }
    };
}

const EXAMPLE_TAG_PREFIX: &[u8] = b"tag:example.com,2026:\0";
const EXAMPLE_TEXT_TAG: &[u8] = b"tag:example.com,2026:text\0";

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

    let count = if reader.chunk == 0 {
        size.min(remaining)
    } else {
        size.min(reader.chunk).min(remaining)
    };
    ptr::copy_nonoverlapping(reader.input.add(reader.offset), buffer, count);
    reader.offset += count;
    *size_read = count;
    1
}

#[test]
fn document_initialize_deep_copies_metadata_and_delete_zeroes_memory() {
    unsafe {
        let mut document = mem::zeroed::<yaml_document_t>();
        let mut version = yaml_version_directive_t { major: 1, minor: 2 };
        let mut handle = b"!e!\0".to_vec();
        let mut prefix = EXAMPLE_TAG_PREFIX.to_vec();
        let mut tags = [yaml_tag_directive_t {
            handle: handle.as_mut_ptr(),
            prefix: prefix.as_mut_ptr(),
        }];

        assert_eq!(
            yaml_document_initialize(
                &mut document,
                &mut version,
                tags.as_mut_ptr(),
                tags.as_mut_ptr().add(1),
                0,
                1,
            ),
            1
        );
        assert_ne!(
            document.version_directive,
            &mut version as *mut yaml_version_directive_t
        );
        assert_eq!(document.version_directive.as_ref().unwrap().major, 1);
        assert_eq!(document.version_directive.as_ref().unwrap().minor, 2);
        assert_eq!(
            document
                .tag_directives
                .end
                .offset_from(document.tag_directives.start),
            1
        );
        assert_ne!(document.tag_directives.start, tags.as_mut_ptr());

        version.major = 9;
        handle[0] = b'X';
        prefix[0] = b'X';
        assert_eq!(document.version_directive.as_ref().unwrap().major, 1);
        assert_eq!(
            CStr::from_ptr(document.tag_directives.start.read().handle.cast()),
            cstr!("!e!")
        );
        assert_eq!(
            CStr::from_ptr(document.tag_directives.start.read().prefix.cast()),
            cstr!("tag:example.com,2026:")
        );

        yaml_document_delete(&mut document);
        assert!(as_bytes(&document).iter().all(|byte| *byte == 0));
    }
}

#[test]
fn document_mutators_preserve_node_numbering_and_structure() {
    unsafe {
        let mut document = mem::zeroed::<yaml_document_t>();
        build_roundtrip_document(&mut document);

        assert!(yaml_document_get_node(&mut document, 0).is_null());
        let root = yaml_document_get_root_node(&mut document);
        assert_eq!(root, yaml_document_get_node(&mut document, 1));
        assert_eq!((*root).r#type, yaml_node_type_t::YAML_MAPPING_NODE);

        let items = lookup_mapping_value(&mut document, root, b"items").expect("items");
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
        let first_item =
            yaml_document_get_node(&mut document, (*items).data.sequence.items.start.read());
        assert_eq!(scalar_value(first_item), b"one");

        let meta = lookup_mapping_value(&mut document, root, b"meta").expect("meta");
        assert_eq!((*meta).r#type, yaml_node_type_t::YAML_MAPPING_NODE);
        assert_eq!(
            (*meta)
                .data
                .mapping
                .pairs
                .top
                .offset_from((*meta).data.mapping.pairs.start),
            1
        );
        let count = lookup_mapping_value(&mut document, meta, b"count").expect("count");
        assert_eq!(scalar_value(count), b"2");

        yaml_document_delete(&mut document);
        assert!(as_bytes(&document).iter().all(|byte| *byte == 0));
    }
}

#[test]
fn parser_load_builds_documents_and_returns_empty_document_on_stream_end() {
    let input = b"%YAML 1.1\n%TAG !e! tag:example.com,2026:\n---\nmessage: &item !e!text \"value\"\nalias: *item\nseq: [one, two]\nmeta: {count: 2}\n...\n";

    unsafe {
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut document = mem::zeroed::<yaml_document_t>();
        let mut end = mem::zeroed::<yaml_document_t>();
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());

        assert_eq!(yaml_parser_load(&mut parser, &mut document), 1);
        assert_eq!(document.version_directive.as_ref().unwrap().major, 1);
        assert_eq!(document.version_directive.as_ref().unwrap().minor, 1);
        assert_eq!(
            document
                .tag_directives
                .end
                .offset_from(document.tag_directives.start),
            1
        );

        let root = yaml_document_get_root_node(&mut document);
        assert!(!root.is_null());
        assert_eq!((*root).r#type, yaml_node_type_t::YAML_MAPPING_NODE);

        let message = lookup_mapping_value(&mut document, root, b"message").expect("message");
        assert_eq!(
            CStr::from_ptr((*message).tag.cast()),
            cstr!("tag:example.com,2026:text")
        );
        assert_eq!(scalar_value(message), b"value");

        let alias = lookup_mapping_value(&mut document, root, b"alias").expect("alias");
        assert_eq!(alias, message);

        let seq = lookup_mapping_value(&mut document, root, b"seq").expect("seq");
        assert_eq!((*seq).r#type, yaml_node_type_t::YAML_SEQUENCE_NODE);
        assert_eq!(
            (*seq)
                .data
                .sequence
                .items
                .top
                .offset_from((*seq).data.sequence.items.start),
            2
        );

        let meta = lookup_mapping_value(&mut document, root, b"meta").expect("meta");
        let count = lookup_mapping_value(&mut document, meta, b"count").expect("count");
        assert_eq!(scalar_value(count), b"2");

        yaml_document_delete(&mut document);

        assert_eq!(yaml_parser_load(&mut parser, &mut end), 1);
        assert!(yaml_document_get_root_node(&mut end).is_null());
        yaml_document_delete(&mut end);
        yaml_parser_delete(&mut parser);
    }
}

#[test]
fn parser_load_reports_composer_errors_for_alias_misuse() {
    unsafe {
        let duplicate_anchor = b"first: &a 1\nsecond: &a 2\n";
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut document = mem::zeroed::<yaml_document_t>();
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(
            &mut parser,
            duplicate_anchor.as_ptr(),
            duplicate_anchor.len(),
        );
        assert_eq!(yaml_parser_load(&mut parser, &mut document), 0);
        assert_eq!(parser.error, yaml_error_type_t::YAML_COMPOSER_ERROR);
        assert_eq!(
            CStr::from_ptr(parser.context),
            cstr!("found duplicate anchor; first occurrence")
        );
        assert_eq!(CStr::from_ptr(parser.problem), cstr!("second occurrence"));
        yaml_parser_delete(&mut parser);

        let undefined_alias = b"value: *missing\n";
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut document = mem::zeroed::<yaml_document_t>();
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(&mut parser, undefined_alias.as_ptr(), undefined_alias.len());
        assert_eq!(yaml_parser_load(&mut parser, &mut document), 0);
        assert_eq!(parser.error, yaml_error_type_t::YAML_COMPOSER_ERROR);
        assert_eq!(
            CStr::from_ptr(parser.problem),
            cstr!("found undefined alias")
        );
        yaml_parser_delete(&mut parser);
    }
}

#[test]
fn parser_load_distinguishes_explicit_empty_documents_from_stream_end() {
    let input = b"---\n...\n---\nanswer: 42\n";

    unsafe {
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut empty = mem::zeroed::<yaml_document_t>();
        let mut document = mem::zeroed::<yaml_document_t>();
        let mut end = mem::zeroed::<yaml_document_t>();
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());

        assert_eq!(yaml_parser_load(&mut parser, &mut empty), 1);
        let root = yaml_document_get_root_node(&mut empty);
        assert!(!root.is_null());
        assert_eq!((*root).r#type, yaml_node_type_t::YAML_SCALAR_NODE);
        assert_eq!(scalar_value(root), b"");
        yaml_document_delete(&mut empty);

        assert_eq!(yaml_parser_load(&mut parser, &mut document), 1);
        let root = yaml_document_get_root_node(&mut document);
        assert!(!root.is_null());
        let answer = lookup_mapping_value(&mut document, root, b"answer").expect("answer");
        assert_eq!(scalar_value(answer), b"42");
        yaml_document_delete(&mut document);

        assert_eq!(yaml_parser_load(&mut parser, &mut end), 1);
        assert!(yaml_document_get_root_node(&mut end).is_null());
        yaml_document_delete(&mut end);
        yaml_parser_delete(&mut parser);
    }
}

#[test]
fn parser_load_reports_alias_misuse_through_chunked_reader() {
    unsafe {
        let duplicate_anchor = b"first: &a 1\nsecond: &a 2\n";
        let mut reader = MemoryReader {
            input: duplicate_anchor.as_ptr(),
            size: duplicate_anchor.len(),
            offset: 0,
            chunk: 1,
        };
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut document = mem::zeroed::<yaml_document_t>();
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input(
            &mut parser,
            Some(memory_read_handler),
            (&mut reader as *mut MemoryReader).cast(),
        );
        assert_eq!(yaml_parser_load(&mut parser, &mut document), 0);
        assert_eq!(parser.error, yaml_error_type_t::YAML_COMPOSER_ERROR);
        assert_eq!(
            CStr::from_ptr(parser.context),
            cstr!("found duplicate anchor; first occurrence")
        );
        assert_eq!(CStr::from_ptr(parser.problem), cstr!("second occurrence"));
        yaml_parser_delete(&mut parser);

        let undefined_alias = b"value: *missing\n";
        let mut reader = MemoryReader {
            input: undefined_alias.as_ptr(),
            size: undefined_alias.len(),
            offset: 0,
            chunk: 1,
        };
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut document = mem::zeroed::<yaml_document_t>();
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input(
            &mut parser,
            Some(memory_read_handler),
            (&mut reader as *mut MemoryReader).cast(),
        );
        assert_eq!(yaml_parser_load(&mut parser, &mut document), 0);
        assert_eq!(parser.error, yaml_error_type_t::YAML_COMPOSER_ERROR);
        assert_eq!(
            CStr::from_ptr(parser.problem),
            cstr!("found undefined alias")
        );
        yaml_parser_delete(&mut parser);
    }
}

#[test]
fn parser_load_supports_chunked_generic_read_handlers() {
    let input = b"answer: 42\n";
    let mut reader = MemoryReader {
        input: input.as_ptr(),
        size: input.len(),
        offset: 0,
        chunk: 2,
    };

    unsafe {
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut document = mem::zeroed::<yaml_document_t>();
        let mut end = mem::zeroed::<yaml_document_t>();
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input(
            &mut parser,
            Some(memory_read_handler),
            (&mut reader as *mut MemoryReader).cast(),
        );

        assert_eq!(yaml_parser_load(&mut parser, &mut document), 1);
        assert!(reader.offset > 0);
        let root = yaml_document_get_root_node(&mut document);
        let answer = lookup_mapping_value(&mut document, root, b"answer").expect("answer");
        assert_eq!(scalar_value(answer), b"42");
        yaml_document_delete(&mut document);

        assert_eq!(yaml_parser_load(&mut parser, &mut end), 1);
        assert!(yaml_document_get_root_node(&mut end).is_null());
        yaml_document_delete(&mut end);
        yaml_parser_delete(&mut parser);
    }
}

#[test]
fn staged_install_runs_phase4_c_probes_and_upstream_loader_tests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stage_root = temp_dir("stage-root-phase-4");
    let arch = multiarch();
    let stage_lib_dir = stage_root.join("usr/lib").join(&arch);
    let compiler = compiler();

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/stage-install.sh"))
            .arg(&stage_root),
        "stage-install",
    );

    let document_probe = temp_dir("document-api-safe").join("document-api-safe");
    run_command(
        Command::new(&compiler)
            .arg("-I")
            .arg(stage_root.join("usr/include"))
            .arg(manifest_dir.join("tests/fixtures/document_api_exports.c"))
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&document_probe),
        "compile staged document_api_exports.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&document_probe),
        "assert staged loader for document_api_exports staged-header mode",
    );
    run_command(
        &mut Command::new(&document_probe),
        "run document_api_exports staged-header mode",
    );

    let document_probe_object = temp_dir("document-api-link-safe").join("document-api-safe.o");
    run_command(
        Command::new(&compiler)
            .arg("-c")
            .arg("-I")
            .arg(manifest_dir.join("include"))
            .arg(manifest_dir.join("tests/fixtures/document_api_exports.c"))
            .arg("-o")
            .arg(&document_probe_object),
        "compile document_api_exports.c against vendored header",
    );
    let document_probe_link = temp_dir("document-api-link-safe").join("document-api-link-safe");
    run_command(
        Command::new(&compiler)
            .arg(&document_probe_object)
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&document_probe_link),
        "link document_api_exports object against staged library",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&document_probe_link),
        "assert staged loader for document_api_exports object-link mode",
    );
    run_command(
        &mut Command::new(&document_probe_link),
        "run document_api_exports object-link mode",
    );

    let run_loader = temp_dir("run-loader-safe").join("run-loader-safe");
    run_command(
        Command::new(&compiler)
            .arg("-I")
            .arg(stage_root.join("usr/include"))
            .arg(manifest_dir.join("compat/original-tests/run-loader.c"))
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&run_loader),
        "compile upstream run-loader.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&run_loader),
        "assert staged loader for run-loader",
    );
    let run_loader_output = Command::new(&run_loader)
        .arg(manifest_dir.join("compat/examples/anchors.yaml"))
        .arg(manifest_dir.join("compat/examples/json.yaml"))
        .arg(manifest_dir.join("compat/examples/mapping.yaml"))
        .arg(manifest_dir.join("compat/examples/tags.yaml"))
        .output()
        .expect("failed to run upstream run-loader");
    assert!(
        run_loader_output.status.success(),
        "upstream run-loader exited with failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_loader_output.stdout),
        String::from_utf8_lossy(&run_loader_output.stderr)
    );
    let run_loader_stdout =
        String::from_utf8(run_loader_output.stdout).expect("run-loader emitted invalid UTF-8");
    assert!(
        !run_loader_stdout.contains("FAILURE"),
        "{run_loader_stdout}"
    );
    assert_eq!(
        run_loader_stdout.matches("SUCCESS").count(),
        4,
        "{run_loader_stdout}"
    );

    let test_reader = temp_dir("test-reader-safe").join("test-reader-safe");
    run_command(
        Command::new(&compiler)
            .arg("-I")
            .arg(stage_root.join("usr/include"))
            .arg(manifest_dir.join("compat/original-tests/test-reader.c"))
            .arg("-L")
            .arg(&stage_lib_dir)
            .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
            .arg("-lyaml")
            .arg("-o")
            .arg(&test_reader),
        "compile upstream test-reader.c",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&test_reader),
        "assert staged loader for test-reader",
    );
    let test_reader_output = Command::new(&test_reader)
        .output()
        .expect("failed to run upstream test-reader");
    assert!(
        test_reader_output.status.success(),
        "upstream test-reader exited with failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&test_reader_output.stdout),
        String::from_utf8_lossy(&test_reader_output.stderr)
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
        b"tag:yaml.org,2002:map\0".as_ptr(),
        yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE,
    );
    let message_key = yaml_document_add_scalar(
        document,
        b"tag:yaml.org,2002:str\0".as_ptr(),
        b"message\0".as_ptr(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let message_value = yaml_document_add_scalar(
        document,
        EXAMPLE_TEXT_TAG.as_ptr(),
        b"hello\0".as_ptr(),
        -1,
        yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
    );
    let items_key = yaml_document_add_scalar(
        document,
        b"tag:yaml.org,2002:str\0".as_ptr(),
        b"items\0".as_ptr(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let items_value = yaml_document_add_sequence(
        document,
        b"tag:yaml.org,2002:seq\0".as_ptr(),
        yaml_sequence_style_t::YAML_BLOCK_SEQUENCE_STYLE,
    );
    let first_item = yaml_document_add_scalar(
        document,
        b"tag:yaml.org,2002:str\0".as_ptr(),
        b"one\0".as_ptr(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let second_item = yaml_document_add_scalar(
        document,
        b"tag:yaml.org,2002:str\0".as_ptr(),
        b"two\0".as_ptr(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let meta_key = yaml_document_add_scalar(
        document,
        b"tag:yaml.org,2002:str\0".as_ptr(),
        b"meta\0".as_ptr(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let meta_value = yaml_document_add_mapping(
        document,
        b"tag:yaml.org,2002:map\0".as_ptr(),
        yaml_mapping_style_t::YAML_FLOW_MAPPING_STYLE,
    );
    let count_key = yaml_document_add_scalar(
        document,
        b"tag:yaml.org,2002:str\0".as_ptr(),
        b"count\0".as_ptr(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    let count_value = yaml_document_add_scalar(
        document,
        b"tag:yaml.org,2002:int\0".as_ptr(),
        b"2\0".as_ptr(),
        -1,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );

    assert_eq!(root, 1);
    assert_eq!(
        yaml_document_get_root_node(document),
        yaml_document_get_node(document, root)
    );
    assert_eq!(message_key, 2);
    assert_eq!(message_value, 3);
    assert_eq!(items_key, 4);
    assert_eq!(items_value, 5);
    assert_eq!(first_item, 6);
    assert_eq!(second_item, 7);
    assert_eq!(meta_key, 8);
    assert_eq!(meta_value, 9);
    assert_eq!(count_key, 10);
    assert_eq!(count_value, 11);

    assert_eq!(
        yaml_document_append_mapping_pair(document, root, message_key, message_value),
        1
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
    assert_eq!(
        yaml_document_append_mapping_pair(document, meta_value, count_key, count_value),
        1
    );
    assert_eq!(
        yaml_document_append_mapping_pair(document, root, meta_key, meta_value),
        1
    );
}

unsafe fn lookup_mapping_value(
    document: *mut yaml_document_t,
    mapping: *mut yaml_node_t,
    key: &[u8],
) -> Option<*mut yaml_node_t> {
    if mapping.is_null() || (*mapping).r#type != yaml_node_type_t::YAML_MAPPING_NODE {
        return None;
    }

    let mut pair = (*mapping).data.mapping.pairs.start;
    while pair < (*mapping).data.mapping.pairs.top {
        let key_node = yaml_document_get_node(document, (*pair).key);
        if !key_node.is_null()
            && (*key_node).r#type == yaml_node_type_t::YAML_SCALAR_NODE
            && scalar_value(key_node) == key
        {
            let value_node = yaml_document_get_node(document, (*pair).value);
            if !value_node.is_null() {
                return Some(value_node);
            }
        }
        pair = pair.add(1);
    }

    None
}

unsafe fn scalar_value<'a>(node: *mut yaml_node_t) -> &'a [u8] {
    slice::from_raw_parts((*node).data.scalar.value, (*node).data.scalar.length)
}

fn as_bytes<T>(value: &T) -> &[u8] {
    unsafe { slice::from_raw_parts((value as *const T).cast::<u8>(), mem::size_of::<T>()) }
}

fn run_command(command: &mut Command, context: &str) {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("{context}: failed to spawn command: {error}"));
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    path.push(format!("yaml-{label}-{unique}"));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn multiarch() -> String {
    for compiler in ["cc", "gcc"] {
        let output = Command::new(compiler).arg("-print-multiarch").output();
        if let Ok(output) = output {
            if output.status.success() {
                let value = String::from_utf8(output.stdout)
                    .expect("compiler multiarch output should be utf8")
                    .trim()
                    .to_owned();
                if !value.is_empty() {
                    return value;
                }
            }
        }
    }

    format!("{}-linux-gnu", std::env::consts::ARCH)
}

fn compiler() -> String {
    env::var("CC").unwrap_or_else(|_| String::from("cc"))
}
