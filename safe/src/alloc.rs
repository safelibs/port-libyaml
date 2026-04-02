use core::ffi::{c_char, c_int, c_void};
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strdup(input: *const c_char) -> *mut c_char;
    fn memset(dest: *mut c_void, value: c_int, size: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, size: usize) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, size: usize) -> *mut c_void;
}

#[inline]
pub unsafe fn malloc_compat(size: usize) -> *mut c_void {
    malloc(if size == 0 { 1 } else { size })
}

#[inline]
pub unsafe fn realloc_compat(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() {
        malloc_compat(size)
    } else {
        realloc(ptr, if size == 0 { 1 } else { size })
    }
}

#[inline]
pub unsafe fn free_compat(ptr: *mut c_void) {
    if !ptr.is_null() {
        free(ptr);
    }
}

#[inline]
pub unsafe fn strdup_compat(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        ptr::null_mut()
    } else {
        strdup(input)
    }
}

#[inline]
pub unsafe fn zero_bytes(dest: *mut c_void, size: usize) {
    let _ = memset(dest, 0, size);
}

#[inline]
pub unsafe fn copy_bytes(dest: *mut c_void, src: *const c_void, size: usize) {
    let _ = memcpy(dest, src, size);
}

#[inline]
pub unsafe fn move_bytes(dest: *mut c_void, src: *const c_void, size: usize) {
    let _ = memmove(dest, src, size);
}
