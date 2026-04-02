use std::env;
use std::ffi::CStr;
use std::fs;
use std::io::{self, Read, Write};
use std::mem;
use std::process;
use std::slice;

use yaml::{
    yaml_event_delete, yaml_event_t, yaml_event_type_t, yaml_mapping_style_t, yaml_parser_delete,
    yaml_parser_initialize, yaml_parser_parse, yaml_parser_set_input_string, yaml_parser_t,
    yaml_scalar_style_t, yaml_sequence_style_t,
};

fn main() {
    let mut flow = -1;
    let mut input_path: Option<String> = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--flow" => {
                let Some(value) = args.next() else {
                    process::exit(usage(1));
                };
                flow = match value.as_str() {
                    "keep" => 0,
                    "on" => 1,
                    "off" => -1,
                    _ => process::exit(usage(1)),
                };
            }
            "--help" | "-h" => process::exit(usage(0)),
            _ => {
                if input_path.is_some() {
                    process::exit(usage(1));
                }
                input_path = Some(arg);
            }
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
        let mut event = mem::zeroed::<yaml_event_t>();
        let mut stdout = io::stdout().lock();

        if yaml_parser_initialize(&mut parser) == 0 {
            eprintln!("Could not initialize the parser object");
            process::exit(1);
        }
        yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());

        loop {
            if yaml_parser_parse(&mut parser, &mut event) == 0 {
                if parser.problem_mark.line != 0 || parser.problem_mark.column != 0 {
                    eprintln!(
                        "Parse error: {}\nLine: {} Column: {}",
                        cstr(parser.problem),
                        parser.problem_mark.line + 1,
                        parser.problem_mark.column + 1
                    );
                } else {
                    eprintln!("Parse error: {}", cstr(parser.problem));
                }
                yaml_parser_delete(&mut parser);
                process::exit(1);
            }

            let event_type = event.r#type;
            match event_type {
                yaml_event_type_t::YAML_NO_EVENT => {
                    stdout.write_all(b"???\n").unwrap();
                }
                yaml_event_type_t::YAML_STREAM_START_EVENT => {
                    stdout.write_all(b"+STR\n").unwrap();
                }
                yaml_event_type_t::YAML_STREAM_END_EVENT => {
                    stdout.write_all(b"-STR\n").unwrap();
                }
                yaml_event_type_t::YAML_DOCUMENT_START_EVENT => {
                    stdout.write_all(b"+DOC").unwrap();
                    if event.data.document_start.implicit == 0 {
                        stdout.write_all(b" ---").unwrap();
                    }
                    stdout.write_all(b"\n").unwrap();
                }
                yaml_event_type_t::YAML_DOCUMENT_END_EVENT => {
                    stdout.write_all(b"-DOC").unwrap();
                    if event.data.document_end.implicit == 0 {
                        stdout.write_all(b" ...").unwrap();
                    }
                    stdout.write_all(b"\n").unwrap();
                }
                yaml_event_type_t::YAML_MAPPING_START_EVENT => {
                    stdout.write_all(b"+MAP").unwrap();
                    if (flow == 0
                        && event.data.mapping_start.style
                            == yaml_mapping_style_t::YAML_FLOW_MAPPING_STYLE)
                        || flow == 1
                    {
                        stdout.write_all(b" {}").unwrap();
                    }
                    if !event.data.mapping_start.anchor.is_null() {
                        stdout.write_all(b" &").unwrap();
                        stdout
                            .write_all(cstr_bytes(event.data.mapping_start.anchor))
                            .unwrap();
                    }
                    if !event.data.mapping_start.tag.is_null() {
                        stdout.write_all(b" <").unwrap();
                        stdout
                            .write_all(cstr_bytes(event.data.mapping_start.tag))
                            .unwrap();
                        stdout.write_all(b">").unwrap();
                    }
                    stdout.write_all(b"\n").unwrap();
                }
                yaml_event_type_t::YAML_MAPPING_END_EVENT => {
                    stdout.write_all(b"-MAP\n").unwrap();
                }
                yaml_event_type_t::YAML_SEQUENCE_START_EVENT => {
                    stdout.write_all(b"+SEQ").unwrap();
                    if (flow == 0
                        && event.data.sequence_start.style
                            == yaml_sequence_style_t::YAML_FLOW_SEQUENCE_STYLE)
                        || flow == 1
                    {
                        stdout.write_all(b" []").unwrap();
                    }
                    if !event.data.sequence_start.anchor.is_null() {
                        stdout.write_all(b" &").unwrap();
                        stdout
                            .write_all(cstr_bytes(event.data.sequence_start.anchor))
                            .unwrap();
                    }
                    if !event.data.sequence_start.tag.is_null() {
                        stdout.write_all(b" <").unwrap();
                        stdout
                            .write_all(cstr_bytes(event.data.sequence_start.tag))
                            .unwrap();
                        stdout.write_all(b">").unwrap();
                    }
                    stdout.write_all(b"\n").unwrap();
                }
                yaml_event_type_t::YAML_SEQUENCE_END_EVENT => {
                    stdout.write_all(b"-SEQ\n").unwrap();
                }
                yaml_event_type_t::YAML_SCALAR_EVENT => {
                    stdout.write_all(b"=VAL").unwrap();
                    if !event.data.scalar.anchor.is_null() {
                        stdout.write_all(b" &").unwrap();
                        stdout
                            .write_all(cstr_bytes(event.data.scalar.anchor))
                            .unwrap();
                    }
                    if !event.data.scalar.tag.is_null() {
                        stdout.write_all(b" <").unwrap();
                        stdout.write_all(cstr_bytes(event.data.scalar.tag)).unwrap();
                        stdout.write_all(b">").unwrap();
                    }
                    match event.data.scalar.style {
                        yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE => {
                            stdout.write_all(b" :").unwrap();
                        }
                        yaml_scalar_style_t::YAML_SINGLE_QUOTED_SCALAR_STYLE => {
                            stdout.write_all(b" '").unwrap();
                        }
                        yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE => {
                            stdout.write_all(b" \"").unwrap();
                        }
                        yaml_scalar_style_t::YAML_LITERAL_SCALAR_STYLE => {
                            stdout.write_all(b" |").unwrap();
                        }
                        yaml_scalar_style_t::YAML_FOLDED_SCALAR_STYLE => {
                            stdout.write_all(b" >").unwrap();
                        }
                        yaml_scalar_style_t::YAML_ANY_SCALAR_STYLE => {
                            unreachable!();
                        }
                    }
                    write_escaped(
                        &mut stdout,
                        event.data.scalar.value,
                        event.data.scalar.length,
                    );
                    stdout.write_all(b"\n").unwrap();
                }
                yaml_event_type_t::YAML_ALIAS_EVENT => {
                    stdout.write_all(b"=ALI *").unwrap();
                    stdout
                        .write_all(cstr_bytes(event.data.alias.anchor))
                        .unwrap();
                    stdout.write_all(b"\n").unwrap();
                }
            }

            yaml_event_delete(&mut event);
            if event_type == yaml_event_type_t::YAML_STREAM_END_EVENT {
                break;
            }
        }

        yaml_parser_delete(&mut parser);
        stdout.flush().unwrap();
    }
}

fn usage(ret: i32) -> i32 {
    eprintln!("Usage: libyaml-parser [--flow (on|off|keep)] [<input-file>]");
    ret
}

unsafe fn cstr(ptr: *const i8) -> String {
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

unsafe fn cstr_bytes(ptr: *const u8) -> &'static [u8] {
    CStr::from_ptr(ptr.cast()).to_bytes()
}

unsafe fn write_escaped(stdout: &mut impl Write, value: *const u8, length: usize) {
    let bytes = slice::from_raw_parts(value, length);
    for byte in bytes {
        match *byte {
            b'\\' => stdout.write_all(b"\\\\").unwrap(),
            b'\0' => stdout.write_all(b"\\0").unwrap(),
            0x08 => stdout.write_all(b"\\b").unwrap(),
            b'\n' => stdout.write_all(b"\\n").unwrap(),
            b'\r' => stdout.write_all(b"\\r").unwrap(),
            b'\t' => stdout.write_all(b"\\t").unwrap(),
            value => stdout.write_all(&[value]).unwrap(),
        }
    }
}
