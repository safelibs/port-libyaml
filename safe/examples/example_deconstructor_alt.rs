use std::env;
use std::ffi::{c_void, CStr};
use std::fs;
use std::io::{self, Read, Write};
use std::mem;
use std::process;
use std::ptr;
use std::slice;

use yaml::{
    yaml_document_add_mapping, yaml_document_add_scalar, yaml_document_add_sequence,
    yaml_document_append_mapping_pair, yaml_document_append_sequence_item, yaml_document_delete,
    yaml_document_initialize, yaml_document_t, yaml_emitter_close, yaml_emitter_delete,
    yaml_emitter_dump, yaml_emitter_initialize, yaml_emitter_open, yaml_emitter_set_canonical,
    yaml_emitter_set_output, yaml_emitter_set_unicode, yaml_emitter_t, yaml_event_delete,
    yaml_event_t, yaml_mapping_style_t, yaml_parser_delete, yaml_parser_initialize,
    yaml_parser_parse, yaml_parser_set_input_string, yaml_parser_t, yaml_scalar_style_t,
    yaml_sequence_style_t,
};

const BOOL_TAG: &[u8] = b"tag:yaml.org,2002:bool\0";
const INT_TAG: &[u8] = b"tag:yaml.org,2002:int\0";

fn main() {
    let mut canonical = 0;
    let mut unicode = 0;
    let mut input_path = None;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!(
                    "{} [--canonical] [--unicode] [input.yaml]\nor\n{} -h | --help\nDeconstruct a YAML stream",
                    program_name(),
                    program_name()
                );
                return;
            }
            "-c" | "--canonical" => canonical = 1,
            "-u" | "--unicode" => unicode = 1,
            _ => input_path = Some(arg),
        }
    }

    let input = read_input(input_path.as_deref());

    unsafe {
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut emitter = mem::zeroed::<yaml_emitter_t>();
        let mut input_event = mem::zeroed::<yaml_event_t>();
        let mut output_document = mem::zeroed::<yaml_document_t>();
        let mut stdout = io::stdout().lock();

        if yaml_parser_initialize(&mut parser) == 0 {
            eprintln!("Could not initialize the parser object");
            process::exit(1);
        }
        if yaml_emitter_initialize(&mut emitter) == 0 {
            yaml_parser_delete(&mut parser);
            eprintln!("Could not initialize the emitter object");
            process::exit(1);
        }

        yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());
        yaml_emitter_set_output(
            &mut emitter,
            Some(write_to_stdout),
            (&mut stdout as *mut _ as *mut c_void).cast(),
        );
        yaml_emitter_set_canonical(&mut emitter, canonical);
        yaml_emitter_set_unicode(&mut emitter, unicode);

        if yaml_emitter_open(&mut emitter) == 0 {
            eprintln!("Emitter error: {}", cstr(emitter.problem));
            process::exit(1);
        }
        if yaml_document_initialize(
            &mut output_document,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            0,
        ) == 0
        {
            eprintln!("Could not initialize the output document");
            process::exit(1);
        }

        let root = yaml_document_add_sequence(
            &mut output_document,
            ptr::null(),
            yaml_sequence_style_t::YAML_BLOCK_SEQUENCE_STYLE,
        );
        if root == 0 {
            yaml_document_delete(&mut output_document);
            process::exit(1);
        }

        loop {
            if yaml_parser_parse(&mut parser, &mut input_event) == 0 {
                eprintln!("Parse error: {}", cstr(parser.problem));
                yaml_document_delete(&mut output_document);
                process::exit(1);
            }

            let properties = yaml_document_add_mapping(
                &mut output_document,
                ptr::null(),
                yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE,
            );
            ensure(properties != 0, &mut output_document);
            ensure(
                yaml_document_append_sequence_item(&mut output_document, root, properties) == 1,
                &mut output_document,
            );

            add_pair(
                &mut output_document,
                properties,
                b"type",
                event_name(&input_event),
                ptr::null(),
                yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
            );

            match input_event.r#type {
                yaml::yaml_event_type_t::YAML_STREAM_START_EVENT => {
                    let encoding = match input_event.data.stream_start.encoding {
                        yaml::yaml_encoding_t::YAML_UTF8_ENCODING => Some(b"utf-8".as_slice()),
                        yaml::yaml_encoding_t::YAML_UTF16LE_ENCODING => {
                            Some(b"utf-16-le".as_slice())
                        }
                        yaml::yaml_encoding_t::YAML_UTF16BE_ENCODING => {
                            Some(b"utf-16-be".as_slice())
                        }
                        yaml::yaml_encoding_t::YAML_ANY_ENCODING => None,
                    };
                    if let Some(encoding) = encoding {
                        add_pair(
                            &mut output_document,
                            properties,
                            b"encoding",
                            encoding,
                            ptr::null(),
                            yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
                        );
                    }
                }
                yaml::yaml_event_type_t::YAML_DOCUMENT_START_EVENT => {
                    if !input_event.data.document_start.version_directive.is_null() {
                        let version = yaml_document_add_mapping(
                            &mut output_document,
                            ptr::null(),
                            yaml_mapping_style_t::YAML_FLOW_MAPPING_STYLE,
                        );
                        ensure(version != 0, &mut output_document);
                        add_mapping_pair_node(
                            &mut output_document,
                            properties,
                            b"version",
                            version,
                        );
                        add_pair(
                            &mut output_document,
                            version,
                            b"major",
                            format!(
                                "{}",
                                (*input_event.data.document_start.version_directive).major
                            )
                            .as_bytes(),
                            INT_TAG.as_ptr(),
                            yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
                        );
                        add_pair(
                            &mut output_document,
                            version,
                            b"minor",
                            format!(
                                "{}",
                                (*input_event.data.document_start.version_directive).minor
                            )
                            .as_bytes(),
                            INT_TAG.as_ptr(),
                            yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
                        );
                    }

                    let tags_start = input_event.data.document_start.tag_directives.start;
                    let tags_end = input_event.data.document_start.tag_directives.end;
                    if tags_start != tags_end {
                        let tags = yaml_document_add_sequence(
                            &mut output_document,
                            ptr::null(),
                            yaml_sequence_style_t::YAML_BLOCK_SEQUENCE_STYLE,
                        );
                        ensure(tags != 0, &mut output_document);
                        add_mapping_pair_node(&mut output_document, properties, b"tags", tags);

                        let mut tag = tags_start;
                        while tag < tags_end {
                            let entry = yaml_document_add_mapping(
                                &mut output_document,
                                ptr::null(),
                                yaml_mapping_style_t::YAML_FLOW_MAPPING_STYLE,
                            );
                            ensure(entry != 0, &mut output_document);
                            ensure(
                                yaml_document_append_sequence_item(
                                    &mut output_document,
                                    tags,
                                    entry,
                                ) == 1,
                                &mut output_document,
                            );
                            add_pair(
                                &mut output_document,
                                entry,
                                b"handle",
                                cbytes((*tag).handle),
                                ptr::null(),
                                yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                            );
                            add_pair(
                                &mut output_document,
                                entry,
                                b"prefix",
                                cbytes((*tag).prefix),
                                ptr::null(),
                                yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                            );
                            tag = tag.add(1);
                        }
                    }

                    add_bool(
                        &mut output_document,
                        properties,
                        b"implicit",
                        input_event.data.document_start.implicit != 0,
                    );
                }
                yaml::yaml_event_type_t::YAML_DOCUMENT_END_EVENT => {
                    add_bool(
                        &mut output_document,
                        properties,
                        b"implicit",
                        input_event.data.document_end.implicit != 0,
                    );
                }
                yaml::yaml_event_type_t::YAML_ALIAS_EVENT => {
                    add_pair(
                        &mut output_document,
                        properties,
                        b"anchor",
                        cbytes(input_event.data.alias.anchor),
                        ptr::null(),
                        yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                    );
                }
                yaml::yaml_event_type_t::YAML_SCALAR_EVENT => {
                    if !input_event.data.scalar.anchor.is_null() {
                        add_pair(
                            &mut output_document,
                            properties,
                            b"anchor",
                            cbytes(input_event.data.scalar.anchor),
                            ptr::null(),
                            yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                        );
                    }
                    if !input_event.data.scalar.tag.is_null() {
                        add_pair(
                            &mut output_document,
                            properties,
                            b"tag",
                            cbytes(input_event.data.scalar.tag),
                            ptr::null(),
                            yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                        );
                    }
                    add_pair(
                        &mut output_document,
                        properties,
                        b"value",
                        slice::from_raw_parts(
                            input_event.data.scalar.value,
                            input_event.data.scalar.length,
                        ),
                        ptr::null(),
                        yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
                    );
                    let implicit = yaml_document_add_mapping(
                        &mut output_document,
                        ptr::null(),
                        yaml_mapping_style_t::YAML_FLOW_MAPPING_STYLE,
                    );
                    ensure(implicit != 0, &mut output_document);
                    add_mapping_pair_node(&mut output_document, properties, b"implicit", implicit);
                    add_bool(
                        &mut output_document,
                        implicit,
                        b"plain",
                        input_event.data.scalar.plain_implicit != 0,
                    );
                    add_bool(
                        &mut output_document,
                        implicit,
                        b"quoted",
                        input_event.data.scalar.quoted_implicit != 0,
                    );
                    add_pair(
                        &mut output_document,
                        properties,
                        b"style",
                        scalar_style_name(input_event.data.scalar.style),
                        ptr::null(),
                        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
                    );
                }
                yaml::yaml_event_type_t::YAML_SEQUENCE_START_EVENT => {
                    add_optional_anchor_and_tag(
                        &mut output_document,
                        properties,
                        input_event.data.sequence_start.anchor,
                        input_event.data.sequence_start.tag,
                    );
                    add_bool(
                        &mut output_document,
                        properties,
                        b"implicit",
                        input_event.data.sequence_start.implicit != 0,
                    );
                    add_pair(
                        &mut output_document,
                        properties,
                        b"style",
                        collection_style_name(input_event.data.sequence_start.style as i32),
                        ptr::null(),
                        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
                    );
                }
                yaml::yaml_event_type_t::YAML_MAPPING_START_EVENT => {
                    add_optional_anchor_and_tag(
                        &mut output_document,
                        properties,
                        input_event.data.mapping_start.anchor,
                        input_event.data.mapping_start.tag,
                    );
                    add_bool(
                        &mut output_document,
                        properties,
                        b"implicit",
                        input_event.data.mapping_start.implicit != 0,
                    );
                    add_pair(
                        &mut output_document,
                        properties,
                        b"style",
                        collection_style_name(input_event.data.mapping_start.style as i32),
                        ptr::null(),
                        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
                    );
                }
                _ => {}
            }

            let done = input_event.r#type == yaml::yaml_event_type_t::YAML_STREAM_END_EVENT;
            yaml_event_delete(&mut input_event);
            if done {
                break;
            }
        }

        if yaml_emitter_dump(&mut emitter, &mut output_document) == 0 {
            eprintln!("Emitter error: {}", cstr(emitter.problem));
            process::exit(1);
        }
        if yaml_emitter_close(&mut emitter) == 0 {
            eprintln!("Emitter error: {}", cstr(emitter.problem));
            process::exit(1);
        }

        yaml_parser_delete(&mut parser);
        yaml_emitter_delete(&mut emitter);
        stdout.flush().expect("failed to flush stdout");
    }
}

