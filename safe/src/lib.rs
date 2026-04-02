#![allow(non_camel_case_types, non_snake_case)]

#[macro_use]
mod macros;
pub mod alloc;
mod api;
mod document;
mod dumper;
mod emitter;
mod event;
pub mod ffi;
mod loader;
mod parser;
mod reader;
mod scanner;
pub mod types;
mod writer;

mod internal {
    pub mod buffer;
    pub mod queue;
    pub mod stack;
    pub mod utf;
}

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;

use crate::internal::buffer::{used_bytes_from_pair, RawBufferTriplet};
use crate::internal::queue::RawQueueQuad;
use crate::internal::stack::RawStackTriplet;
pub(crate) use crate::types::yaml_error_type_t::{
    YAML_MEMORY_ERROR, YAML_NO_ERROR, YAML_SCANNER_ERROR,
};
pub(crate) use crate::types::yaml_scalar_style_t::{
    YAML_DOUBLE_QUOTED_SCALAR_STYLE, YAML_FOLDED_SCALAR_STYLE, YAML_LITERAL_SCALAR_STYLE,
    YAML_PLAIN_SCALAR_STYLE, YAML_SINGLE_QUOTED_SCALAR_STYLE,
};
pub(crate) use crate::types::yaml_token_type_t::{
    YAML_ALIAS_TOKEN, YAML_ANCHOR_TOKEN, YAML_BLOCK_END_TOKEN, YAML_BLOCK_ENTRY_TOKEN,
    YAML_BLOCK_MAPPING_START_TOKEN, YAML_BLOCK_SEQUENCE_START_TOKEN, YAML_DOCUMENT_END_TOKEN,
    YAML_DOCUMENT_START_TOKEN, YAML_FLOW_ENTRY_TOKEN, YAML_FLOW_MAPPING_END_TOKEN,
    YAML_FLOW_MAPPING_START_TOKEN, YAML_FLOW_SEQUENCE_END_TOKEN, YAML_FLOW_SEQUENCE_START_TOKEN,
    YAML_KEY_TOKEN, YAML_SCALAR_TOKEN, YAML_STREAM_END_TOKEN, YAML_STREAM_START_TOKEN,
    YAML_TAG_DIRECTIVE_TOKEN, YAML_TAG_TOKEN, YAML_VALUE_TOKEN, YAML_VERSION_DIRECTIVE_TOKEN,
};

pub use api::{
    yaml_emitter_delete, yaml_emitter_initialize, yaml_emitter_set_break,
    yaml_emitter_set_canonical, yaml_emitter_set_encoding, yaml_emitter_set_indent,
    yaml_emitter_set_output, yaml_emitter_set_output_file, yaml_emitter_set_output_string,
    yaml_emitter_set_unicode, yaml_emitter_set_width, yaml_parser_delete, yaml_parser_initialize,
    yaml_parser_set_encoding, yaml_parser_set_input, yaml_parser_set_input_file,
    yaml_parser_set_input_string, yaml_token_delete,
};
pub use document::{
    yaml_document_add_mapping, yaml_document_add_scalar, yaml_document_add_sequence,
    yaml_document_append_mapping_pair, yaml_document_append_sequence_item, yaml_document_delete,
    yaml_document_get_node, yaml_document_get_root_node, yaml_document_initialize,
};
pub use dumper::{yaml_emitter_close, yaml_emitter_dump, yaml_emitter_open};
pub use emitter::yaml_emitter_emit;
pub use event::{
    yaml_alias_event_initialize, yaml_document_end_event_initialize,
    yaml_document_start_event_initialize, yaml_event_delete, yaml_mapping_end_event_initialize,
    yaml_mapping_start_event_initialize, yaml_scalar_event_initialize,
    yaml_sequence_end_event_initialize, yaml_sequence_start_event_initialize,
    yaml_stream_end_event_initialize, yaml_stream_start_event_initialize,
};
pub use internal::utf::{
    INITIAL_QUEUE_SIZE, INITIAL_STACK_SIZE, INITIAL_STRING_SIZE, INPUT_BUFFER_SIZE,
    INPUT_RAW_BUFFER_SIZE, MAX_FILE_SIZE, OUTPUT_BUFFER_SIZE, OUTPUT_RAW_BUFFER_SIZE,
};
pub use loader::yaml_parser_load;
pub use parser::yaml_parser_parse;
pub use reader::yaml_parser_update_buffer;
pub use scanner::{yaml_parser_fetch_more_tokens, yaml_parser_scan};
pub use types::*;
pub use writer::yaml_emitter_flush;

