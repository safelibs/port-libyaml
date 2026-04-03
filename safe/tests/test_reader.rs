use std::ffi::CStr;
use std::mem;
use std::ptr;
use std::slice;

use yaml::{
    yaml_document_delete, yaml_document_get_root_node, yaml_document_t, yaml_encoding_t,
    yaml_error_type_t, yaml_node_type_t, yaml_parser_delete, yaml_parser_initialize,
    yaml_parser_load, yaml_parser_set_encoding, yaml_parser_set_input,
    yaml_parser_set_input_string, yaml_parser_t, yaml_parser_update_buffer, MAX_FILE_SIZE,
};

macro_rules! cstr {
    ($value:literal) => {
        unsafe { ::std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($value, "\0").as_bytes()) }
    };
}

const BOM_ORIGINAL: &[u8] = b"Hi is \xD0\x9F\xD1\x80\xD0\xB8\xD0\xB2\xD0\xB5\xD1\x82";
const UTF16LE_GREETING_NO_BOM: &[u8] = &[
    b'H', 0x00, b'i', 0x00, b' ', 0x00, b'i', 0x00, b's', 0x00, b' ', 0x00, 0x1f, 0x04, 0x40, 0x04,
    0x38, 0x04, 0x32, 0x04, 0x35, 0x04, 0x42, 0x04,
];
const LONG: usize = 100_000;

#[repr(C)]
struct MemoryReader {
    input: *const u8,
    size: usize,
    offset: usize,
    chunk: usize,
}

unsafe extern "C" fn memory_read_handler(
    data: *mut core::ffi::c_void,
    buffer: *mut u8,
    size: usize,
    size_read: *mut usize,
) -> i32 {
    let reader = &mut *data.cast::<MemoryReader>();
    let remaining = reader.size.saturating_sub(reader.offset);
    if remaining == 0 {
        *size_read = 0;
        return 1;
    }

    let mut limit = size;
    if reader.chunk != 0 && limit > reader.chunk {
        limit = reader.chunk;
    }
    let count = limit.min(remaining);
    ptr::copy_nonoverlapping(reader.input.add(reader.offset), buffer, count);
    reader.offset += count;
    *size_read = count;
    1
}

