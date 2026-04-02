use std::ffi::CStr;
use std::fs;
use std::mem;
use std::path::PathBuf;

use yaml::{
    yaml_error_type_t, yaml_parser_delete, yaml_parser_initialize, yaml_parser_scan,
    yaml_parser_set_input_string, yaml_parser_t, yaml_token_delete, yaml_token_t,
    yaml_token_type_t,
};

struct ScanOutcome {
    ok: bool,
    error: yaml_error_type_t,
    problem: Option<String>,
    line: usize,
    column: usize,
    token_count: usize,
}

#[test]
fn cve_2014_9130_wrapped_scalar_trigger_returns_scanner_error_instead_of_aborting() {
    let trigger = b"a: \"\\n\"     b: true\n";
    let outcome = scan_all(trigger);

    assert!(!outcome.ok);
    assert_eq!(outcome.error, yaml_error_type_t::YAML_SCANNER_ERROR);
    assert_eq!(
        outcome.problem.as_deref(),
        Some("mapping values are not allowed in this context")
    );
    assert_eq!(outcome.line, 0);
    assert_eq!(outcome.column, 13);
    assert_eq!(outcome.token_count, 7);
}

#[test]
fn clusterfuzz_fixture_returns_recoverable_scanner_error() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = fs::read(
        manifest_dir
            .join("compat/regression-inputs/clusterfuzz-testcase-minimized-5607885063061504.yml"),
    )
    .expect("failed to read clusterfuzz regression fixture");

    let outcome = scan_all(&fixture);
    assert!(!outcome.ok);
    assert_eq!(outcome.error, yaml_error_type_t::YAML_SCANNER_ERROR);
    assert_eq!(
        outcome.problem.as_deref(),
        Some("found unexpected end of stream")
    );
    assert_eq!(outcome.line, 1);
    assert_eq!(outcome.column, 0);
    assert_eq!(outcome.token_count, 1);
}

fn scan_all(input: &[u8]) -> ScanOutcome {
    let mut parser = unsafe { mem::zeroed::<yaml_parser_t>() };
    unsafe {
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());

        let mut token_count = 0usize;
        let mut ok = true;
        loop {
            let mut token = mem::zeroed::<yaml_token_t>();
            if yaml_parser_scan(&mut parser, &mut token) == 0 {
                ok = false;
                break;
            }
            token_count += 1;
            let done = token.r#type == yaml_token_type_t::YAML_STREAM_END_TOKEN;
            yaml_token_delete(&mut token);
            if done {
                break;
            }
        }

        let problem = if parser.problem.is_null() {
            None
        } else {
            Some(
                CStr::from_ptr(parser.problem)
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        let outcome = ScanOutcome {
            ok,
            error: parser.error,
            problem,
            line: parser.problem_mark.line,
            column: parser.problem_mark.column,
            token_count,
        };
        yaml_parser_delete(&mut parser);
        outcome
    }
}
