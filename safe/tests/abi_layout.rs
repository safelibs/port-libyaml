use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::mem::{align_of, size_of, MaybeUninit};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use yaml::types::*;

macro_rules! rust_offset {
    ($ty:ty, $($field:tt)+) => {{
        let uninit = MaybeUninit::<$ty>::uninit();
        let base = uninit.as_ptr();
        unsafe { (std::ptr::addr_of!((*base).$($field)+) as usize) - (base as usize) }
    }};
}

macro_rules! insert_size {
    ($map:ident, $ty:ty, $label:expr) => {{
        $map.insert($label.to_owned(), (size_of::<$ty>(), align_of::<$ty>()));
    }};
}

macro_rules! insert_offset {
    ($map:ident, $ty:ty, $label:expr, $($field:tt)+) => {{
        $map.insert(format!("{}|{}", stringify!($ty), $label), rust_offset!($ty, $($field)+));
    }};
}

macro_rules! insert_enum {
    ($map:ident, $ty:ty, $label:expr, $value:expr) => {{
        $map.insert(format!("{}|{}", stringify!($ty), $label), $value as i32);
    }};
}

#[test]
fn abi_layout_matches_c_header() {
    assert_eq!(size_of::<yaml_token_t>(), 80);
    assert_eq!(size_of::<yaml_event_t>(), 104);
    assert_eq!(size_of::<yaml_node_t>(), 96);
    assert_eq!(size_of::<yaml_document_t>(), 104);
    assert_eq!(size_of::<yaml_parser_t>(), 480);
    assert_eq!(size_of::<yaml_emitter_t>(), 432);

    let probe_output = run_probe();
    let (c_sizes, c_offsets, c_enums) = parse_probe(&probe_output);

    assert_eq!(c_sizes, rust_sizes());
    assert_eq!(c_offsets, rust_offsets());
    assert_eq!(c_enums, rust_enums());
}

