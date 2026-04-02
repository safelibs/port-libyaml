use crate::alloc;
use core::ffi::c_void;

pub struct RawStackTriplet<'a> {
    start: &'a mut *mut c_void,
    top: &'a mut *mut c_void,
    end: &'a mut *mut c_void,
}

impl<'a> RawStackTriplet<'a> {
    pub unsafe fn from_raw(
        start: *mut *mut c_void,
        top: *mut *mut c_void,
        end: *mut *mut c_void,
    ) -> Option<Self> {
        if start.is_null() || top.is_null() || end.is_null() {
            return None;
        }

        Some(Self {
            start: &mut *start,
            top: &mut *top,
            end: &mut *end,
        })
    }

    pub unsafe fn extend(&mut self) -> bool {
        let (top_offset, span) = match span(self.start_value(), self.top_value(), self.end_value())
        {
            Some((top_offset, span)) => (top_offset, span),
            _ => return false,
        };

        if span >= (i32::MAX as usize) / 2 {
            return false;
        }

        let new_size = span.saturating_mul(2);
        if new_size < span {
            return false;
        }

        let new_start = alloc::realloc_compat(self.start_value(), new_size);
        if new_start.is_null() {
            return false;
        }

        *self.top = new_start.cast::<u8>().add(top_offset).cast();
        *self.end = new_start.cast::<u8>().add(new_size).cast();
        *self.start = new_start;
        true
    }

    fn start_value(&self) -> *mut c_void {
        *self.start
    }

    fn top_value(&self) -> *mut c_void {
        *self.top
    }

    fn end_value(&self) -> *mut c_void {
        *self.end
    }
}

fn span(start: *mut c_void, top: *mut c_void, end: *mut c_void) -> Option<(usize, usize)> {
    let start_addr = start as usize;
    let top_addr = top as usize;
    let end_addr = end as usize;

    if start_addr == 0 {
        return if top.is_null() && end.is_null() {
            Some((0, 0))
        } else {
            None
        };
    }

    if top_addr < start_addr || top_addr > end_addr || end_addr < start_addr {
        return None;
    }

    Some((top_addr - start_addr, end_addr - start_addr))
}