pub(crate) const OK: c_int = 1;
pub(crate) const FAIL: c_int = 0;

pub(crate) mod libc {
    pub type c_char = core::ffi::c_char;
    pub type c_int = core::ffi::c_int;
    pub type c_long = i64;
    pub type c_uchar = u8;
    pub type c_uint = u32;
    pub type c_ulong = usize;
    pub type c_void = core::ffi::c_void;
}

pub(crate) mod externs {
    use super::alloc;
    use core::ffi::{c_char, c_int, c_void};

    #[inline]
    pub unsafe fn memset(dest: *mut c_void, value: c_int, size: usize) -> *mut c_void {
        if value == 0 {
            alloc::zero_bytes(dest, size);
        } else {
            core::ptr::write_bytes(dest.cast::<u8>(), value as u8, size);
        }
        dest
    }

    #[inline]
    pub unsafe fn memcpy(dest: *mut c_void, src: *const c_void, size: usize) -> *mut c_void {
        alloc::copy_bytes(dest, src, size);
        dest
    }

    #[inline]
    pub unsafe fn memmove(dest: *mut c_void, src: *const c_void, size: usize) -> *mut c_void {
        alloc::move_bytes(dest, src, size);
        dest
    }

    #[inline]
    pub unsafe fn strcmp(lhs: *const c_char, rhs: *const c_char) -> c_int {
        alloc::compare_c_strings(lhs, rhs)
    }

    #[inline]
    pub unsafe fn strncmp(lhs: *const c_char, rhs: *const c_char, size: usize) -> c_int {
        alloc::compare_n_c_strings(lhs, rhs, size)
    }

    #[inline]
    pub unsafe fn strlen(input: *const c_char) -> usize {
        alloc::c_string_len(input)
    }
}

pub(crate) mod success {
    use core::ops::Deref;

    pub const OK: Success = Success { ok: true };
    pub const FAIL: Success = Success { ok: false };

    #[must_use]
    pub struct Success {
        pub ok: bool,
    }

    pub struct Failure {
        pub fail: bool,
    }

    impl Deref for Success {
        type Target = Failure;

        fn deref(&self) -> &Self::Target {
            if self.ok {
                &Failure { fail: false }
            } else {
                &Failure { fail: true }
            }
        }
    }
}

pub(crate) mod yaml {
    pub type ptrdiff_t = i64;
    pub type size_t = usize;
    pub type yaml_char_t = crate::yaml_char_t;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct yaml_string_t {
        pub start: *mut yaml_char_t,
        pub end: *mut yaml_char_t,
        pub pointer: *mut yaml_char_t,
    }

    pub const NULL_STRING: yaml_string_t = yaml_string_t {
        start: core::ptr::null_mut(),
        end: core::ptr::null_mut(),
        pointer: core::ptr::null_mut(),
    };
}

const YAML_VERSION_MAJOR: c_int = 0;
const YAML_VERSION_MINOR: c_int = 2;
const YAML_VERSION_PATCH: c_int = 5;
const YAML_VERSION_STRING: &[u8] = b"0.2.5\0";

#[no_mangle]
pub unsafe extern "C" fn yaml_get_version_string() -> *const c_char {
    ffi::const_ptr_boundary(|| YAML_VERSION_STRING.as_ptr().cast())
}

