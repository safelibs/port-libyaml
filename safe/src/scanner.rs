use core::ffi::c_int;
use core::mem::size_of;
use core::ptr;

use crate::ffi;
use crate::types::{yaml_parser_t, yaml_token_t};

unsafe extern "C" {
    #[link_name = "phase2_hidden_yaml_parser_fetch_more_tokens"]
    fn hidden_yaml_parser_fetch_more_tokens(parser: *mut yaml_parser_t) -> c_int;
    #[link_name = "phase2_hidden_yaml_parser_scan"]
    fn hidden_yaml_parser_scan(parser: *mut yaml_parser_t, token: *mut yaml_token_t) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_fetch_more_tokens(parser: *mut yaml_parser_t) -> c_int {
    ffi::int_boundary(|| unsafe {
        if parser.is_null() {
            return 0;
        }
        hidden_yaml_parser_fetch_more_tokens(parser)
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_scan(
    parser: *mut yaml_parser_t,
    token: *mut yaml_token_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if !token.is_null() {
            ptr::write_bytes(token.cast::<u8>(), 0, size_of::<yaml_token_t>());
        }
        if parser.is_null() || token.is_null() {
            return 0;
        }
        hidden_yaml_parser_scan(parser, token)
    })
}