fn program_name() -> String {
    env::args()
        .next()
        .unwrap_or_else(|| String::from("example_deconstructor_alt"))
}

fn read_input(path: Option<&str>) -> Vec<u8> {
    match path {
        Some(path) => fs::read(path).unwrap_or_else(|error| {
            panic!("failed to read {path}: {error}");
        }),
        None => {
            let mut buffer = Vec::new();
            io::stdin()
                .read_to_end(&mut buffer)
                .expect("failed to read stdin");
            buffer
        }
    }
}

unsafe fn add_optional_anchor_and_tag(
    document: *mut yaml_document_t,
    mapping: i32,
    anchor: *const u8,
    tag: *const u8,
) {
    if !anchor.is_null() {
        add_pair(
            document,
            mapping,
            b"anchor",
            cbytes(anchor),
            ptr::null(),
            yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
        );
    }
    if !tag.is_null() {
        add_pair(
            document,
            mapping,
            b"tag",
            cbytes(tag),
            ptr::null(),
            yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
        );
    }
}

unsafe fn add_bool(document: *mut yaml_document_t, mapping: i32, key: &[u8], value: bool) {
    add_pair(
        document,
        mapping,
        key,
        if value { b"true" } else { b"false" },
        BOOL_TAG.as_ptr(),
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
}

unsafe fn add_mapping_pair_node(
    document: *mut yaml_document_t,
    mapping: i32,
    key: &[u8],
    value: i32,
) {
    let key_node = yaml_document_add_scalar(
        document,
        ptr::null(),
        key.as_ptr(),
        key.len() as i32,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    ensure(key_node != 0, document);
    ensure(
        yaml_document_append_mapping_pair(document, mapping, key_node, value) == 1,
        document,
    );
}

unsafe fn add_pair(
    document: *mut yaml_document_t,
    mapping: i32,
    key: &[u8],
    value: &[u8],
    tag: *const u8,
    style: yaml_scalar_style_t,
) {
    let key_node = yaml_document_add_scalar(
        document,
        ptr::null(),
        key.as_ptr(),
        key.len() as i32,
        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
    );
    ensure(key_node != 0, document);
    let value_node =
        yaml_document_add_scalar(document, tag, value.as_ptr(), value.len() as i32, style);
    ensure(value_node != 0, document);
    ensure(
        yaml_document_append_mapping_pair(document, mapping, key_node, value_node) == 1,
        document,
    );
}

unsafe fn ensure(ok: bool, document: *mut yaml_document_t) {
    if !ok {
        yaml_document_delete(document);
        process::exit(1);
    }
}

unsafe extern "C" fn write_to_stdout(data: *mut c_void, buffer: *mut u8, size: usize) -> i32 {
    let stdout = &mut *(data.cast::<io::StdoutLock<'_>>());
    let bytes = slice::from_raw_parts(buffer.cast_const(), size);
    stdout.write_all(bytes).map(|_| 1).unwrap_or(0)
}

unsafe fn cstr(ptr: *const i8) -> String {
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

unsafe fn cbytes(ptr: *const u8) -> &'static [u8] {
    CStr::from_ptr(ptr.cast()).to_bytes()
}

fn event_name(event: &yaml_event_t) -> &'static [u8] {
    match event.r#type {
        yaml::yaml_event_type_t::YAML_STREAM_START_EVENT => b"STREAM-START",
        yaml::yaml_event_type_t::YAML_STREAM_END_EVENT => b"STREAM-END",
        yaml::yaml_event_type_t::YAML_DOCUMENT_START_EVENT => b"DOCUMENT-START",
        yaml::yaml_event_type_t::YAML_DOCUMENT_END_EVENT => b"DOCUMENT-END",
        yaml::yaml_event_type_t::YAML_ALIAS_EVENT => b"ALIAS",
        yaml::yaml_event_type_t::YAML_SCALAR_EVENT => b"SCALAR",
        yaml::yaml_event_type_t::YAML_SEQUENCE_START_EVENT => b"SEQUENCE-START",
        yaml::yaml_event_type_t::YAML_SEQUENCE_END_EVENT => b"SEQUENCE-END",
        yaml::yaml_event_type_t::YAML_MAPPING_START_EVENT => b"MAPPING-START",
        yaml::yaml_event_type_t::YAML_MAPPING_END_EVENT => b"MAPPING-END",
        yaml::yaml_event_type_t::YAML_NO_EVENT => b"NO-EVENT",
    }
}

fn scalar_style_name(style: yaml::yaml_scalar_style_t) -> &'static [u8] {
    match style {
        yaml::yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE => b"plain",
        yaml::yaml_scalar_style_t::YAML_SINGLE_QUOTED_SCALAR_STYLE => b"single-quoted",
        yaml::yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE => b"double-quoted",
        yaml::yaml_scalar_style_t::YAML_LITERAL_SCALAR_STYLE => b"literal",
        yaml::yaml_scalar_style_t::YAML_FOLDED_SCALAR_STYLE => b"folded",
        yaml::yaml_scalar_style_t::YAML_ANY_SCALAR_STYLE => b"any",
    }
}

fn collection_style_name(style: i32) -> &'static [u8] {
    match style {
        x if x == yaml::yaml_sequence_style_t::YAML_FLOW_SEQUENCE_STYLE as i32 => b"flow",
        x if x == yaml::yaml_mapping_style_t::YAML_FLOW_MAPPING_STYLE as i32 => b"flow",
        _ => b"block",
    }
}
