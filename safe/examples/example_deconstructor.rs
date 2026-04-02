use std::env;
use std::ffi::{c_void, CStr};
use std::fs;
use std::io::{self, Read, Write};
use std::mem;
use std::process;
use std::ptr;
use std::slice;

use yaml::{
    yaml_document_end_event_initialize, yaml_document_start_event_initialize, yaml_emitter_delete,
    yaml_emitter_emit, yaml_emitter_initialize, yaml_emitter_set_canonical,
    yaml_emitter_set_output, yaml_emitter_set_unicode, yaml_emitter_t, yaml_event_delete,
    yaml_event_t, yaml_mapping_end_event_initialize, yaml_mapping_start_event_initialize,
    yaml_mapping_style_t, yaml_parser_delete, yaml_parser_initialize, yaml_parser_parse,
    yaml_parser_set_input_string, yaml_parser_t, yaml_scalar_event_initialize, yaml_scalar_style_t,
    yaml_sequence_end_event_initialize, yaml_sequence_start_event_initialize,
    yaml_sequence_style_t, yaml_stream_end_event_initialize, yaml_stream_start_event_initialize,
};

const STR_TAG: &[u8] = b"tag:yaml.org,2002:str\0";
const MAP_TAG: &[u8] = b"tag:yaml.org,2002:map\0";
const SEQ_TAG: &[u8] = b"tag:yaml.org,2002:seq\0";

fn main() {
    let mut canonical = 0;
    let mut unicode = 0;
    let mut input_path = None;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                eprintln!("Usage: example_deconstructor [--canonical] [--unicode] [input.yaml]");
                process::exit(0);
            }
            "-c" | "--canonical" => canonical = 1,
            "-u" | "--unicode" => unicode = 1,
            _ => input_path = Some(arg),
        }
    }

    let input = match input_path {
        Some(path) => fs::read(&path).unwrap_or_else(|error| {
            panic!("failed to read {path}: {error}");
        }),
        None => {
            let mut buffer = Vec::new();
            io::stdin()
                .read_to_end(&mut buffer)
                .expect("failed to read stdin");
            buffer
        }
    };

    unsafe {
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut emitter = mem::zeroed::<yaml_emitter_t>();
        let mut input_event = mem::zeroed::<yaml_event_t>();
        let mut output_event = mem::zeroed::<yaml_event_t>();
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

        emit_ok(
            &mut emitter,
            yaml_stream_start_event_initialize(
                &mut output_event,
                yaml::yaml_encoding_t::YAML_UTF8_ENCODING,
            ),
            &mut output_event,
        );
        emit_ok(
            &mut emitter,
            yaml_document_start_event_initialize(
                &mut output_event,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                0,
            ),
            &mut output_event,
        );
        emit_ok(
            &mut emitter,
            yaml_sequence_start_event_initialize(
                &mut output_event,
                ptr::null(),
                SEQ_TAG.as_ptr(),
                1,
                yaml_sequence_style_t::YAML_BLOCK_SEQUENCE_STYLE,
            ),
            &mut output_event,
        );

        loop {
            if yaml_parser_parse(&mut parser, &mut input_event) == 0 {
                eprintln!("Parse error: {}", cstr(parser.problem));
                process::exit(1);
            }

            emit_mapping_start(&mut emitter, &mut output_event);
            emit_pair(
                &mut emitter,
                &mut output_event,
                b"type",
                event_name(&input_event),
            );

            match input_event.r#type {
                yaml::yaml_event_type_t::YAML_ALIAS_EVENT => {
                    emit_pair(
                        &mut emitter,
                        &mut output_event,
                        b"anchor",
                        cbytes(input_event.data.alias.anchor),
                    );
                }
                yaml::yaml_event_type_t::YAML_SCALAR_EVENT => {
                    emit_pair(
                        &mut emitter,
                        &mut output_event,
                        b"value",
                        slice::from_raw_parts(
                            input_event.data.scalar.value,
                            input_event.data.scalar.length,
                        ),
                    );
                }
                _ => {}
            }

            emit_ok(
                &mut emitter,
                yaml_mapping_end_event_initialize(&mut output_event),
                &mut output_event,
            );

            let done = input_event.r#type == yaml::yaml_event_type_t::YAML_STREAM_END_EVENT;
            yaml_event_delete(&mut input_event);
            if done {
                break;
            }
        }

        emit_ok(
            &mut emitter,
            yaml_sequence_end_event_initialize(&mut output_event),
            &mut output_event,
        );
        emit_ok(
            &mut emitter,
            yaml_document_end_event_initialize(&mut output_event, 0),
            &mut output_event,
        );
        emit_ok(
            &mut emitter,
            yaml_stream_end_event_initialize(&mut output_event),
            &mut output_event,
        );

        yaml_parser_delete(&mut parser);
        yaml_emitter_delete(&mut emitter);
        stdout.flush().expect("failed to flush stdout");
    }
}

unsafe fn emit_mapping_start(emitter: *mut yaml_emitter_t, event: *mut yaml_event_t) {
    emit_ok(
        emitter,
        yaml_mapping_start_event_initialize(
            event,
            ptr::null(),
            MAP_TAG.as_ptr(),
            1,
            yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE,
        ),
        event,
    );
}

unsafe fn emit_pair(
    emitter: *mut yaml_emitter_t,
    event: *mut yaml_event_t,
    key: &[u8],
    value: &[u8],
) {
    emit_ok(
        emitter,
        yaml_scalar_event_initialize(
            event,
            ptr::null(),
            STR_TAG.as_ptr(),
            key.as_ptr(),
            key.len() as i32,
            1,
            1,
            yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
        ),
        event,
    );
    emit_ok(
        emitter,
        yaml_scalar_event_initialize(
            event,
            ptr::null(),
            STR_TAG.as_ptr(),
            value.as_ptr(),
            value.len() as i32,
            1,
            1,
            yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
        ),
        event,
    );
}

unsafe fn emit_ok(emitter: *mut yaml_emitter_t, init_ok: i32, event: *mut yaml_event_t) {
    if init_ok == 0 {
        eprintln!("Memory error while creating an event");
        process::exit(1);
    }
    if yaml_emitter_emit(emitter, event) == 0 {
        eprintln!("Emitter error: {}", cstr((*emitter).problem));
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