#[test]
fn utf8_sequence_matrix_matches_vendored_reader_suite() {
    let cases: &[(&str, &[u8], bool)] = &[
        (
            "a simple test",
            b"'test' is '\xD0\xBF\xD1\x80\xD0\xBE\xD0\xB2\xD0\xB5\xD1\x80\xD0\xBA\xD0\xB0' in Russian!",
            true,
        ),
        ("an empty line", b"!", true),
        ("u-0 is a control character", b"\x00!", false),
        ("u-80 is a control character", b"\xC2\x80!", false),
        ("u-800 is valid", b"\xE0\xA0\x80!", true),
        ("u-10000 is valid", b"\xF0\x90\x80\x80!", true),
        ("5 bytes sequences are not allowed", b"\xF8\x88\x80\x80\x80!", false),
        (
            "6 bytes sequences are not allowed",
            b"\xFC\x84\x80\x80\x80\x80!",
            false,
        ),
        ("u-7f is a control character", b"\x7F!", false),
        ("u-7FF is valid", b"\xDF\xBF!", true),
        ("u-FFFF is a control character", b"\xEF\xBF\xBF!", false),
        ("u-1FFFFF is too large", b"\xF7\xBF\xBF\xBF!", false),
        ("u-3FFFFFF is 5 bytes", b"\xFB\xBF\xBF\xBF\xBF!", false),
        (
            "u-7FFFFFFF is 6 bytes",
            b"\xFD\xBF\xBF\xBF\xBF\xBF!",
            false,
        ),
        ("u-D7FF", b"\xED\x9F\xBF!", true),
        ("u-E000", b"\xEE\x80\x80!", true),
        ("u-FFFD", b"\xEF\xBF\xBD!", true),
        ("u-10FFFF", b"\xF4\x8F\xBF\xBF!", true),
        ("u-110000", b"\xF4\x90\x80\x80!", false),
        ("first continuation byte", b"\x80!", false),
        ("last continuation byte", b"\xBF!", false),
        ("2 continuation bytes", b"\x80\xBF!", false),
        ("3 continuation bytes", b"\x80\xBF\x80!", false),
        ("4 continuation bytes", b"\x80\xBF\x80\xBF!", false),
        ("5 continuation bytes", b"\x80\xBF\x80\xBF\x80!", false),
        ("6 continuation bytes", b"\x80\xBF\x80\xBF\x80\xBF!", false),
        ("7 continuation bytes", b"\x80\xBF\x80\xBF\x80\xBF\x80!", false),
        (
            "sequence of all 64 possible continuation bytes",
            b"\x80|\x81|\x82|\x83|\x84|\x85|\x86|\x87|\x88|\x89|\x8A|\x8B|\x8C|\x8D|\x8E|\x8F|\
\x90|\x91|\x92|\x93|\x94|\x95|\x96|\x97|\x98|\x99|\x9A|\x9B|\x9C|\x9D|\x9E|\x9F|\
\xA0|\xA1|\xA2|\xA3|\xA4|\xA5|\xA6|\xA7|\xA8|\xA9|\xAA|\xAB|\xAC|\xAD|\xAE|\xAF|\
\xB0|\xB1|\xB2|\xB3|\xB4|\xB5|\xB6|\xB7|\xB8|\xB9|\xBA|\xBB|\xBC|\xBD|\xBE|\xBF!",
            false,
        ),
        (
            "32 first bytes of 2-byte sequences",
            b"\xC0 |\xC1 |\xC2 |\xC3 |\xC4 |\xC5 |\xC6 |\xC7 |\xC8 |\xC9 |\xCA |\xCB |\xCC |\xCD |\xCE |\xCF |\
\xD0 |\xD1 |\xD2 |\xD3 |\xD4 |\xD5 |\xD6 |\xD7 |\xD8 |\xD9 |\xDA |\xDB |\xDC |\xDD |\xDE |\xDF !",
            false,
        ),
        (
            "16 first bytes of 3-byte sequences",
            b"\xE0 |\xE1 |\xE2 |\xE3 |\xE4 |\xE5 |\xE6 |\xE7 |\xE8 |\xE9 |\xEA |\xEB |\xEC |\xED |\xEE |\xEF !",
            false,
        ),
        (
            "8 first bytes of 4-byte sequences",
            b"\xF0 |\xF1 |\xF2 |\xF3 |\xF4 |\xF5 |\xF6 |\xF7 !",
            false,
        ),
        ("4 first bytes of 5-byte sequences", b"\xF8 |\xF9 |\xFA |\xFB !", false),
        ("2 first bytes of 6-byte sequences", b"\xFC |\xFD !", false),
        (
            "sequences with last byte missing {u-0}",
            b"\xC0|\xE0\x80|\xF0\x80\x80|\xF8\x80\x80\x80|\xFC\x80\x80\x80\x80!",
            false,
        ),
        (
            "sequences with last byte missing {u-...FF}",
            b"\xDF|\xEF\xBF|\xF7\xBF\xBF|\xFB\xBF\xBF\xBF|\xFD\xBF\xBF\xBF\xBF!",
            false,
        ),
        ("impossible bytes", b"\xFE|\xFF|\xFE\xFE\xFF\xFF!", false),
        (
            "overlong sequences {u-2f}",
            b"\xC0\xAF|\xE0\x80\xAF|\xF0\x80\x80\xAF|\xF8\x80\x80\x80\xAF|\xFC\x80\x80\x80\x80\xAF!",
            false,
        ),
        (
            "maximum overlong sequences",
            b"\xC1\xBF|\xE0\x9F\xBF|\xF0\x8F\xBF\xBF|\xF8\x87\xBF\xBF\xBF|\xFC\x83\xBF\xBF\xBF\xBF!",
            false,
        ),
        (
            "overlong representation of the NUL character",
            b"\xC0\x80|\xE0\x80\x80|\xF0\x80\x80\x80|\xF8\x80\x80\x80\x80|\xFC\x80\x80\x80\x80\x80!",
            false,
        ),
        (
            "single UTF-16 surrogates",
            b"\xED\xA0\x80|\xED\xAD\xBF|\xED\xAE\x80|\xED\xAF\xBF|\xED\xB0\x80|\xED\xBE\x80|\xED\xBF\xBF!",
            false,
        ),
        (
            "paired UTF-16 surrogates",
            b"\xED\xA0\x80\xED\xB0\x80|\xED\xA0\x80\xED\xBF\xBF|\xED\xAD\xBF\xED\xB0\x80|\
\xED\xAD\xBF\xED\xBF\xBF|\xED\xAE\x80\xED\xB0\x80|\xED\xAE\x80\xED\xBF\xBF|\
\xED\xAF\xBF\xED\xB0\x80|\xED\xAF\xBF\xED\xBF\xBF!",
            false,
        ),
        ("other illegal code positions", b"\xEF\xBF\xBE|\xEF\xBF\xBF!", false),
    ];

    for (title, input, expect_success) in cases {
        for segment in split_segments(input) {
            check_utf8_segment(title, segment, *expect_success);
        }
    }
}

