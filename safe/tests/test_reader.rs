use std::ffi::CStr;
use std::mem;
use std::ptr;
use std::slice;

use yaml::{
    yaml_encoding_t, yaml_error_type_t, yaml_parser_delete, yaml_parser_initialize,
    yaml_parser_set_encoding, yaml_parser_set_input, yaml_parser_set_input_string, yaml_parser_t,
    yaml_parser_update_buffer, MAX_FILE_SIZE,
};

#[repr(C)]
struct MemoryReader {
    input: *const u8,
    size: usize,
    offset: usize,
    chunk: usize,
}

struct ReaderOutcome {
    ok: bool,
    error: yaml_error_type_t,
    problem: Option<String>,
    decoded: Vec<u8>,
}

unsafe extern "C" fn memory_read_handler(
    data: *mut core::ffi::c_void,
    buffer: *mut u8,
    size: usize,
    size_read: *mut usize,
) -> i32 {
    let reader = &mut *(data.cast::<MemoryReader>());
    let remaining = reader.size.saturating_sub(reader.offset);
    if remaining == 0 {
        *size_read = 0;
        return 1;
    }

    let limit = if reader.chunk == 0 {
        size
    } else {
        size.min(reader.chunk)
    };
    let count = limit.min(remaining);
    ptr::copy_nonoverlapping(reader.input.add(reader.offset), buffer, count);
    reader.offset += count;
    *size_read = count;
    1
}

#[test]
fn utf8_validation_matrix_matches_reader_accept_reject_rules() {
    let cases: &[(&str, &[u8], bool)] = &[
        (
            "simple utf8",
            b"'test' is '\xD0\xBF\xD1\x80\xD0\xBE\xD0\xB2\xD0\xB5\xD1\x80\xD0\xBA\xD0\xB0' in Russian!",
            true,
        ),
        ("empty line", b"!", true),
        ("u0000 control", b"\x00!", false),
        ("u0080 control", b"\xC2\x80!", false),
        ("u0800 valid", b"\xE0\xA0\x80!", true),
        ("u10000 valid", b"\xF0\x90\x80\x80!", true),
        ("five-byte sequence", b"\xF8\x88\x80\x80\x80!", false),
        ("six-byte sequence", b"\xFC\x84\x80\x80\x80\x80!", false),
        ("u007f control", b"\x7F!", false),
        ("u07ff valid", b"\xDF\xBF!", true),
        ("uFFFF control", b"\xEF\xBF\xBF!", false),
        ("u110000 invalid", b"\xF4\x90\x80\x80!", false),
        ("first continuation byte", b"\x80!", false),
        ("missing trailing byte", b"\xDF!", false),
        ("impossible bytes", b"\xFE|\xFF|\xFE\xFE\xFF\xFF!", false),
        ("overlong nul", b"\xC0\x80!", false),
        ("surrogate", b"\xED\xA0\x80!", false),
        ("uD7FF valid", b"\xED\x9F\xBF!", true),
        ("uE000 valid", b"\xEE\x80\x80!", true),
        ("uFFFD valid", b"\xEF\xBF\xBD!", true),
        ("u10FFFF valid", b"\xF4\x8F\xBF\xBF!", true),
        ("illegal code position", b"\xEF\xBF\xBE!", false),
    ];

    for (title, input, expect_success) in cases {
        let outcome = read_from_string(input, None);
        assert_eq!(
            outcome.ok, *expect_success,
            "{title}: unexpected reader outcome: error={:?} problem={:?}",
            outcome.error, outcome.problem
        );
        if *expect_success {
            assert_eq!(outcome.error, yaml_error_type_t::YAML_NO_ERROR, "{title}");
        } else {
            assert_eq!(
                outcome.error,
                yaml_error_type_t::YAML_READER_ERROR,
                "{title}"
            );
            assert!(
                outcome.problem.is_some(),
                "{title}: missing reader problem description"
            );
        }
    }
}

#[test]
fn bom_detection_and_utf16_decoding_match_upstream_examples() {
    let expected = b"Hi is \xD0\x9F\xD1\x80\xD0\xB8\xD0\xB2\xD0\xB5\xD1\x82";
    let cases: &[(&str, &[u8])] = &[
        (
            "no bom utf8",
            b"Hi is \xD0\x9F\xD1\x80\xD0\xB8\xD0\xB2\xD0\xB5\xD1\x82",
        ),
        (
            "utf8 bom",
            b"\xEF\xBB\xBFHi is \xD0\x9F\xD1\x80\xD0\xB8\xD0\xB2\xD0\xB5\xD1\x82",
        ),
        (
            "utf16le bom",
            b"\xFF\xFEH\x00i\x00 \x00i\x00s\x00 \x00\x1F\x04@\x048\x042\x045\x04B\x04",
        ),
        (
            "utf16be bom",
            b"\xFE\xFF\x00H\x00i\x00 \x00i\x00s\x00 \x04\x1F\x04@\x048\x042\x045\x04B",
        ),
    ];

    for (title, input) in cases {
        let outcome = read_from_string(input, None);
        assert!(outcome.ok, "{title}: {:?}", outcome.problem);
        assert_eq!(outcome.decoded, expected, "{title}");
    }
}

