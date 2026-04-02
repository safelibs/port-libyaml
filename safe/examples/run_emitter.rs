use std::env;
use std::ffi::CStr;
use std::fs;
use std::mem;

use yaml::{
    yaml_alias_event_initialize, yaml_document_end_event_initialize,
    yaml_document_start_event_initialize, yaml_emitter_delete, yaml_emitter_emit,
    yaml_emitter_initialize, yaml_emitter_set_output_string, yaml_emitter_t, yaml_event_delete,
    yaml_event_t, yaml_event_type_t, yaml_mapping_end_event_initialize,
    yaml_mapping_start_event_initialize, yaml_parser_delete, yaml_parser_initialize,
    yaml_parser_parse, yaml_parser_set_input_string, yaml_parser_t, yaml_scalar_event_initialize,
    yaml_sequence_end_event_initialize, yaml_sequence_start_event_initialize,
    yaml_stream_end_event_initialize, yaml_stream_start_event_initialize,
};

const BUFFER_SIZE: usize = 65_536;

fn main() {
    let files: Vec<String> = env::args().skip(1).collect();
    if files.is_empty() {
        eprintln!("Usage: run_emitter file1.yaml ...");
        return;
    }

    for (index, path) in files.iter().enumerate() {
        let input = fs::read(path).unwrap_or_else(|error| {
            panic!("failed to read {path}: {error}");
        });

        unsafe {
            let mut parser = mem::zeroed::<yaml_parser_t>();
            let mut emitter = mem::zeroed::<yaml_emitter_t>();
            let mut event = mem::zeroed::<yaml_event_t>();
            let mut emitted = vec![0u8; BUFFER_SIZE + 1];
            let mut written = 0usize;
            let mut originals = Vec::<yaml_event_t>::new();
            let mut failed = false;

            print!(
                "[{}] Parsing, emitting, and parsing again '{}': ",
                index + 1,
                path
            );
            assert_eq!(yaml_parser_initialize(&mut parser), 1);
            yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());
            assert_eq!(yaml_emitter_initialize(&mut emitter), 1);
            yaml_emitter_set_output_string(
                &mut emitter,
                emitted.as_mut_ptr(),
                BUFFER_SIZE,
                &mut written,
            );

            loop {
                if yaml_parser_parse(&mut parser, &mut event) == 0 {
                    failed = true;
                    break;
                }

                originals.push(copy_event(&event));
                let done = event.r#type == yaml_event_type_t::YAML_STREAM_END_EVENT;
                if yaml_emitter_emit(&mut emitter, &mut event) == 0 {
                    failed = true;
                    break;
                }
                if done {
                    break;
                }
            }

            yaml_parser_delete(&mut parser);
            yaml_emitter_delete(&mut emitter);

            if !failed {
                assert_eq!(yaml_parser_initialize(&mut parser), 1);
                yaml_parser_set_input_string(&mut parser, emitted.as_ptr(), written);
                for expected in &originals {
                    assert_eq!(yaml_parser_parse(&mut parser, &mut event), 1);
                    if !same_kind(expected, &event) {
                        failed = true;
                    }
                    yaml_event_delete(&mut event);
                    if failed {
                        break;
                    }
                }
                yaml_parser_delete(&mut parser);
            }

            if failed {
                eprintln!("FAILURE");
                if !parser.problem.is_null() {
                    eprintln!(
                        "{}",
                        CStr::from_ptr(parser.problem.cast()).to_string_lossy()
                    );
                }
            } else {
                println!("PASSED (length: {written})");
                print!("SOURCE:\n{}", String::from_utf8_lossy(&input));
                print!("#### (length: {})\n", input.len());
                print!("OUTPUT:\n{}", String::from_utf8_lossy(&emitted[..written]));
                print!("#### (length: {written})\n");
            }

            for mut original in originals {
                yaml_event_delete(&mut original);
            }
        }
    }
}

unsafe fn copy_event(event: &yaml_event_t) -> yaml_event_t {
    let mut copy = mem::zeroed::<yaml_event_t>();
    let ok = match event.r#type {
        yaml_event_type_t::YAML_STREAM_START_EVENT => {
            yaml_stream_start_event_initialize(&mut copy, event.data.stream_start.encoding)
        }
        yaml_event_type_t::YAML_STREAM_END_EVENT => yaml_stream_end_event_initialize(&mut copy),
        yaml_event_type_t::YAML_DOCUMENT_START_EVENT => yaml_document_start_event_initialize(
            &mut copy,
            event.data.document_start.version_directive,
            event.data.document_start.tag_directives.start,
            event.data.document_start.tag_directives.end,
            event.data.document_start.implicit,
        ),
        yaml_event_type_t::YAML_DOCUMENT_END_EVENT => {
            yaml_document_end_event_initialize(&mut copy, event.data.document_end.implicit)
        }
        yaml_event_type_t::YAML_ALIAS_EVENT => {
            yaml_alias_event_initialize(&mut copy, event.data.alias.anchor)
        }
        yaml_event_type_t::YAML_SCALAR_EVENT => yaml_scalar_event_initialize(
            &mut copy,
            event.data.scalar.anchor,
            event.data.scalar.tag,
            event.data.scalar.value,
            event.data.scalar.length as i32,
            event.data.scalar.plain_implicit,
            event.data.scalar.quoted_implicit,
            event.data.scalar.style,
        ),
        yaml_event_type_t::YAML_SEQUENCE_START_EVENT => yaml_sequence_start_event_initialize(
            &mut copy,
            event.data.sequence_start.anchor,
            event.data.sequence_start.tag,
            event.data.sequence_start.implicit,
            event.data.sequence_start.style,
        ),
        yaml_event_type_t::YAML_SEQUENCE_END_EVENT => yaml_sequence_end_event_initialize(&mut copy),
        yaml_event_type_t::YAML_MAPPING_START_EVENT => yaml_mapping_start_event_initialize(
            &mut copy,
            event.data.mapping_start.anchor,
            event.data.mapping_start.tag,
            event.data.mapping_start.implicit,
            event.data.mapping_start.style,
        ),
        yaml_event_type_t::YAML_MAPPING_END_EVENT => yaml_mapping_end_event_initialize(&mut copy),
        yaml_event_type_t::YAML_NO_EVENT => 0,
    };
    assert_eq!(ok, 1);
    copy
}

unsafe fn same_kind(lhs: &yaml_event_t, rhs: &yaml_event_t) -> bool {
    if lhs.r#type != rhs.r#type {
        return false;
    }

    match lhs.r#type {
        yaml_event_type_t::YAML_ALIAS_EVENT => {
            CStr::from_ptr(lhs.data.alias.anchor.cast())
                == CStr::from_ptr(rhs.data.alias.anchor.cast())
        }
        yaml_event_type_t::YAML_SCALAR_EVENT => {
            lhs.data.scalar.length == rhs.data.scalar.length
                && lhs.data.scalar.plain_implicit == rhs.data.scalar.plain_implicit
                && lhs.data.scalar.quoted_implicit == rhs.data.scalar.quoted_implicit
        }
        yaml_event_type_t::YAML_SEQUENCE_START_EVENT => {
            lhs.data.sequence_start.implicit == rhs.data.sequence_start.implicit
        }
        yaml_event_type_t::YAML_MAPPING_START_EVENT => {
            lhs.data.mapping_start.implicit == rhs.data.mapping_start.implicit
        }
        _ => true,
    }
}
