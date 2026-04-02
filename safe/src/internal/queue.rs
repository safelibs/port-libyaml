use crate::alloc;
use core::ffi::c_void;

pub struct RawQueueQuad<'a> {
    start: &'a mut *mut c_void,
    head: &'a mut *mut c_void,
    tail: &'a mut *mut c_void,
    end: &'a mut *mut c_void,
}

impl<'a> RawQueueQuad<'a> {
    pub unsafe fn from_raw(
        start: *mut *mut c_void,
        head: *mut *mut c_void,
        tail: *mut *mut c_void,
        end: *mut *mut c_void,
    ) -> Option<Self> {
        if start.is_null() || head.is_null() || tail.is_null() || end.is_null() {
            return None;
        }

        Some(Self {
            start: &mut *start,
            head: &mut *head,
            tail: &mut *tail,
            end: &mut *end,
        })
    }

    pub unsafe fn extend_or_move(&mut self) -> bool {
        let (head_offset, tail_offset, span) = match span(
            self.start_value(),
            self.head_value(),
            self.tail_value(),
            self.end_value(),
        ) {
            Some(values) => values,
            _ => return false,
        };

        if self.start_value() == self.head_value() && self.tail_value() == self.end_value() {
            let new_size = span.saturating_mul(2);
            if new_size < span {
                return false;
            }

            let new_start = alloc::realloc_compat(self.start_value(), new_size);
            if new_start.is_null() {
                return false;
            }

            *self.head = new_start.cast::<u8>().add(head_offset).cast();
            *self.tail = new_start.cast::<u8>().add(tail_offset).cast();
            *self.end = new_start.cast::<u8>().add(new_size).cast();
            *self.start = new_start;
        }

        if self.tail_value() == self.end_value() {
            let moved_size =
                (self.tail_value() as usize).saturating_sub(self.head_value() as usize);
            if self.head_value() != self.tail_value() {
                alloc::move_bytes(
                    self.start_value(),
                    self.head_value().cast_const(),
                    moved_size,
                );
            }

            let new_start = self.start_value() as usize;
            let new_head = new_start;
            let new_tail = new_start.saturating_add(moved_size);

            *self.head = new_head as *mut c_void;
            *self.tail = new_tail as *mut c_void;
        }

        true
    }

    fn start_value(&self) -> *mut c_void {
        *self.start
    }

    fn head_value(&self) -> *mut c_void {
        *self.head
    }

    fn tail_value(&self) -> *mut c_void {
        *self.tail
    }

    fn end_value(&self) -> *mut c_void {
        *self.end
    }
}

fn span(
    start: *mut c_void,
    head: *mut c_void,
    tail: *mut c_void,
    end: *mut c_void,
) -> Option<(usize, usize, usize)> {
    let start_addr = start as usize;
    let head_addr = head as usize;
    let tail_addr = tail as usize;
    let end_addr = end as usize;

    if start_addr == 0 {
        return if head.is_null() && tail.is_null() && end.is_null() {
            Some((0, 0, 0))
        } else {
            None
        };
    }

    if head_addr < start_addr
        || head_addr > tail_addr
        || tail_addr > end_addr
        || end_addr < start_addr
    {
        return None;
    }

    Some((
        head_addr - start_addr,
        tail_addr - start_addr,
        end_addr - start_addr,
    ))
}
