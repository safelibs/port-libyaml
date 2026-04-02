use std::env;
use std::ffi::{c_void, CStr};
use std::fs;
use std::io::{self, BufRead, Write};
use std::mem;
use std::process;
use std::ptr;
use std::slice;

use yaml::{
    yaml_alias_event_initialize, yaml_document_end_event_initialize,
    yaml_document_start_event_initialize, yaml_emitter_delete, yaml_emitter_emit,
    yaml_emitter_initialize, yaml_emitter_set_output, yaml_emitter_set_unicode, yaml_emitter_t,
    yaml_event_t, yaml_mapping_end_event_initialize, yaml_mapping_start_event_initialize,
    yaml_mapping_style_t, yaml_scalar_event_initialize, yaml_scalar_style_t,
    yaml_sequence_end_event_initialize, yaml_sequence_start_event_initialize,
    yaml_sequence_style_t, yaml_stream_end_event_initialize, yaml_stream_start_event_initialize,
    yaml_tag_directive_t, yaml_version_directive_t,
};

fn main() {
    let mut args = env::args().skip(1);
    let input = match args.next() {
        Some(path) if path == "-h" || path == "--help" => {
            eprintln!("Usage: run_emitter_test_suite [event-stream.txt]");
            process::exit(0);
        }
        Some(path) => fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("failed to read {path}: {error}");
        }),
        None => {
            let stdin = io::stdin();
            let mut input = String::new();
            for line in stdin.lock().lines() {
                input.push_str(&line.expect("failed to read stdin line"));
                input.push('\n');
            }
            input
        }
    };

    unsafe {
        let mut emitter = mem::zeroed::<yaml_emitter_t>();
        let mut stdout = io::stdout().lock();
        if yaml_emitter_initialize(&mut emitter) == 0 {
            eprintln!("Could not initialize the emitter object");
            process::exit(1);
        }

        yaml_emitter_set_output(
            &mut emitter,
            Some(write_to_stdout),
            (&mut stdout as *mut _ as *mut c_void).cast(),
        );
        yaml_emitter_set_unicode(&mut emitter, 0);

        for raw_line in input.lines() {
            let line = raw_line.trim_end();
            if line.is_empty() {
                continue;
            }
            let mut event = mem::zeroed::<yaml_event_t>();
            let mut anchor = [0u8; 256];
            let mut tag = [0u8; 256];

            let ok = if line.starts_with("+STR") {
                yaml_stream_start_event_initialize(
                    &mut event,
                    yaml::yaml_encoding_t::YAML_UTF8_ENCODING,
                )
            } else if line.starts_with("-STR") {
                yaml_stream_end_event_initialize(&mut event)
            } else if line.starts_with("+DOC") {
                let implicit = (!line[4..].starts_with(" ---")) as i32;
                yaml_document_start_event_initialize(
                    &mut event,
                    ptr::null_mut::<yaml_version_directive_t>(),
                    ptr::null_mut::<yaml_tag_directive_t>(),
                    ptr::null_mut::<yaml_tag_directive_t>(),
                    implicit,
                )
            } else if line.starts_with("-DOC") {
                let implicit = (!line[4..].starts_with(" ...")) as i32;
                yaml_document_end_event_initialize(&mut event, implicit)
            } else if line.starts_with("+MAP") {
                yaml_mapping_start_event_initialize(
                    &mut event,
                    get_anchor('&', line, &mut anchor),
                    get_tag(line, &mut tag),
                    0,
                    yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE,
                )
            } else if line.starts_with("-MAP") {
                yaml_mapping_end_event_initialize(&mut event)
            } else if line.starts_with("+SEQ") {
                yaml_sequence_start_event_initialize(
                    &mut event,
                    get_anchor('&', line, &mut anchor),
                    get_tag(line, &mut tag),
                    0,
                    yaml_sequence_style_t::YAML_BLOCK_SEQUENCE_STYLE,
                )
            } else if line.starts_with("-SEQ") {
                yaml_sequence_end_event_initialize(&mut event)
            } else if line.starts_with("=VAL") {
                let (value, style) = get_value(line);
                let implicit = get_tag(line, &mut tag).is_null() as i32;
                yaml_scalar_event_initialize(
                    &mut event,
                    get_anchor('&', line, &mut anchor),
                    get_tag(line, &mut tag),
                    value.as_ptr(),
                    value.len() as i32,
                    implicit,
                    implicit,
                    style,
                )
            } else if line.starts_with("=ALI") {
                yaml_alias_event_initialize(&mut event, get_anchor('*', line, &mut anchor))
            } else {
                eprintln!("Unknown event: '{line}'");
                yaml_emitter_delete(&mut emitter);
                process::exit(1);
            };

            if ok == 0 {
                eprintln!("Memory error while creating an event");
                yaml_emitter_delete(&mut emitter);
                process::exit(1);
            }

            if yaml_emitter_emit(&mut emitter, &mut event) == 0 {
                eprintln!(
                    "{}: {}",
                    emitter_error_name(&emitter),
                    cstr(emitter.problem)
                );
                yaml_emitter_delete(&mut emitter);
                process::exit(1);
            }
        }

        yaml_emitter_delete(&mut emitter);
        stdout.flush().expect("failed to flush stdout");
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

fn emitter_error_name(emitter: &yaml_emitter_t) -> &'static str {
    match emitter.error {
        yaml::yaml_error_type_t::YAML_MEMORY_ERROR => "Memory error",
        yaml::yaml_error_type_t::YAML_WRITER_ERROR => "Writer error",
        yaml::yaml_error_type_t::YAML_EMITTER_ERROR => "Emitter error",
        _ => "Internal error",
    }
}

fn get_anchor(sigil: char, line: &str, buffer: &mut [u8; 256]) -> *mut u8 {
    let Some(start) = line.find(sigil) else {
        return ptr::null_mut();
    };
    let value = &line[start + 1..];
    let end = value.find(' ').unwrap_or(value.len());
    buffer[..end].copy_from_slice(&value.as_bytes()[..end]);
    buffer[end] = 0;
    buffer.as_mut_ptr()
}

fn get_tag(line: &str, buffer: &mut [u8; 256]) -> *mut u8 {
    let Some(start) = line.find('<') else {
        return ptr::null_mut();
    };
    let Some(end) = line[start + 1..].find('>') else {
        return ptr::null_mut();
    };
    let value = &line[start + 1..start + 1 + end];
    buffer[..value.len()].copy_from_slice(value.as_bytes());
    buffer[value.len()] = 0;
    buffer.as_mut_ptr()
}

fn get_value(line: &str) -> (Vec<u8>, yaml_scalar_style_t) {
    for (index, ch) in line.as_bytes().iter().enumerate().skip(4) {
        if *ch != b' ' {
            continue;
        }
        let start = index + 1;
        let (style, offset) = match line.as_bytes().get(start).copied() {
            Some(b':') => (yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE, 1),
            Some(b'\'') => (yaml_scalar_style_t::YAML_SINGLE_QUOTED_SCALAR_STYLE, 1),
            Some(b'"') => (yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE, 1),
            Some(b'|') => (yaml_scalar_style_t::YAML_LITERAL_SCALAR_STYLE, 1),
            Some(b'>') => (yaml_scalar_style_t::YAML_FOLDED_SCALAR_STYLE, 1),
            _ => continue,
        };
        return (line.as_bytes()[start + offset..].to_vec(), style);
    }

    unreachable!("value marker not found in event line: {line}");
}