#[no_mangle]
pub unsafe extern "C" fn yaml_get_version(major: *mut c_int, minor: *mut c_int, patch: *mut c_int) {
    ffi::void_boundary(|| {
        if !major.is_null() {
            unsafe {
                *major = YAML_VERSION_MAJOR;
            }
        }
        if !minor.is_null() {
            unsafe {
                *minor = YAML_VERSION_MINOR;
            }
        }
        if !patch.is_null() {
            unsafe {
                *patch = YAML_VERSION_PATCH;
            }
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_malloc(size: usize) -> *mut c_void {
    ffi::ptr_boundary(|| unsafe { alloc::malloc_compat(size) })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    ffi::ptr_boundary(|| unsafe { alloc::realloc_compat(ptr, size) })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_free(ptr: *mut c_void) {
    ffi::void_boundary(|| unsafe { alloc::free_compat(ptr) });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_strdup(input: *const yaml_char_t) -> *mut yaml_char_t {
    ffi::ptr_boundary(|| unsafe { alloc::strdup_compat(input.cast()).cast() })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_string_extend(
    start: *mut *mut yaml_char_t,
    pointer: *mut *mut yaml_char_t,
    end: *mut *mut yaml_char_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        match RawBufferTriplet::from_raw(start, pointer, end) {
            Some(mut buffer) => {
                if buffer.extend() {
                    1
                } else {
                    0
                }
            }
            None => 0,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_string_join(
    a_start: *mut *mut yaml_char_t,
    a_pointer: *mut *mut yaml_char_t,
    a_end: *mut *mut yaml_char_t,
    b_start: *mut *mut yaml_char_t,
    b_pointer: *mut *mut yaml_char_t,
    _b_end: *mut *mut yaml_char_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        let mut a = match RawBufferTriplet::from_raw(a_start, a_pointer, a_end) {
            Some(view) => view,
            None => return 0,
        };
        if b_start.is_null() || b_pointer.is_null() {
            return 0;
        }
        let b_start_value = *b_start;
        let b_pointer_value = *b_pointer;
        if b_start_value == b_pointer_value {
            return 1;
        }
        let source_len = match used_bytes_from_pair(b_start_value, b_pointer_value) {
            Some(value) => value,
            None => return 0,
        };

        loop {
            match a.available_bytes() {
                Some(available) if available > source_len => break,
                Some(_) => {
                    let before = a.end_value() as usize - a.start_value() as usize;
                    if !a.extend() {
                        return 0;
                    }
                    let after = a.end_value() as usize - a.start_value() as usize;
                    if after <= before {
                        return 0;
                    }
                }
                None => return 0,
            }
        }

        alloc::copy_bytes(a.pointer_value().cast(), b_start_value.cast(), source_len);
        *a_pointer = a.pointer_value().add(source_len);

        1
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_stack_extend(
    start: *mut *mut c_void,
    top: *mut *mut c_void,
    end: *mut *mut c_void,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        match RawStackTriplet::from_raw(start, top, end) {
            Some(mut stack) => {
                if stack.extend() {
                    1
                } else {
                    0
                }
            }
            None => 0,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_queue_extend(
    start: *mut *mut c_void,
    head: *mut *mut c_void,
    tail: *mut *mut c_void,
    end: *mut *mut c_void,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        match RawQueueQuad::from_raw(start, head, tail, end) {
            Some(mut queue) => {
                if queue.extend_or_move() {
                    1
                } else {
                    0
                }
            }
            None => 0,
        }
    })
}

pub(crate) trait PointerExt: Sized {
    fn c_offset_from(self, origin: Self) -> isize;
}

impl<T> PointerExt for *const T {
    fn c_offset_from(self, origin: *const T) -> isize {
        (self as isize - origin as isize) / size_of::<T>() as isize
    }
}

impl<T> PointerExt for *mut T {
    fn c_offset_from(self, origin: *mut T) -> isize {
        (self as isize - origin as isize) / size_of::<T>() as isize
    }
}
