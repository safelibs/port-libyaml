use crate::alloc;
use crate::types::yaml_char_t;

pub struct RawBufferTriplet<'a> {
    start: &'a mut *mut yaml_char_t,
    pointer: &'a mut *mut yaml_char_t,
    end: &'a mut *mut yaml_char_t,
}

impl<'a> RawBufferTriplet<'a> {
    pub unsafe fn from_raw(
        start: *mut *mut yaml_char_t,
        pointer: *mut *mut yaml_char_t,
        end: *mut *mut yaml_char_t,
    ) -> Option<Self> {
        if start.is_null() || pointer.is_null() || end.is_null() {
            return None;
        }

        Some(Self {
            start: &mut *start,
            pointer: &mut *pointer,
            end: &mut *end,
        })
    }

    pub fn start_value(&self) -> *mut yaml_char_t {
        *self.start
    }

    pub fn pointer_value(&self) -> *mut yaml_char_t {
        *self.pointer
    }

    pub fn end_value(&self) -> *mut yaml_char_t {
        *self.end
    }

    pub fn available_bytes(&self) -> Option<usize> {
        byte_span(self.start_value(), self.pointer_value(), self.end_value())
            .map(|(_, used, capacity)| capacity.saturating_sub(used))
    }

    pub unsafe fn extend(&mut self) -> bool {
        let (start_addr, pointer_offset, capacity) =
            match byte_span(self.start_value(), self.pointer_value(), self.end_value()) {
                Some((start_addr, used, capacity)) => (start_addr, used, capacity),
                _ => return false,
            };

        let new_size = capacity.saturating_mul(2);
        if new_size < capacity {
            return false;
        }

        let new_start =
            alloc::realloc_compat(self.start_value().cast(), new_size).cast::<yaml_char_t>();
        if new_start.is_null() {
            return false;
        }

        alloc::zero_bytes(new_start.add(capacity).cast(), capacity);

        *self.pointer = new_start.add(pointer_offset);
        *self.end = new_start.add(new_size);
        *self.start = new_start;

        let _ = start_addr;
        true
    }
}

pub fn used_bytes_from_pair(start: *mut yaml_char_t, pointer: *mut yaml_char_t) -> Option<usize> {
    byte_pair_span(start, pointer).map(|(_, used)| used)
}

fn byte_span(
    start: *mut yaml_char_t,
    pointer: *mut yaml_char_t,
    end: *mut yaml_char_t,
) -> Option<(usize, usize, usize)> {
    let start_addr = start as usize;
    let pointer_addr = pointer as usize;
    let end_addr = end as usize;

    if start_addr == 0 {
        return if pointer.is_null() && end.is_null() {
            Some((0, 0, 0))
        } else {
            None
        };
    }

    if pointer_addr < start_addr || pointer_addr > end_addr || end_addr < start_addr {
        return None;
    }

    Some((start_addr, pointer_addr - start_addr, end_addr - start_addr))
}

fn byte_pair_span(start: *mut yaml_char_t, pointer: *mut yaml_char_t) -> Option<(usize, usize)> {
    let start_addr = start as usize;
    let pointer_addr = pointer as usize;

    if start_addr == 0 {
        return if pointer.is_null() {
            Some((0, 0))
        } else {
            None
        };
    }

    if pointer_addr < start_addr {
        return None;
    }

    Some((start_addr, pointer_addr - start_addr))
}
