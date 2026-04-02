use std::env;
use std::ffi::CStr;
use std::fs;
use std::mem;

use yaml::{yaml_event_delete, yaml_parser_delete, yaml_parser_initialize, yaml_parser_parse};
use yaml::{yaml_event_t, yaml_event_type_t, yaml_parser_set_input_string, yaml_parser_t};

fn main() {
    let files: Vec<String> = env::args().skip(1).collect();
    if files.is_empty() {
        println!(
            "Usage: {} file1.yaml ...",
            env::args()
                .next()
                .unwrap_or_else(|| String::from("run_parser"))
        );
        return;
    }

    for (index, path) in files.iter().enumerate() {
        let input = fs::read(path).unwrap_or_else(|error| {
            panic!("failed to read {path}: {error}");
        });

        unsafe {
            let mut parser = mem::zeroed::<yaml_parser_t>();
            let mut event = mem::zeroed::<yaml_event_t>();
            let mut done = false;
            let mut count = 0;
            let mut error = false;

            print!("[{}] Parsing '{}': ", index + 1, path);
            assert_eq!(yaml_parser_initialize(&mut parser), 1);
            yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());

            while !done {
                if yaml_parser_parse(&mut parser, &mut event) == 0 {
                    error = true;
                    break;
                }

                done = event.r#type == yaml_event_type_t::YAML_STREAM_END_EVENT;
                yaml_event_delete(&mut event);
                count += 1;
            }

            if error && !parser.problem.is_null() {
                eprintln!("{}", CStr::from_ptr(parser.problem).to_string_lossy());
            }

            yaml_parser_delete(&mut parser);
            println!(
                "{} ({count} events)",
                if error { "FAILURE" } else { "SUCCESS" }
            );
        }
    }
}