#[test]
fn bom_detection_matches_vendored_reader_suite() {
    let cases: &[(&str, &[u8])] = &[
        (
            "no bom (utf-8)",
            b"Hi is \xD0\x9F\xD1\x80\xD0\xB8\xD0\xB2\xD0\xB5\xD1\x82!",
        ),
        (
            "bom (utf-8)",
            b"\xEF\xBB\xBFHi is \xD0\x9F\xD1\x80\xD0\xB8\xD0\xB2\xD0\xB5\xD1\x82!",
        ),
        (
            "bom (utf-16-le)",
            b"\xFF\xFEH\x00i\x00 \x00i\x00s\x00 \x00\x1F\x04@\x048\x042\x045\x04B\x04!",
        ),
        (
            "bom (utf-16-be)",
            b"\xFE\xFF\x00H\x00i\x00 \x00i\x00s\x00 \x04\x1F\x04@\x048\x042\x045\x04B!",
        ),
    ];

    for (title, input) in cases {
        let segment = split_segments(input)
            .next()
            .expect("bom case should contain data");
        let decoded = load_scalar_value(segment, None).expect(title);
        assert_eq!(decoded, BOM_ORIGINAL, "{title}");
    }
}

#[test]
fn chunked_generic_reader_still_detects_boms() {
    let cases: &[(&str, &[u8])] = &[
        (
            "bom (utf-8) through chunked reader",
            b"\xEF\xBB\xBFHi is \xD0\x9F\xD1\x80\xD0\xB8\xD0\xB2\xD0\xB5\xD1\x82",
        ),
        (
            "bom (utf-16-le) through chunked reader",
            b"\xFF\xFEH\x00i\x00 \x00i\x00s\x00 \x00\x1F\x04@\x048\x042\x045\x04B\x04",
        ),
        (
            "bom (utf-16-be) through chunked reader",
            b"\xFE\xFF\x00H\x00i\x00 \x00i\x00s\x00 \x04\x1F\x04@\x048\x042\x045\x04B",
        ),
    ];

    for (title, input) in cases {
        let decoded = load_scalar_value_from_reader(input, 1, None).expect(title);
        assert_eq!(decoded, BOM_ORIGINAL, "{title}");
    }
}