fn run_probe() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join("tests/fixtures/abi_layout.c");
    let include_dir = manifest_dir.join("include");
    let temp_root = temp_dir("abi-layout");
    let binary = temp_root.join("abi-layout-probe");

    let compiler = match env::var("CC") {
        Ok(value) if !value.is_empty() => value,
        _ => String::from("cc"),
    };

    let compile_output = Command::new(&compiler)
        .arg("-std=c11")
        .arg("-I")
        .arg(&include_dir)
        .arg(&fixture)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("failed to run C compiler");
    assert!(
        compile_output.status.success(),
        "C fixture compilation failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr)
    );

    let run_output = Command::new(&binary)
        .output()
        .expect("failed to execute ABI probe");
    assert!(
        run_output.status.success(),
        "ABI probe failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    String::from_utf8(run_output.stdout).expect("ABI probe emitted invalid UTF-8")
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

fn parse_probe(
    output: &str,
) -> (
    BTreeMap<String, (usize, usize)>,
    BTreeMap<String, usize>,
    BTreeMap<String, i32>,
) {
    let mut sizes = BTreeMap::new();
    let mut offsets = BTreeMap::new();
    let mut enums = BTreeMap::new();

    for line in output.lines() {
        let parts: Vec<_> = line.split('|').collect();
        match parts.as_slice() {
            ["SIZE", name, size, align] => {
                sizes.insert(
                    (*name).to_owned(),
                    (
                        size.parse::<usize>().expect("invalid size entry"),
                        align.parse::<usize>().expect("invalid align entry"),
                    ),
                );
            }
            ["OFFSET", ty, field, offset] => {
                offsets.insert(
                    format!("{ty}|{field}"),
                    offset.parse::<usize>().expect("invalid offset entry"),
                );
            }
            ["ENUM", ty, symbol, value] => {
                enums.insert(
                    format!("{ty}|{symbol}"),
                    value.parse::<i32>().expect("invalid enum entry"),
                );
            }
            _ => panic!("unexpected probe output line: {line}"),
        }
    }

    (sizes, offsets, enums)
}

fn rust_sizes() -> BTreeMap<String, (usize, usize)> {
    let mut sizes = BTreeMap::new();

    insert_size!(sizes, yaml_char_t, "yaml_char_t");
    insert_size!(sizes, yaml_version_directive_t, "yaml_version_directive_t");
    insert_size!(sizes, yaml_tag_directive_t, "yaml_tag_directive_t");
    insert_size!(sizes, yaml_encoding_t, "yaml_encoding_t");
    insert_size!(sizes, yaml_break_t, "yaml_break_t");
    insert_size!(sizes, yaml_error_type_t, "yaml_error_type_t");
    insert_size!(sizes, yaml_mark_t, "yaml_mark_t");
    insert_size!(sizes, yaml_scalar_style_t, "yaml_scalar_style_t");
    insert_size!(sizes, yaml_sequence_style_t, "yaml_sequence_style_t");
    insert_size!(sizes, yaml_mapping_style_t, "yaml_mapping_style_t");
    insert_size!(sizes, yaml_token_type_t, "yaml_token_type_t");
    insert_size!(sizes, yaml_token_t, "yaml_token_t");
    insert_size!(sizes, yaml_event_type_t, "yaml_event_type_t");
    insert_size!(sizes, yaml_event_t, "yaml_event_t");
    insert_size!(sizes, yaml_node_type_t, "yaml_node_type_t");
    insert_size!(sizes, yaml_node_item_t, "yaml_node_item_t");
    insert_size!(sizes, yaml_node_pair_t, "yaml_node_pair_t");
    insert_size!(sizes, yaml_node_t, "yaml_node_t");
    insert_size!(sizes, yaml_document_t, "yaml_document_t");
    insert_size!(sizes, yaml_simple_key_t, "yaml_simple_key_t");
    insert_size!(sizes, yaml_parser_state_t, "yaml_parser_state_t");
    insert_size!(sizes, yaml_alias_data_t, "yaml_alias_data_t");
    insert_size!(sizes, yaml_parser_t, "yaml_parser_t");
    insert_size!(sizes, yaml_emitter_state_t, "yaml_emitter_state_t");
    insert_size!(sizes, yaml_anchors_t, "yaml_anchors_t");
    insert_size!(sizes, yaml_emitter_t, "yaml_emitter_t");

    sizes
}

fn rust_offsets() -> BTreeMap<String, usize> {
    let mut offsets = BTreeMap::new();

    insert_offset!(offsets, yaml_version_directive_t, "major", major);
    insert_offset!(offsets, yaml_version_directive_t, "minor", minor);

    insert_offset!(offsets, yaml_tag_directive_t, "handle", handle);
    insert_offset!(offsets, yaml_tag_directive_t, "prefix", prefix);

    insert_offset!(offsets, yaml_mark_t, "index", index);
    insert_offset!(offsets, yaml_mark_t, "line", line);
    insert_offset!(offsets, yaml_mark_t, "column", column);

    insert_offset!(offsets, yaml_token_t, "type", r#type);
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.stream_start",
        data.stream_start
    );
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.stream_start.encoding",
        data.stream_start.encoding
    );
    insert_offset!(offsets, yaml_token_t, "data.alias", data.alias);
    insert_offset!(offsets, yaml_token_t, "data.alias.value", data.alias.value);
    insert_offset!(offsets, yaml_token_t, "data.anchor", data.anchor);
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.anchor.value",
        data.anchor.value
    );
    insert_offset!(offsets, yaml_token_t, "data.tag", data.tag);
    insert_offset!(offsets, yaml_token_t, "data.tag.handle", data.tag.handle);
    insert_offset!(offsets, yaml_token_t, "data.tag.suffix", data.tag.suffix);
    insert_offset!(offsets, yaml_token_t, "data.scalar", data.scalar);
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.scalar.value",
        data.scalar.value
    );
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.scalar.length",
        data.scalar.length
    );
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.scalar.style",
        data.scalar.style
    );
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.version_directive",
        data.version_directive
    );
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.version_directive.major",
        data.version_directive.major
    );
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.version_directive.minor",
        data.version_directive.minor
    );
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.tag_directive",
        data.tag_directive
    );
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.tag_directive.handle",
        data.tag_directive.handle
    );
    insert_offset!(
        offsets,
        yaml_token_t,
        "data.tag_directive.prefix",
        data.tag_directive.prefix
    );
    insert_offset!(offsets, yaml_token_t, "start_mark", start_mark);
    insert_offset!(offsets, yaml_token_t, "start_mark.index", start_mark.index);
    insert_offset!(offsets, yaml_token_t, "start_mark.line", start_mark.line);
    insert_offset!(
        offsets,
        yaml_token_t,
        "start_mark.column",
        start_mark.column
    );
    insert_offset!(offsets, yaml_token_t, "end_mark", end_mark);
    insert_offset!(offsets, yaml_token_t, "end_mark.index", end_mark.index);
    insert_offset!(offsets, yaml_token_t, "end_mark.line", end_mark.line);
    insert_offset!(offsets, yaml_token_t, "end_mark.column", end_mark.column);

    insert_offset!(offsets, yaml_event_t, "type", r#type);
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.stream_start",
        data.stream_start
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.stream_start.encoding",
        data.stream_start.encoding
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.document_start",
        data.document_start
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.document_start.version_directive",
        data.document_start.version_directive
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.document_start.tag_directives",
        data.document_start.tag_directives
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.document_start.tag_directives.start",
        data.document_start.tag_directives.start
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.document_start.tag_directives.end",
        data.document_start.tag_directives.end
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.document_start.implicit",
        data.document_start.implicit
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.document_end",
        data.document_end
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.document_end.implicit",
        data.document_end.implicit
    );
    insert_offset!(offsets, yaml_event_t, "data.alias", data.alias);
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.alias.anchor",
        data.alias.anchor
    );
    insert_offset!(offsets, yaml_event_t, "data.scalar", data.scalar);
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.scalar.anchor",
        data.scalar.anchor
    );
    insert_offset!(offsets, yaml_event_t, "data.scalar.tag", data.scalar.tag);
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.scalar.value",
        data.scalar.value
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.scalar.length",
        data.scalar.length
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.scalar.plain_implicit",
        data.scalar.plain_implicit
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.scalar.quoted_implicit",
        data.scalar.quoted_implicit
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.scalar.style",
        data.scalar.style
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.sequence_start",
        data.sequence_start
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.sequence_start.anchor",
        data.sequence_start.anchor
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.sequence_start.tag",
        data.sequence_start.tag
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.sequence_start.implicit",
        data.sequence_start.implicit
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.sequence_start.style",
        data.sequence_start.style
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.mapping_start",
        data.mapping_start
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.mapping_start.anchor",
        data.mapping_start.anchor
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.mapping_start.tag",
        data.mapping_start.tag
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.mapping_start.implicit",
        data.mapping_start.implicit
    );
    insert_offset!(
        offsets,
        yaml_event_t,
        "data.mapping_start.style",
        data.mapping_start.style
    );
    insert_offset!(offsets, yaml_event_t, "start_mark", start_mark);
    insert_offset!(offsets, yaml_event_t, "start_mark.index", start_mark.index);
    insert_offset!(offsets, yaml_event_t, "start_mark.line", start_mark.line);
    insert_offset!(
        offsets,
        yaml_event_t,
        "start_mark.column",
        start_mark.column
    );
    insert_offset!(offsets, yaml_event_t, "end_mark", end_mark);
    insert_offset!(offsets, yaml_event_t, "end_mark.index", end_mark.index);
    insert_offset!(offsets, yaml_event_t, "end_mark.line", end_mark.line);
    insert_offset!(offsets, yaml_event_t, "end_mark.column", end_mark.column);

    insert_offset!(offsets, yaml_node_pair_t, "key", key);
    insert_offset!(offsets, yaml_node_pair_t, "value", value);

    insert_offset!(offsets, yaml_node_t, "type", r#type);
    insert_offset!(offsets, yaml_node_t, "tag", tag);
    insert_offset!(offsets, yaml_node_t, "data.scalar", data.scalar);
    insert_offset!(offsets, yaml_node_t, "data.scalar.value", data.scalar.value);
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.scalar.length",
        data.scalar.length
    );
    insert_offset!(offsets, yaml_node_t, "data.scalar.style", data.scalar.style);
    insert_offset!(offsets, yaml_node_t, "data.sequence", data.sequence);
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.sequence.items",
        data.sequence.items
    );
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.sequence.items.start",
        data.sequence.items.start
    );
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.sequence.items.end",
        data.sequence.items.end
    );
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.sequence.items.top",
        data.sequence.items.top
    );
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.sequence.style",
        data.sequence.style
    );
    insert_offset!(offsets, yaml_node_t, "data.mapping", data.mapping);
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.mapping.pairs",
        data.mapping.pairs
    );
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.mapping.pairs.start",
        data.mapping.pairs.start
    );
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.mapping.pairs.end",
        data.mapping.pairs.end
    );
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.mapping.pairs.top",
        data.mapping.pairs.top
    );
    insert_offset!(
        offsets,
        yaml_node_t,
        "data.mapping.style",
        data.mapping.style
    );
    insert_offset!(offsets, yaml_node_t, "start_mark", start_mark);
    insert_offset!(offsets, yaml_node_t, "start_mark.index", start_mark.index);
    insert_offset!(offsets, yaml_node_t, "start_mark.line", start_mark.line);
    insert_offset!(offsets, yaml_node_t, "start_mark.column", start_mark.column);
    insert_offset!(offsets, yaml_node_t, "end_mark", end_mark);
    insert_offset!(offsets, yaml_node_t, "end_mark.index", end_mark.index);
    insert_offset!(offsets, yaml_node_t, "end_mark.line", end_mark.line);
    insert_offset!(offsets, yaml_node_t, "end_mark.column", end_mark.column);

    insert_offset!(offsets, yaml_document_t, "nodes", nodes);
    insert_offset!(offsets, yaml_document_t, "nodes.start", nodes.start);
    insert_offset!(offsets, yaml_document_t, "nodes.end", nodes.end);
    insert_offset!(offsets, yaml_document_t, "nodes.top", nodes.top);
    insert_offset!(
        offsets,
        yaml_document_t,
        "version_directive",
        version_directive
    );
    insert_offset!(offsets, yaml_document_t, "tag_directives", tag_directives);
    insert_offset!(
        offsets,
        yaml_document_t,
        "tag_directives.start",
        tag_directives.start
    );
    insert_offset!(
        offsets,
        yaml_document_t,
        "tag_directives.end",
        tag_directives.end
    );
    insert_offset!(offsets, yaml_document_t, "start_implicit", start_implicit);
    insert_offset!(offsets, yaml_document_t, "end_implicit", end_implicit);
    insert_offset!(offsets, yaml_document_t, "start_mark", start_mark);
    insert_offset!(
        offsets,
        yaml_document_t,
        "start_mark.index",
        start_mark.index
    );
    insert_offset!(offsets, yaml_document_t, "start_mark.line", start_mark.line);
    insert_offset!(
        offsets,
        yaml_document_t,
        "start_mark.column",
        start_mark.column
    );
    insert_offset!(offsets, yaml_document_t, "end_mark", end_mark);
    insert_offset!(offsets, yaml_document_t, "end_mark.index", end_mark.index);
    insert_offset!(offsets, yaml_document_t, "end_mark.line", end_mark.line);
    insert_offset!(offsets, yaml_document_t, "end_mark.column", end_mark.column);

    insert_offset!(offsets, yaml_simple_key_t, "possible", possible);
    insert_offset!(offsets, yaml_simple_key_t, "required", required);
    insert_offset!(offsets, yaml_simple_key_t, "token_number", token_number);
    insert_offset!(offsets, yaml_simple_key_t, "mark", mark);
    insert_offset!(offsets, yaml_simple_key_t, "mark.index", mark.index);
    insert_offset!(offsets, yaml_simple_key_t, "mark.line", mark.line);
    insert_offset!(offsets, yaml_simple_key_t, "mark.column", mark.column);

    insert_offset!(offsets, yaml_alias_data_t, "anchor", anchor);
    insert_offset!(offsets, yaml_alias_data_t, "index", index);
    insert_offset!(offsets, yaml_alias_data_t, "mark", mark);
    insert_offset!(offsets, yaml_alias_data_t, "mark.index", mark.index);
    insert_offset!(offsets, yaml_alias_data_t, "mark.line", mark.line);
    insert_offset!(offsets, yaml_alias_data_t, "mark.column", mark.column);

    insert_offset!(offsets, yaml_parser_t, "error", error);
    insert_offset!(offsets, yaml_parser_t, "problem", problem);
    insert_offset!(offsets, yaml_parser_t, "problem_offset", problem_offset);
    insert_offset!(offsets, yaml_parser_t, "problem_value", problem_value);
    insert_offset!(offsets, yaml_parser_t, "problem_mark", problem_mark);
    insert_offset!(
        offsets,
        yaml_parser_t,
        "problem_mark.index",
        problem_mark.index
    );
    insert_offset!(
        offsets,
        yaml_parser_t,
        "problem_mark.line",
        problem_mark.line
    );
    insert_offset!(
        offsets,
        yaml_parser_t,
        "problem_mark.column",
        problem_mark.column
    );
    insert_offset!(offsets, yaml_parser_t, "context", context);
    insert_offset!(offsets, yaml_parser_t, "context_mark", context_mark);
    insert_offset!(
        offsets,
        yaml_parser_t,
        "context_mark.index",
        context_mark.index
    );
    insert_offset!(
        offsets,
        yaml_parser_t,
        "context_mark.line",
        context_mark.line
    );
    insert_offset!(
        offsets,
        yaml_parser_t,
        "context_mark.column",
        context_mark.column
    );
    insert_offset!(offsets, yaml_parser_t, "read_handler", read_handler);
    insert_offset!(
        offsets,
        yaml_parser_t,
        "read_handler_data",
        read_handler_data
    );
    insert_offset!(offsets, yaml_parser_t, "input", input);
    insert_offset!(offsets, yaml_parser_t, "input.string", input.string);
    insert_offset!(
        offsets,
        yaml_parser_t,
        "input.string.start",
        input.string.start
    );
    insert_offset!(offsets, yaml_parser_t, "input.string.end", input.string.end);
    insert_offset!(
        offsets,
        yaml_parser_t,
        "input.string.current",
        input.string.current
    );
    insert_offset!(offsets, yaml_parser_t, "input.file", input.file);
    insert_offset!(offsets, yaml_parser_t, "eof", eof);
    insert_offset!(offsets, yaml_parser_t, "buffer", buffer);
    insert_offset!(offsets, yaml_parser_t, "buffer.start", buffer.start);
    insert_offset!(offsets, yaml_parser_t, "buffer.end", buffer.end);
    insert_offset!(offsets, yaml_parser_t, "buffer.pointer", buffer.pointer);
    insert_offset!(offsets, yaml_parser_t, "buffer.last", buffer.last);
    insert_offset!(offsets, yaml_parser_t, "unread", unread);
    insert_offset!(offsets, yaml_parser_t, "raw_buffer", raw_buffer);
    insert_offset!(offsets, yaml_parser_t, "raw_buffer.start", raw_buffer.start);
    insert_offset!(offsets, yaml_parser_t, "raw_buffer.end", raw_buffer.end);
    insert_offset!(
        offsets,
        yaml_parser_t,
        "raw_buffer.pointer",
        raw_buffer.pointer
    );
    insert_offset!(offsets, yaml_parser_t, "raw_buffer.last", raw_buffer.last);
    insert_offset!(offsets, yaml_parser_t, "encoding", encoding);
    insert_offset!(offsets, yaml_parser_t, "offset", offset);
    insert_offset!(offsets, yaml_parser_t, "mark", mark);
    insert_offset!(offsets, yaml_parser_t, "mark.index", mark.index);
    insert_offset!(offsets, yaml_parser_t, "mark.line", mark.line);
    insert_offset!(offsets, yaml_parser_t, "mark.column", mark.column);
    insert_offset!(
        offsets,
        yaml_parser_t,
        "stream_start_produced",
        stream_start_produced
    );
    insert_offset!(
        offsets,
        yaml_parser_t,
        "stream_end_produced",
        stream_end_produced
    );
    insert_offset!(offsets, yaml_parser_t, "flow_level", flow_level);
    insert_offset!(offsets, yaml_parser_t, "tokens", tokens);
    insert_offset!(offsets, yaml_parser_t, "tokens.start", tokens.start);
    insert_offset!(offsets, yaml_parser_t, "tokens.end", tokens.end);
    insert_offset!(offsets, yaml_parser_t, "tokens.head", tokens.head);
    insert_offset!(offsets, yaml_parser_t, "tokens.tail", tokens.tail);
    insert_offset!(offsets, yaml_parser_t, "tokens_parsed", tokens_parsed);
    insert_offset!(offsets, yaml_parser_t, "token_available", token_available);
    insert_offset!(offsets, yaml_parser_t, "indents", indents);
    insert_offset!(offsets, yaml_parser_t, "indents.start", indents.start);
    insert_offset!(offsets, yaml_parser_t, "indents.end", indents.end);
    insert_offset!(offsets, yaml_parser_t, "indents.top", indents.top);
    insert_offset!(offsets, yaml_parser_t, "indent", indent);
    insert_offset!(
        offsets,
        yaml_parser_t,
        "simple_key_allowed",
        simple_key_allowed
    );
    insert_offset!(offsets, yaml_parser_t, "simple_keys", simple_keys);
    insert_offset!(
        offsets,
        yaml_parser_t,
        "simple_keys.start",
        simple_keys.start
    );
    insert_offset!(offsets, yaml_parser_t, "simple_keys.end", simple_keys.end);
    insert_offset!(offsets, yaml_parser_t, "simple_keys.top", simple_keys.top);
    insert_offset!(offsets, yaml_parser_t, "states", states);
    insert_offset!(offsets, yaml_parser_t, "states.start", states.start);
    insert_offset!(offsets, yaml_parser_t, "states.end", states.end);
    insert_offset!(offsets, yaml_parser_t, "states.top", states.top);
    insert_offset!(offsets, yaml_parser_t, "state", state);
    insert_offset!(offsets, yaml_parser_t, "marks", marks);
    insert_offset!(offsets, yaml_parser_t, "marks.start", marks.start);
    insert_offset!(offsets, yaml_parser_t, "marks.end", marks.end);
    insert_offset!(offsets, yaml_parser_t, "marks.top", marks.top);
    insert_offset!(offsets, yaml_parser_t, "tag_directives", tag_directives);
    insert_offset!(
        offsets,
        yaml_parser_t,
        "tag_directives.start",
        tag_directives.start
    );
    insert_offset!(
        offsets,
        yaml_parser_t,
        "tag_directives.end",
        tag_directives.end
    );
    insert_offset!(
        offsets,
        yaml_parser_t,
        "tag_directives.top",
        tag_directives.top
    );
    insert_offset!(offsets, yaml_parser_t, "aliases", aliases);
    insert_offset!(offsets, yaml_parser_t, "aliases.start", aliases.start);
    insert_offset!(offsets, yaml_parser_t, "aliases.end", aliases.end);
    insert_offset!(offsets, yaml_parser_t, "aliases.top", aliases.top);
    insert_offset!(offsets, yaml_parser_t, "document", document);

    insert_offset!(offsets, yaml_anchors_t, "references", references);
    insert_offset!(offsets, yaml_anchors_t, "anchor", anchor);
    insert_offset!(offsets, yaml_anchors_t, "serialized", serialized);

    insert_offset!(offsets, yaml_emitter_t, "error", error);
    insert_offset!(offsets, yaml_emitter_t, "problem", problem);
    insert_offset!(offsets, yaml_emitter_t, "write_handler", write_handler);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "write_handler_data",
        write_handler_data
    );
    insert_offset!(offsets, yaml_emitter_t, "output", output);
    insert_offset!(offsets, yaml_emitter_t, "output.string", output.string);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "output.string.buffer",
        output.string.buffer
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "output.string.size",
        output.string.size
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "output.string.size_written",
        output.string.size_written
    );
    insert_offset!(offsets, yaml_emitter_t, "output.file", output.file);
    insert_offset!(offsets, yaml_emitter_t, "buffer", buffer);
    insert_offset!(offsets, yaml_emitter_t, "buffer.start", buffer.start);
    insert_offset!(offsets, yaml_emitter_t, "buffer.end", buffer.end);
    insert_offset!(offsets, yaml_emitter_t, "buffer.pointer", buffer.pointer);
    insert_offset!(offsets, yaml_emitter_t, "buffer.last", buffer.last);
    insert_offset!(offsets, yaml_emitter_t, "raw_buffer", raw_buffer);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "raw_buffer.start",
        raw_buffer.start
    );
    insert_offset!(offsets, yaml_emitter_t, "raw_buffer.end", raw_buffer.end);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "raw_buffer.pointer",
        raw_buffer.pointer
    );
    insert_offset!(offsets, yaml_emitter_t, "raw_buffer.last", raw_buffer.last);
    insert_offset!(offsets, yaml_emitter_t, "encoding", encoding);
    insert_offset!(offsets, yaml_emitter_t, "canonical", canonical);
    insert_offset!(offsets, yaml_emitter_t, "best_indent", best_indent);
    insert_offset!(offsets, yaml_emitter_t, "best_width", best_width);
    insert_offset!(offsets, yaml_emitter_t, "unicode", unicode);
    insert_offset!(offsets, yaml_emitter_t, "line_break", line_break);
    insert_offset!(offsets, yaml_emitter_t, "states", states);
    insert_offset!(offsets, yaml_emitter_t, "states.start", states.start);
    insert_offset!(offsets, yaml_emitter_t, "states.end", states.end);
    insert_offset!(offsets, yaml_emitter_t, "states.top", states.top);
    insert_offset!(offsets, yaml_emitter_t, "state", state);
    insert_offset!(offsets, yaml_emitter_t, "events", events);
    insert_offset!(offsets, yaml_emitter_t, "events.start", events.start);
    insert_offset!(offsets, yaml_emitter_t, "events.end", events.end);
    insert_offset!(offsets, yaml_emitter_t, "events.head", events.head);
    insert_offset!(offsets, yaml_emitter_t, "events.tail", events.tail);
    insert_offset!(offsets, yaml_emitter_t, "indents", indents);
    insert_offset!(offsets, yaml_emitter_t, "indents.start", indents.start);
    insert_offset!(offsets, yaml_emitter_t, "indents.end", indents.end);
    insert_offset!(offsets, yaml_emitter_t, "indents.top", indents.top);
    insert_offset!(offsets, yaml_emitter_t, "tag_directives", tag_directives);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "tag_directives.start",
        tag_directives.start
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "tag_directives.end",
        tag_directives.end
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "tag_directives.top",
        tag_directives.top
    );
    insert_offset!(offsets, yaml_emitter_t, "indent", indent);
    insert_offset!(offsets, yaml_emitter_t, "flow_level", flow_level);
    insert_offset!(offsets, yaml_emitter_t, "root_context", root_context);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "sequence_context",
        sequence_context
    );
    insert_offset!(offsets, yaml_emitter_t, "mapping_context", mapping_context);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "simple_key_context",
        simple_key_context
    );
    insert_offset!(offsets, yaml_emitter_t, "line", line);
    insert_offset!(offsets, yaml_emitter_t, "column", column);
    insert_offset!(offsets, yaml_emitter_t, "whitespace", whitespace);
    insert_offset!(offsets, yaml_emitter_t, "indention", indention);
    insert_offset!(offsets, yaml_emitter_t, "open_ended", open_ended);
    insert_offset!(offsets, yaml_emitter_t, "anchor_data", anchor_data);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "anchor_data.anchor",
        anchor_data.anchor
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "anchor_data.anchor_length",
        anchor_data.anchor_length
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "anchor_data.alias",
        anchor_data.alias
    );
    insert_offset!(offsets, yaml_emitter_t, "tag_data", tag_data);
    insert_offset!(offsets, yaml_emitter_t, "tag_data.handle", tag_data.handle);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "tag_data.handle_length",
        tag_data.handle_length
    );
    insert_offset!(offsets, yaml_emitter_t, "tag_data.suffix", tag_data.suffix);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "tag_data.suffix_length",
        tag_data.suffix_length
    );
    insert_offset!(offsets, yaml_emitter_t, "scalar_data", scalar_data);
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "scalar_data.value",
        scalar_data.value
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "scalar_data.length",
        scalar_data.length
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "scalar_data.multiline",
        scalar_data.multiline
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "scalar_data.flow_plain_allowed",
        scalar_data.flow_plain_allowed
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "scalar_data.block_plain_allowed",
        scalar_data.block_plain_allowed
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "scalar_data.single_quoted_allowed",
        scalar_data.single_quoted_allowed
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "scalar_data.block_allowed",
        scalar_data.block_allowed
    );
    insert_offset!(
        offsets,
        yaml_emitter_t,
        "scalar_data.style",
        scalar_data.style
    );
    insert_offset!(offsets, yaml_emitter_t, "opened", opened);
    insert_offset!(offsets, yaml_emitter_t, "closed", closed);
    insert_offset!(offsets, yaml_emitter_t, "anchors", anchors);
    insert_offset!(offsets, yaml_emitter_t, "last_anchor_id", last_anchor_id);
    insert_offset!(offsets, yaml_emitter_t, "document", document);

    offsets
}

