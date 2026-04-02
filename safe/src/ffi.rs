use core::ffi::c_int;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

#[inline]
pub fn int_boundary<F>(body: F) -> c_int
where
    F: FnOnce() -> c_int,
{
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(_) => 0,
    }
}

#[inline]
pub fn ptr_boundary<T, F>(body: F) -> *mut T
where
    F: FnOnce() -> *mut T,
{
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(_) => ptr::null_mut(),
    }
}

#[inline]
pub fn const_ptr_boundary<T, F>(body: F) -> *const T
where
    F: FnOnce() -> *const T,
{
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(_) => ptr::null(),
    }
}

#[inline]
pub fn void_boundary<F>(body: F)
where
    F: FnOnce(),
{
    let _ = catch_unwind(AssertUnwindSafe(body));
}