#[test]
fn long_utf8_sequence_decodes_as_single_scalar() {
    let mut input = Vec::with_capacity(3 + LONG * 2);
    let mut expected = Vec::with_capacity(LONG * 2);
    input.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    for index in 0..LONG {
        let low = if index % 2 == 0 { 0xAF } else { 0x90 };
        input.extend_from_slice(&[0xD0, low]);
        expected.extend_from_slice(&[0xD0, low]);
    }

    let decoded = load_scalar_value(&input, None).expect("long utf8 should decode");
    assert_eq!(decoded, expected);
}

#[test]
fn long_utf16_sequence_decodes_as_single_scalar() {
    let mut input = Vec::with_capacity(2 + LONG * 2);
    let mut expected = Vec::with_capacity(LONG * 2);
    input.extend_from_slice(&[0xFF, 0xFE]);
    for index in 0..LONG {
        input.push(if index % 2 == 0 { b'/' } else { 0x10 });
        input.push(0x04);
        expected.extend_from_slice(&[0xD0, if index % 2 == 0 { 0xAF } else { 0x90 }]);
    }

    let decoded = load_scalar_value(&input, None).expect("long utf16 should decode");
    assert_eq!(decoded, expected);
}

#[test]
fn generic_input_handler_with_explicit_encoding_decodes_utf16le_without_bom() {
    let mut parser = unsafe { mem::zeroed::<yaml_parser_t>() };
    let mut document = unsafe { mem::zeroed::<yaml_document_t>() };
    let mut reader = MemoryReader {
        input: UTF16LE_GREETING_NO_BOM.as_ptr(),
        size: UTF16LE_GREETING_NO_BOM.len(),
        offset: 0,
        chunk: 1,
    };

    unsafe {
        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input(
            &mut parser,
            Some(memory_read_handler),
            (&mut reader as *mut MemoryReader).cast(),
        );
        yaml_parser_set_encoding(&mut parser, yaml_encoding_t::YAML_UTF16LE_ENCODING);
        assert_eq!(yaml_parser_load(&mut parser, &mut document), 1);
        let root = yaml_document_get_root_node(&mut document);
        assert!(!root.is_null());
        assert_eq!((*root).r#type, yaml_node_type_t::YAML_SCALAR_NODE);
        assert_eq!(
            slice::from_raw_parts((*root).data.scalar.value, (*root).data.scalar.length),
            BOM_ORIGINAL
        );
        yaml_document_delete(&mut document);
        yaml_parser_delete(&mut parser);
    }
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
        assert_eq!(CStr::from_ptr(parser.problem), cstr!("input is too long"));
        assert_eq!(parser.problem_offset, MAX_FILE_SIZE);
        assert_eq!(parser.problem_value, -1);
        yaml_parser_delete(&mut parser);
    }
}

fn split_segments(input: &[u8]) -> impl Iterator<Item = &[u8]> {
    struct Segments<'a> {
        input: &'a [u8],
        offset: usize,
        done: bool,
    }

    impl<'a> Iterator for Segments<'a> {
        type Item = &'a [u8];

        fn next(&mut self) -> Option<Self::Item> {
            if self.done || self.offset >= self.input.len() {
                return None;
            }

            let start = self.offset;
            while self.offset < self.input.len()
                && self.input[self.offset] != b'|'
                && self.input[self.offset] != b'!'
            {
                self.offset += 1;
            }
            let end = self.offset;
            if self.offset < self.input.len() {
                self.done = self.input[self.offset] == b'!';
                self.offset += 1;
            } else {
                self.done = true;
            }
            Some(&self.input[start..end])
        }
    }

    Segments {
        input,
        offset: 0,
        done: false,
    }
}

