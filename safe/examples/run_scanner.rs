use std::env;
use std::fs;
use std::mem;
use std::process::ExitCode;

use yaml::{
    yaml_parser_delete, yaml_parser_initialize, yaml_parser_scan, yaml_parser_set_input_string,
    yaml_parser_t, yaml_token_delete, yaml_token_t, yaml_token_type_t,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} file1.yaml ...", args[0]);
        return ExitCode::SUCCESS;
    }

    let mut overall_ok = true;

    for (index, path) in args.iter().enumerate().skip(1) {
        let input = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                println!("[{index}] Scanning '{path}': FAILURE ({error})");
                overall_ok = false;
                continue;
            }
        };

        print!("[{index}] Scanning '{path}': ");

        let mut parser = unsafe { mem::zeroed::<yaml_parser_t>() };
        let initialized = unsafe { yaml_parser_initialize(&mut parser) };
        if initialized == 0 {
            println!("FAILURE (parser initialization failed)");
            overall_ok = false;
            continue;
        }

        unsafe {
            yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());
        }

        let mut done = false;
        let mut count = 0usize;
        let mut error = false;

        while !done {
            let mut token = unsafe { mem::zeroed::<yaml_token_t>() };
            let scanned = unsafe { yaml_parser_scan(&mut parser, &mut token) };
            if scanned == 0 {
                error = true;
                break;
            }

            done = token.r#type == yaml_token_type_t::YAML_STREAM_END_TOKEN;
            unsafe {
                yaml_token_delete(&mut token);
            }
            count += 1;
        }

        unsafe {
            yaml_parser_delete(&mut parser);
        }

        if error {
            println!("FAILURE ({count} tokens)");
            overall_ok = false;
        } else {
            println!("SUCCESS ({count} tokens)");
        }
    }

    if overall_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
