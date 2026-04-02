use core::ffi::{c_int, c_void};

use crate::ffi;
use crate::types::{
    yaml_encoding_t, yaml_file_t, yaml_parser_t, yaml_read_handler_t, yaml_token_t,
};

unsafe extern "C" {
    #[link_name = "phase2_hidden_yaml_parser_initialize"]
    fn hidden_yaml_parser_initialize(parser: *mut yaml_parser_t) -> c_int;
    #[link_name = "phase2_hidden_yaml_parser_delete"]
    fn hidden_yaml_parser_delete(parser: *mut yaml_parser_t);
    #[link_name = "phase2_hidden_yaml_parser_set_input_string"]
    fn hidden_yaml_parser_set_input_string(
        parser: *mut yaml_parser_t,
        input: *const u8,
        size: usize,
    );
    #[link_name = "phase2_hidden_yaml_parser_set_input_file"]
    fn hidden_yaml_parser_set_input_file(parser: *mut yaml_parser_t, file: *mut yaml_file_t);
    #[link_name = "phase2_hidden_yaml_parser_set_input"]
    fn hidden_yaml_parser_set_input(
        parser: *mut yaml_parser_t,
        handler: yaml_read_handler_t,
        data: *mut c_void,
    );
    #[link_name = "phase2_hidden_yaml_parser_set_encoding"]
    fn hidden_yaml_parser_set_encoding(parser: *mut yaml_parser_t, encoding: yaml_encoding_t);
    #[link_name = "phase2_hidden_yaml_token_delete"]
    fn hidden_yaml_token_delete(token: *mut yaml_token_t);
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_initialize(parser: *mut yaml_parser_t) -> c_int {
    ffi::int_boundary(|| unsafe {
        if parser.is_null() {
            return 0;
        }
        hidden_yaml_parser_initialize(parser)
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_delete(parser: *mut yaml_parser_t) {
    ffi::void_boundary(|| unsafe {
        if parser.is_null() {
            return;
        }
        hidden_yaml_parser_delete(parser);
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_set_input_string(
    parser: *mut yaml_parser_t,
    input: *const u8,
    size: usize,
) {
    ffi::void_boundary(|| unsafe {
        if parser.is_null() || input.is_null() {
            return;
        }
        if (*parser).read_handler.is_some() {
            return;
        }
        hidden_yaml_parser_set_input_string(parser, input, size);
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_set_input_file(parser: *mut yaml_parser_t, file: *mut yaml_file_t) {
    ffi::void_boundary(|| unsafe {
        if parser.is_null() || file.is_null() {
            return;
        }
        if (*parser).read_handler.is_some() {
            return;
        }
        hidden_yaml_parser_set_input_file(parser, file);
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_set_input(
    parser: *mut yaml_parser_t,
    handler: yaml_read_handler_t,
    data: *mut c_void,
) {
    ffi::void_boundary(|| unsafe {
        if parser.is_null() || handler.is_none() {
            return;
        }
        if (*parser).read_handler.is_some() {
            return;
        }
        hidden_yaml_parser_set_input(parser, handler, data);
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_set_encoding(
    parser: *mut yaml_parser_t,
    encoding: yaml_encoding_t,
) {
    ffi::void_boundary(|| unsafe {
        if parser.is_null() {
            return;
        }
        if (*parser).encoding != yaml_encoding_t::YAML_ANY_ENCODING {
            return;
        }
        hidden_yaml_parser_set_encoding(parser, encoding);
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_token_delete(token: *mut yaml_token_t) {
    ffi::void_boundary(|| unsafe {
        if token.is_null() {
            return;
        }
        hidden_yaml_token_delete(token);
    });
}
