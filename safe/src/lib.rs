#![allow(non_camel_case_types, non_snake_case)]

pub mod alloc;
pub mod ffi;
pub mod types;

mod internal {
    pub mod buffer;
    pub mod queue;
    pub mod stack;
}

use core::ffi::{c_char, c_int, c_void};

use crate::internal::buffer::{used_bytes_from_pair, RawBufferTriplet};
use crate::internal::queue::RawQueueQuad;
use crate::internal::stack::RawStackTriplet;

pub use types::*;

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