#[test]
fn generic_input_handler_with_explicit_encoding_decodes_utf16le_without_bom() {
    let input = [
        b'H', 0x00, b'i', 0x00, b' ', 0x00, b'i', 0x00, b's', 0x00, b' ', 0x00, 0x1F, 0x04, 0x40,
        0x04, 0x38, 0x04, 0x32, 0x04, 0x35, 0x04, 0x42, 0x04,
    ];
    let outcome =
        read_with_generic_handler(&input, 1, Some(yaml_encoding_t::YAML_UTF16LE_ENCODING));
    assert!(outcome.ok, "{:?}", outcome.problem);
    assert_eq!(
        outcome.decoded,
        b"Hi is \xD0\x9F\xD1\x80\xD0\xB8\xD0\xB2\xD0\xB5\xD1\x82"
    );
}

#[test]
fn oversized_input_guard_reports_exact_reader_error_fields() {
    let mut parser = unsafe { mem::zeroed::<yaml_parser_t>() };
    unsafe {
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input_string(&mut parser, b"a".as_ptr(), 1);
        parser.offset = MAX_FILE_SIZE;
        assert_eq!(yaml_parser_update_buffer(&mut parser, 1), 0);
        assert_eq!(parser.error, yaml_error_type_t::YAML_READER_ERROR);
        assert_eq!(CStr::from_ptr(parser.problem), c"input is too long");
        assert_eq!(parser.problem_offset, MAX_FILE_SIZE);
        assert_eq!(parser.problem_value, -1);
        yaml_parser_delete(&mut parser);
    }
}

fn read_from_string(input: &[u8], encoding: Option<yaml_encoding_t>) -> ReaderOutcome {
    let mut parser = unsafe { mem::zeroed::<yaml_parser_t>() };
    unsafe {
        let initialized = yaml_parser_initialize(&mut parser);
        assert_eq!(initialized, 1);
        yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());
        if let Some(encoding) = encoding {
            yaml_parser_set_encoding(&mut parser, encoding);
        }
        let ok = update_all(&mut parser);
        let outcome = snapshot(&parser, ok);
        yaml_parser_delete(&mut parser);
        outcome
    }
}

fn read_with_generic_handler(
    input: &[u8],
    chunk: usize,
    encoding: Option<yaml_encoding_t>,
) -> ReaderOutcome {
    let mut parser = unsafe { mem::zeroed::<yaml_parser_t>() };
    let mut reader = MemoryReader {
        input: input.as_ptr(),
        size: input.len(),
        offset: 0,
        chunk,
    };

    unsafe {
        let initialized = yaml_parser_initialize(&mut parser);
        assert_eq!(initialized, 1);
        yaml_parser_set_input(
            &mut parser,
            Some(memory_read_handler),
            (&mut reader as *mut MemoryReader).cast(),
        );
        if let Some(encoding) = encoding {
            yaml_parser_set_encoding(&mut parser, encoding);
        }
        let ok = update_all(&mut parser);
        let outcome = snapshot(&parser, ok);
        yaml_parser_delete(&mut parser);
        outcome
    }
}

unsafe fn update_all(parser: *mut yaml_parser_t) -> bool {
    loop {
        let target = (*parser).unread.saturating_add(1);
        if yaml_parser_update_buffer(parser, target) == 0 {
            return false;
        }
        if (*parser).eof != 0 && (*parser).raw_buffer.pointer == (*parser).raw_buffer.last {
            return true;
        }
    }
}

unsafe fn snapshot(parser: &yaml_parser_t, ok: bool) -> ReaderOutcome {
    let problem = if parser.problem.is_null() {
        None
    } else {
        Some(
            CStr::from_ptr(parser.problem)
                .to_string_lossy()
                .into_owned(),
        )
    };

    let decoded = if ok {
        let len = parser.buffer.last as usize - parser.buffer.pointer as usize;
        let slice = slice::from_raw_parts(parser.buffer.pointer, len);
        let content = if slice.last() == Some(&0) {
            &slice[..slice.len().saturating_sub(1)]
        } else {
            slice
        };
        content.to_vec()
    } else {
        Vec::new()
    };

    ReaderOutcome {
        ok,
        error: parser.error,
        problem,
        decoded,
    }
}
