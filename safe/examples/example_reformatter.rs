use std::env;
use std::ffi::{c_void, CStr};
use std::fs;
use std::io::{self, Read, Write};
use std::mem;
use std::process;
use std::slice;

use yaml::{
    yaml_emitter_delete, yaml_emitter_emit, yaml_emitter_initialize, yaml_emitter_set_canonical,
    yaml_emitter_set_output, yaml_emitter_set_unicode, yaml_emitter_t, yaml_event_t,
    yaml_parser_delete, yaml_parser_initialize, yaml_parser_parse, yaml_parser_set_input_string,
    yaml_parser_t,
};

fn main() {
    let mut canonical = 0;
    let mut unicode = 0;
    let mut input_path = None;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                eprintln!("Usage: example_reformatter [--canonical] [--unicode] [input.yaml]");
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
        let mut event = mem::zeroed::<yaml_event_t>();
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

        loop {
            if yaml_parser_parse(&mut parser, &mut event) == 0 {
                eprintln!("Parse error: {}", cstr(parser.problem));
                yaml_parser_delete(&mut parser);
                yaml_emitter_delete(&mut emitter);
                process::exit(1);
            }
            let done = event.r#type == yaml::yaml_event_type_t::YAML_STREAM_END_EVENT;
            if yaml_emitter_emit(&mut emitter, &mut event) == 0 {
                eprintln!("Emitter error: {}", cstr(emitter.problem));
                yaml_parser_delete(&mut parser);
                yaml_emitter_delete(&mut emitter);
                process::exit(1);
            }
            if done {
                break;
            }
        }

        yaml_parser_delete(&mut parser);
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
