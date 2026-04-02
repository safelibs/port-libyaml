use core::ffi::{c_char, c_int};

use crate::ffi;
use crate::internal::utf::MAX_FILE_SIZE;
use crate::types::{yaml_error_type_t, yaml_parser_t};

const INPUT_TOO_LONG: &[u8] = b"input is too long\0";

unsafe extern "C" {
    #[link_name = "phase2_hidden_yaml_parser_update_buffer"]
    fn hidden_yaml_parser_update_buffer(parser: *mut yaml_parser_t, length: usize) -> c_int;
}

#[inline]
unsafe fn set_input_too_long(parser: *mut yaml_parser_t) -> c_int {
    (*parser).error = yaml_error_type_t::YAML_READER_ERROR;
    (*parser).problem = INPUT_TOO_LONG.as_ptr().cast::<c_char>();
    (*parser).problem_offset = (*parser).offset;
    (*parser).problem_value = -1;
    0
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_update_buffer(parser: *mut yaml_parser_t, length: usize) -> c_int {
    ffi::int_boundary(|| unsafe {
        if parser.is_null() {
            return 0;
        }
        if (*parser).read_handler.is_none() {
            return 0;
        }
        if (*parser).offset >= MAX_FILE_SIZE {
            return set_input_too_long(parser);
        }
        let result = hidden_yaml_parser_update_buffer(parser, length);
        if result != 0 && (*parser).offset >= MAX_FILE_SIZE {
            return set_input_too_long(parser);
        }
        result
    })
}