fn rust_enums() -> BTreeMap<String, i32> {
    let mut values = BTreeMap::new();

    insert_enum!(
        values,
        yaml_encoding_t,
        "YAML_ANY_ENCODING",
        yaml_encoding_t::YAML_ANY_ENCODING
    );
    insert_enum!(
        values,
        yaml_encoding_t,
        "YAML_UTF8_ENCODING",
        yaml_encoding_t::YAML_UTF8_ENCODING
    );
    insert_enum!(
        values,
        yaml_encoding_t,
        "YAML_UTF16LE_ENCODING",
        yaml_encoding_t::YAML_UTF16LE_ENCODING
    );
    insert_enum!(
        values,
        yaml_encoding_t,
        "YAML_UTF16BE_ENCODING",
        yaml_encoding_t::YAML_UTF16BE_ENCODING
    );

    insert_enum!(
        values,
        yaml_break_t,
        "YAML_ANY_BREAK",
        yaml_break_t::YAML_ANY_BREAK
    );
    insert_enum!(
        values,
        yaml_break_t,
        "YAML_CR_BREAK",
        yaml_break_t::YAML_CR_BREAK
    );
    insert_enum!(
        values,
        yaml_break_t,
        "YAML_LN_BREAK",
        yaml_break_t::YAML_LN_BREAK
    );
    insert_enum!(
        values,
        yaml_break_t,
        "YAML_CRLN_BREAK",
        yaml_break_t::YAML_CRLN_BREAK
    );

    insert_enum!(
        values,
        yaml_error_type_t,
        "YAML_NO_ERROR",
        yaml_error_type_t::YAML_NO_ERROR
    );
    insert_enum!(
        values,
        yaml_error_type_t,
        "YAML_MEMORY_ERROR",
        yaml_error_type_t::YAML_MEMORY_ERROR
    );
    insert_enum!(
        values,
        yaml_error_type_t,
        "YAML_READER_ERROR",
        yaml_error_type_t::YAML_READER_ERROR
    );
    insert_enum!(
        values,
        yaml_error_type_t,
        "YAML_SCANNER_ERROR",
        yaml_error_type_t::YAML_SCANNER_ERROR
    );
    insert_enum!(
        values,
        yaml_error_type_t,
        "YAML_PARSER_ERROR",
        yaml_error_type_t::YAML_PARSER_ERROR
    );
    insert_enum!(
        values,
        yaml_error_type_t,
        "YAML_COMPOSER_ERROR",
        yaml_error_type_t::YAML_COMPOSER_ERROR
    );
    insert_enum!(
        values,
        yaml_error_type_t,
        "YAML_WRITER_ERROR",
        yaml_error_type_t::YAML_WRITER_ERROR
    );
    insert_enum!(
        values,
        yaml_error_type_t,
        "YAML_EMITTER_ERROR",
        yaml_error_type_t::YAML_EMITTER_ERROR
    );

    insert_enum!(
        values,
        yaml_scalar_style_t,
        "YAML_ANY_SCALAR_STYLE",
        yaml_scalar_style_t::YAML_ANY_SCALAR_STYLE
    );
    insert_enum!(
        values,
        yaml_scalar_style_t,
        "YAML_PLAIN_SCALAR_STYLE",
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE
    );
    insert_enum!(
        values,
        yaml_scalar_style_t,
        "YAML_SINGLE_QUOTED_SCALAR_STYLE",
        yaml_scalar_style_t::YAML_SINGLE_QUOTED_SCALAR_STYLE
    );
    insert_enum!(
        values,
        yaml_scalar_style_t,
        "YAML_DOUBLE_QUOTED_SCALAR_STYLE",
        yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE
    );
    insert_enum!(
        values,
        yaml_scalar_style_t,
        "YAML_LITERAL_SCALAR_STYLE",
        yaml_scalar_style_t::YAML_LITERAL_SCALAR_STYLE
    );
    insert_enum!(
        values,
        yaml_scalar_style_t,
        "YAML_FOLDED_SCALAR_STYLE",
        yaml_scalar_style_t::YAML_FOLDED_SCALAR_STYLE
    );

    insert_enum!(
        values,
        yaml_sequence_style_t,
        "YAML_ANY_SEQUENCE_STYLE",
        yaml_sequence_style_t::YAML_ANY_SEQUENCE_STYLE
    );
    insert_enum!(
        values,
        yaml_sequence_style_t,
        "YAML_BLOCK_SEQUENCE_STYLE",
        yaml_sequence_style_t::YAML_BLOCK_SEQUENCE_STYLE
    );
    insert_enum!(
        values,
        yaml_sequence_style_t,
        "YAML_FLOW_SEQUENCE_STYLE",
        yaml_sequence_style_t::YAML_FLOW_SEQUENCE_STYLE
    );

    insert_enum!(
        values,
        yaml_mapping_style_t,
        "YAML_ANY_MAPPING_STYLE",
        yaml_mapping_style_t::YAML_ANY_MAPPING_STYLE
    );
    insert_enum!(
        values,
        yaml_mapping_style_t,
        "YAML_BLOCK_MAPPING_STYLE",
        yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE
    );
    insert_enum!(
        values,
        yaml_mapping_style_t,
        "YAML_FLOW_MAPPING_STYLE",
        yaml_mapping_style_t::YAML_FLOW_MAPPING_STYLE
    );

    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_NO_TOKEN",
        yaml_token_type_t::YAML_NO_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_STREAM_START_TOKEN",
        yaml_token_type_t::YAML_STREAM_START_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_STREAM_END_TOKEN",
        yaml_token_type_t::YAML_STREAM_END_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_VERSION_DIRECTIVE_TOKEN",
        yaml_token_type_t::YAML_VERSION_DIRECTIVE_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_TAG_DIRECTIVE_TOKEN",
        yaml_token_type_t::YAML_TAG_DIRECTIVE_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_DOCUMENT_START_TOKEN",
        yaml_token_type_t::YAML_DOCUMENT_START_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_DOCUMENT_END_TOKEN",
        yaml_token_type_t::YAML_DOCUMENT_END_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_BLOCK_SEQUENCE_START_TOKEN",
        yaml_token_type_t::YAML_BLOCK_SEQUENCE_START_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_BLOCK_MAPPING_START_TOKEN",
        yaml_token_type_t::YAML_BLOCK_MAPPING_START_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_BLOCK_END_TOKEN",
        yaml_token_type_t::YAML_BLOCK_END_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_FLOW_SEQUENCE_START_TOKEN",
        yaml_token_type_t::YAML_FLOW_SEQUENCE_START_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_FLOW_SEQUENCE_END_TOKEN",
        yaml_token_type_t::YAML_FLOW_SEQUENCE_END_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_FLOW_MAPPING_START_TOKEN",
        yaml_token_type_t::YAML_FLOW_MAPPING_START_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_FLOW_MAPPING_END_TOKEN",
        yaml_token_type_t::YAML_FLOW_MAPPING_END_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_BLOCK_ENTRY_TOKEN",
        yaml_token_type_t::YAML_BLOCK_ENTRY_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_FLOW_ENTRY_TOKEN",
        yaml_token_type_t::YAML_FLOW_ENTRY_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_KEY_TOKEN",
        yaml_token_type_t::YAML_KEY_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_VALUE_TOKEN",
        yaml_token_type_t::YAML_VALUE_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_ALIAS_TOKEN",
        yaml_token_type_t::YAML_ALIAS_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_ANCHOR_TOKEN",
        yaml_token_type_t::YAML_ANCHOR_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_TAG_TOKEN",
        yaml_token_type_t::YAML_TAG_TOKEN
    );
    insert_enum!(
        values,
        yaml_token_type_t,
        "YAML_SCALAR_TOKEN",
        yaml_token_type_t::YAML_SCALAR_TOKEN
    );

    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_NO_EVENT",
        yaml_event_type_t::YAML_NO_EVENT
    );
    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_STREAM_START_EVENT",
        yaml_event_type_t::YAML_STREAM_START_EVENT
    );
    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_STREAM_END_EVENT",
        yaml_event_type_t::YAML_STREAM_END_EVENT
    );
    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_DOCUMENT_START_EVENT",
        yaml_event_type_t::YAML_DOCUMENT_START_EVENT
    );
    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_DOCUMENT_END_EVENT",
        yaml_event_type_t::YAML_DOCUMENT_END_EVENT
    );
    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_ALIAS_EVENT",
        yaml_event_type_t::YAML_ALIAS_EVENT
    );
    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_SCALAR_EVENT",
        yaml_event_type_t::YAML_SCALAR_EVENT
    );
    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_SEQUENCE_START_EVENT",
        yaml_event_type_t::YAML_SEQUENCE_START_EVENT
    );
    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_SEQUENCE_END_EVENT",
        yaml_event_type_t::YAML_SEQUENCE_END_EVENT
    );
    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_MAPPING_START_EVENT",
        yaml_event_type_t::YAML_MAPPING_START_EVENT
    );
    insert_enum!(
        values,
        yaml_event_type_t,
        "YAML_MAPPING_END_EVENT",
        yaml_event_type_t::YAML_MAPPING_END_EVENT
    );

    insert_enum!(
        values,
        yaml_node_type_t,
        "YAML_NO_NODE",
        yaml_node_type_t::YAML_NO_NODE
    );
    insert_enum!(
        values,
        yaml_node_type_t,
        "YAML_SCALAR_NODE",
        yaml_node_type_t::YAML_SCALAR_NODE
    );
    insert_enum!(
        values,
        yaml_node_type_t,
        "YAML_SEQUENCE_NODE",
        yaml_node_type_t::YAML_SEQUENCE_NODE
    );
    insert_enum!(
        values,
        yaml_node_type_t,
        "YAML_MAPPING_NODE",
        yaml_node_type_t::YAML_MAPPING_NODE
    );

    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_STREAM_START_STATE",
        yaml_parser_state_t::YAML_PARSE_STREAM_START_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_IMPLICIT_DOCUMENT_START_STATE",
        yaml_parser_state_t::YAML_PARSE_IMPLICIT_DOCUMENT_START_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_DOCUMENT_START_STATE",
        yaml_parser_state_t::YAML_PARSE_DOCUMENT_START_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_DOCUMENT_CONTENT_STATE",
        yaml_parser_state_t::YAML_PARSE_DOCUMENT_CONTENT_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_DOCUMENT_END_STATE",
        yaml_parser_state_t::YAML_PARSE_DOCUMENT_END_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_BLOCK_NODE_STATE",
        yaml_parser_state_t::YAML_PARSE_BLOCK_NODE_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_BLOCK_NODE_OR_INDENTLESS_SEQUENCE_STATE",
        yaml_parser_state_t::YAML_PARSE_BLOCK_NODE_OR_INDENTLESS_SEQUENCE_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_FLOW_NODE_STATE",
        yaml_parser_state_t::YAML_PARSE_FLOW_NODE_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_BLOCK_SEQUENCE_FIRST_ENTRY_STATE",
        yaml_parser_state_t::YAML_PARSE_BLOCK_SEQUENCE_FIRST_ENTRY_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_BLOCK_SEQUENCE_ENTRY_STATE",
        yaml_parser_state_t::YAML_PARSE_BLOCK_SEQUENCE_ENTRY_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE",
        yaml_parser_state_t::YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_BLOCK_MAPPING_FIRST_KEY_STATE",
        yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_FIRST_KEY_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_BLOCK_MAPPING_KEY_STATE",
        yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_KEY_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_BLOCK_MAPPING_VALUE_STATE",
        yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_VALUE_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_FLOW_SEQUENCE_FIRST_ENTRY_STATE",
        yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_FIRST_ENTRY_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_FLOW_SEQUENCE_ENTRY_STATE",
        yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_KEY_STATE",
        yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_KEY_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE",
        yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_END_STATE",
        yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_END_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_FLOW_MAPPING_FIRST_KEY_STATE",
        yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_FIRST_KEY_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_FLOW_MAPPING_KEY_STATE",
        yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_KEY_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_FLOW_MAPPING_VALUE_STATE",
        yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_VALUE_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_FLOW_MAPPING_EMPTY_VALUE_STATE",
        yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_EMPTY_VALUE_STATE
    );
    insert_enum!(
        values,
        yaml_parser_state_t,
        "YAML_PARSE_END_STATE",
        yaml_parser_state_t::YAML_PARSE_END_STATE
    );

    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_STREAM_START_STATE",
        yaml_emitter_state_t::YAML_EMIT_STREAM_START_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_FIRST_DOCUMENT_START_STATE",
        yaml_emitter_state_t::YAML_EMIT_FIRST_DOCUMENT_START_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_DOCUMENT_START_STATE",
        yaml_emitter_state_t::YAML_EMIT_DOCUMENT_START_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_DOCUMENT_CONTENT_STATE",
        yaml_emitter_state_t::YAML_EMIT_DOCUMENT_CONTENT_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_DOCUMENT_END_STATE",
        yaml_emitter_state_t::YAML_EMIT_DOCUMENT_END_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_FLOW_SEQUENCE_FIRST_ITEM_STATE",
        yaml_emitter_state_t::YAML_EMIT_FLOW_SEQUENCE_FIRST_ITEM_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_FLOW_SEQUENCE_ITEM_STATE",
        yaml_emitter_state_t::YAML_EMIT_FLOW_SEQUENCE_ITEM_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_FLOW_MAPPING_FIRST_KEY_STATE",
        yaml_emitter_state_t::YAML_EMIT_FLOW_MAPPING_FIRST_KEY_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_FLOW_MAPPING_KEY_STATE",
        yaml_emitter_state_t::YAML_EMIT_FLOW_MAPPING_KEY_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_FLOW_MAPPING_SIMPLE_VALUE_STATE",
        yaml_emitter_state_t::YAML_EMIT_FLOW_MAPPING_SIMPLE_VALUE_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_FLOW_MAPPING_VALUE_STATE",
        yaml_emitter_state_t::YAML_EMIT_FLOW_MAPPING_VALUE_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_BLOCK_SEQUENCE_FIRST_ITEM_STATE",
        yaml_emitter_state_t::YAML_EMIT_BLOCK_SEQUENCE_FIRST_ITEM_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_BLOCK_SEQUENCE_ITEM_STATE",
        yaml_emitter_state_t::YAML_EMIT_BLOCK_SEQUENCE_ITEM_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_BLOCK_MAPPING_FIRST_KEY_STATE",
        yaml_emitter_state_t::YAML_EMIT_BLOCK_MAPPING_FIRST_KEY_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_BLOCK_MAPPING_KEY_STATE",
        yaml_emitter_state_t::YAML_EMIT_BLOCK_MAPPING_KEY_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_BLOCK_MAPPING_SIMPLE_VALUE_STATE",
        yaml_emitter_state_t::YAML_EMIT_BLOCK_MAPPING_SIMPLE_VALUE_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_BLOCK_MAPPING_VALUE_STATE",
        yaml_emitter_state_t::YAML_EMIT_BLOCK_MAPPING_VALUE_STATE
    );
    insert_enum!(
        values,
        yaml_emitter_state_t,
        "YAML_EMIT_END_STATE",
        yaml_emitter_state_t::YAML_EMIT_END_STATE
    );

    values
}