fn check_utf8_segment(title: &str, input: &[u8], expect_success: bool) {
    unsafe {
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut document = mem::zeroed::<yaml_document_t>();
        let ok = load_document(input, None, &mut parser, &mut document);

        if expect_success {
            assert!(
                ok,
                "{title}: expected success, got error={:?} problem={:?}",
                parser.error,
                problem(&parser)
            );
            let root = yaml_document_get_root_node(&mut document);
            if input.is_empty() {
                assert!(root.is_null(), "{title}: expected an empty stream");
            } else {
                assert!(!root.is_null(), "{title}: expected a scalar document");
                assert_eq!(
                    (*root).r#type,
                    yaml_node_type_t::YAML_SCALAR_NODE,
                    "{title}"
                );
            }
        } else {
            assert!(!ok, "{title}: expected a reader error");
            assert_eq!(
                parser.error,
                yaml_error_type_t::YAML_READER_ERROR,
                "{title}"
            );
        }

        if ok {
            yaml_document_delete(&mut document);
        }
        yaml_parser_delete(&mut parser);
    }
}

fn load_scalar_value(input: &[u8], encoding: Option<yaml_encoding_t>) -> Result<Vec<u8>, String> {
    unsafe {
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut document = mem::zeroed::<yaml_document_t>();
        let ok = load_document(input, encoding, &mut parser, &mut document);
        if !ok {
            let message = problem(&parser).unwrap_or_else(|| String::from("unknown parser error"));
            yaml_parser_delete(&mut parser);
            return Err(message);
        }

        let root = yaml_document_get_root_node(&mut document);
        if root.is_null() || (*root).r#type != yaml_node_type_t::YAML_SCALAR_NODE {
            yaml_document_delete(&mut document);
            yaml_parser_delete(&mut parser);
            return Err(String::from("expected scalar document"));
        }

        let value =
            slice::from_raw_parts((*root).data.scalar.value, (*root).data.scalar.length).to_vec();
        yaml_document_delete(&mut document);
        yaml_parser_delete(&mut parser);
        Ok(value)
    }
}

fn load_scalar_value_from_reader(
    input: &[u8],
    chunk: usize,
    encoding: Option<yaml_encoding_t>,
) -> Result<Vec<u8>, String> {
    unsafe {
        let mut parser = mem::zeroed::<yaml_parser_t>();
        let mut document = mem::zeroed::<yaml_document_t>();
        let mut reader = MemoryReader {
            input: input.as_ptr(),
            size: input.len(),
            offset: 0,
            chunk,
        };

        assert_eq!(yaml_parser_initialize(&mut parser), 1);
        yaml_parser_set_input(
            &mut parser,
            Some(memory_read_handler),
            (&mut reader as *mut MemoryReader).cast(),
        );
        if let Some(encoding) = encoding {
            yaml_parser_set_encoding(&mut parser, encoding);
        }

        if yaml_parser_load(&mut parser, &mut document) != 1 {
            let message = problem(&parser).unwrap_or_else(|| String::from("unknown parser error"));
            yaml_parser_delete(&mut parser);
            return Err(message);
        }

        let root = yaml_document_get_root_node(&mut document);
        if root.is_null() || (*root).r#type != yaml_node_type_t::YAML_SCALAR_NODE {
            yaml_document_delete(&mut document);
            yaml_parser_delete(&mut parser);
            return Err(String::from("expected scalar document"));
        }

        let value =
            slice::from_raw_parts((*root).data.scalar.value, (*root).data.scalar.length).to_vec();
        yaml_document_delete(&mut document);
        yaml_parser_delete(&mut parser);
        Ok(value)
    }
}

unsafe fn load_document(
    input: &[u8],
    encoding: Option<yaml_encoding_t>,
    parser: *mut yaml_parser_t,
    document: *mut yaml_document_t,
) -> bool {
    assert_eq!(yaml_parser_initialize(parser), 1);
    yaml_parser_set_input_string(parser, input.as_ptr(), input.len());
    if let Some(encoding) = encoding {
        yaml_parser_set_encoding(parser, encoding);
    }
    yaml_parser_load(parser, document) == 1
}

fn problem(parser: &yaml_parser_t) -> Option<String> {
    if parser.problem.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(parser.problem) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}
