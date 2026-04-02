macro_rules! BUFFER_INIT {
    ($context:expr, $buffer:expr, $size:expr) => {{
        $buffer.start = crate::yaml_malloc($size) as _;
        if $buffer.start.is_null() {
            $context.error = crate::types::yaml_error_type_t::YAML_MEMORY_ERROR;
            crate::FAIL
        } else {
            $buffer.last = $buffer.start;
            $buffer.pointer = $buffer.start;
            $buffer.end = $buffer.start.wrapping_add($size);
            crate::OK
        }
    }};
}

macro_rules! BUFFER_DEL {
    ($buffer:expr) => {{
        crate::yaml_free($buffer.start.cast());
        $buffer.start = core::ptr::null_mut();
        $buffer.pointer = core::ptr::null_mut();
        $buffer.last = core::ptr::null_mut();
        $buffer.end = core::ptr::null_mut();
    }};
}

macro_rules! STRING_INIT {
    ($context:expr, $string:expr) => {{
        $string.start =
            crate::yaml_malloc(crate::INITIAL_STRING_SIZE) as *mut crate::types::yaml_char_t;
        $string.pointer = $string.start;
        $string.end = if $string.start.is_null() {
            core::ptr::null_mut()
        } else {
            $string.start.wrapping_add(crate::INITIAL_STRING_SIZE)
        };
        if $string.start.is_null() {
            $context.error = crate::types::yaml_error_type_t::YAML_MEMORY_ERROR;
            crate::FAIL
        } else {
            crate::alloc::zero_bytes($string.start.cast(), crate::INITIAL_STRING_SIZE);
            crate::OK
        }
    }};
}

macro_rules! STRING_ASSIGN {
    ($string:expr, $length:expr) => {
        crate::yaml::yaml_string_t {
            start: $string,
            end: $string.wrapping_add($length),
            pointer: $string,
        }
    };
}

macro_rules! STRING_DEL {
    ($string:expr) => {{
        crate::yaml_free($string.start.cast());
        $string.start = core::ptr::null_mut();
        $string.pointer = core::ptr::null_mut();
        $string.end = core::ptr::null_mut();
    }};
}

macro_rules! STRING_EXTEND {
    ($context:expr, $string:expr) => {{
        if !$string.end.is_null() && $string.pointer.wrapping_add(5) >= $string.end {
            if crate::yaml_string_extend(
                core::ptr::addr_of_mut!($string.start),
                core::ptr::addr_of_mut!($string.pointer),
                core::ptr::addr_of_mut!($string.end),
            ) == crate::FAIL
            {
                $context.error = crate::types::yaml_error_type_t::YAML_MEMORY_ERROR;
                crate::FAIL
            } else {
                crate::OK
            }
        } else {
            crate::OK
        }
    }};
}

macro_rules! CLEAR {
    ($string:expr) => {{
        $string.pointer = $string.start;
        if !$string.start.is_null() {
            crate::alloc::zero_bytes(
                $string.start.cast(),
                crate::PointerExt::c_offset_from($string.end, $string.start) as usize,
            );
        }
    }};
}

macro_rules! JOIN {
    ($context:expr, $string_a:expr, $string_b:expr) => {{
        if crate::yaml_string_join(
            core::ptr::addr_of_mut!($string_a.start),
            core::ptr::addr_of_mut!($string_a.pointer),
            core::ptr::addr_of_mut!($string_a.end),
            core::ptr::addr_of_mut!($string_b.start),
            core::ptr::addr_of_mut!($string_b.pointer),
            core::ptr::addr_of_mut!($string_b.end),
        ) == crate::FAIL
        {
            $context.error = crate::types::yaml_error_type_t::YAML_MEMORY_ERROR;
            crate::FAIL
        } else {
            $string_b.pointer = $string_b.start;
            crate::OK
        }
    }};
}

macro_rules! CHECK_AT {
    ($string:expr, $octet:expr, $offset:expr) => {
        *$string.pointer.wrapping_offset($offset as isize) == $octet
    };
}

macro_rules! CHECK {
    ($string:expr, $octet:expr) => {
        *$string.pointer == $octet
    };
}

macro_rules! IS_ALPHA {
    ($string:expr) => {
        (*$string.pointer >= b'0' && *$string.pointer <= b'9')
            || (*$string.pointer >= b'A' && *$string.pointer <= b'Z')
            || (*$string.pointer >= b'a' && *$string.pointer <= b'z')
            || *$string.pointer == b'_'
            || *$string.pointer == b'-'
    };
}

macro_rules! IS_DIGIT {
    ($string:expr) => {
        *$string.pointer >= b'0' && *$string.pointer <= b'9'
    };
}

macro_rules! AS_DIGIT {
    ($string:expr) => {
        (*$string.pointer - b'0') as core::ffi::c_int
    };
}

macro_rules! IS_HEX_AT {
    ($string:expr, $offset:expr) => {
        (*$string.pointer.wrapping_offset($offset) >= b'0'
            && *$string.pointer.wrapping_offset($offset) <= b'9')
            || (*$string.pointer.wrapping_offset($offset) >= b'A'
                && *$string.pointer.wrapping_offset($offset) <= b'F')
            || (*$string.pointer.wrapping_offset($offset) >= b'a'
                && *$string.pointer.wrapping_offset($offset) <= b'f')
    };
}

macro_rules! AS_HEX_AT {
    ($string:expr, $offset:expr) => {
        if *$string.pointer.wrapping_offset($offset) >= b'A'
            && *$string.pointer.wrapping_offset($offset) <= b'F'
        {
            *$string.pointer.wrapping_offset($offset) - b'A' + 10
        } else if *$string.pointer.wrapping_offset($offset) >= b'a'
            && *$string.pointer.wrapping_offset($offset) <= b'f'
        {
            *$string.pointer.wrapping_offset($offset) - b'a' + 10
        } else {
            *$string.pointer.wrapping_offset($offset) - b'0'
        } as core::ffi::c_int
    };
}

macro_rules! IS_ASCII {
    ($string:expr) => {
        *$string.pointer <= b'\x7F'
    };
}

macro_rules! IS_PRINTABLE {
    ($string:expr) => {
        (*$string.pointer == 0x0A
            || (*$string.pointer >= 0x20 && *$string.pointer <= 0x7E)
            || (*$string.pointer == 0xC2 && *$string.pointer.wrapping_offset(1) >= 0xA0)
            || (*$string.pointer > 0xC2 && *$string.pointer < 0xED)
            || (*$string.pointer == 0xED && *$string.pointer.wrapping_offset(1) < 0xA0)
            || *$string.pointer == 0xEE
            || (*$string.pointer == 0xEF
                && !(*$string.pointer.wrapping_offset(1) == 0xBB
                    && *$string.pointer.wrapping_offset(2) == 0xBF)
                && !(*$string.pointer.wrapping_offset(1) == 0xBF
                    && (*$string.pointer.wrapping_offset(2) == 0xBE
                        || *$string.pointer.wrapping_offset(2) == 0xBF))))
    };
}

macro_rules! IS_Z_AT {
    ($string:expr, $offset:expr) => {
        CHECK_AT!($string, b'\0', $offset)
    };
}

macro_rules! IS_Z {
    ($string:expr) => {
        IS_Z_AT!($string, 0)
    };
}

macro_rules! IS_BOM {
    ($string:expr) => {
        CHECK_AT!($string, b'\xEF', 0)
            && CHECK_AT!($string, b'\xBB', 1)
            && CHECK_AT!($string, b'\xBF', 2)
    };
}

macro_rules! IS_SPACE_AT {
    ($string:expr, $offset:expr) => {
        CHECK_AT!($string, b' ', $offset)
    };
}

macro_rules! IS_SPACE {
    ($string:expr) => {
        IS_SPACE_AT!($string, 0)
    };
}

macro_rules! IS_TAB_AT {
    ($string:expr, $offset:expr) => {
        CHECK_AT!($string, b'\t', $offset)
    };
}

macro_rules! IS_TAB {
    ($string:expr) => {
        IS_TAB_AT!($string, 0)
    };
}

macro_rules! IS_BLANK_AT {
    ($string:expr, $offset:expr) => {
        IS_SPACE_AT!($string, $offset) || IS_TAB_AT!($string, $offset)
    };
}

macro_rules! IS_BLANK {
    ($string:expr) => {
        IS_BLANK_AT!($string, 0)
    };
}

macro_rules! IS_BREAK_AT {
    ($string:expr, $offset:expr) => {
        CHECK_AT!($string, b'\r', $offset)
            || CHECK_AT!($string, b'\n', $offset)
            || (CHECK_AT!($string, b'\xC2', $offset) && CHECK_AT!($string, b'\x85', $offset + 1))
            || (CHECK_AT!($string, b'\xE2', $offset)
                && CHECK_AT!($string, b'\x80', $offset + 1)
                && CHECK_AT!($string, b'\xA8', $offset + 2))
            || (CHECK_AT!($string, b'\xE2', $offset)
                && CHECK_AT!($string, b'\x80', $offset + 1)
                && CHECK_AT!($string, b'\xA9', $offset + 2))
    };
}

macro_rules! IS_BREAK {
    ($string:expr) => {
        IS_BREAK_AT!($string, 0)
    };
}

macro_rules! IS_CRLF {
    ($string:expr) => {
        CHECK_AT!($string, b'\r', 0) && CHECK_AT!($string, b'\n', 1)
    };
}

macro_rules! IS_BREAKZ_AT {
    ($string:expr, $offset:expr) => {
        IS_BREAK_AT!($string, $offset) || IS_Z_AT!($string, $offset)
    };
}

macro_rules! IS_BREAKZ {
    ($string:expr) => {
        IS_BREAKZ_AT!($string, 0)
    };
}

macro_rules! IS_BLANKZ_AT {
    ($string:expr, $offset:expr) => {
        IS_BLANK_AT!($string, $offset) || IS_BREAKZ_AT!($string, $offset)
    };
}

macro_rules! IS_BLANKZ {
    ($string:expr) => {
        IS_BLANKZ_AT!($string, 0)
    };
}

macro_rules! WIDTH_AT {
    ($string:expr, $offset:expr) => {
        if *$string.pointer.wrapping_offset($offset as isize) & 0x80 == 0x00 {
            1
        } else if *$string.pointer.wrapping_offset($offset as isize) & 0xE0 == 0xC0 {
            2
        } else if *$string.pointer.wrapping_offset($offset as isize) & 0xF0 == 0xE0 {
            3
        } else if *$string.pointer.wrapping_offset($offset as isize) & 0xF8 == 0xF0 {
            4
        } else {
            0
        }
    };
}

macro_rules! WIDTH {
    ($string:expr) => {
        WIDTH_AT!($string, 0)
    };
}

macro_rules! MOVE {
    ($string:expr) => {
        $string.pointer = $string.pointer.wrapping_offset(WIDTH!($string) as isize)
    };
}

macro_rules! COPY {
    ($string_a:expr, $string_b:expr) => {
        if *$string_b.pointer & 0x80 == 0x00 {
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.wrapping_offset(1);
            $string_b.pointer = $string_b.pointer.wrapping_offset(1);
        } else if *$string_b.pointer & 0xE0 == 0xC0 {
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.wrapping_offset(1);
            $string_b.pointer = $string_b.pointer.wrapping_offset(1);
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.wrapping_offset(1);
            $string_b.pointer = $string_b.pointer.wrapping_offset(1);
        } else if *$string_b.pointer & 0xF0 == 0xE0 {
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.wrapping_offset(1);
            $string_b.pointer = $string_b.pointer.wrapping_offset(1);
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.wrapping_offset(1);
            $string_b.pointer = $string_b.pointer.wrapping_offset(1);
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.wrapping_offset(1);
            $string_b.pointer = $string_b.pointer.wrapping_offset(1);
        } else if *$string_b.pointer & 0xF8 == 0xF0 {
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.wrapping_offset(1);
            $string_b.pointer = $string_b.pointer.wrapping_offset(1);
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.wrapping_offset(1);
            $string_b.pointer = $string_b.pointer.wrapping_offset(1);
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.wrapping_offset(1);
            $string_b.pointer = $string_b.pointer.wrapping_offset(1);
            *$string_a.pointer = *$string_b.pointer;
            $string_a.pointer = $string_a.pointer.wrapping_offset(1);
            $string_b.pointer = $string_b.pointer.wrapping_offset(1);
        }
    };
}

macro_rules! STACK_INIT {
    ($stack:expr, $type:ty) => {{
        $stack.start = crate::yaml_malloc(crate::INITIAL_STACK_SIZE * core::mem::size_of::<$type>())
            as *mut $type;
        if $stack.start.is_null() {
            crate::FAIL
        } else {
            $stack.top = $stack.start;
            $stack.end = $stack.start.wrapping_add(crate::INITIAL_STACK_SIZE);
            crate::OK
        }
    }};
}

macro_rules! STACK_DEL {
    ($stack:expr) => {{
        crate::yaml_free($stack.start.cast());
        $stack.start = core::ptr::null_mut();
        $stack.top = core::ptr::null_mut();
        $stack.end = core::ptr::null_mut();
    }};
}

macro_rules! STACK_EMPTY {
    ($stack:expr) => {
        $stack.start == $stack.top
    };
}

macro_rules! PUSH {
    (do $context:expr, $stack:expr, $push:expr) => {{
        if $stack.top == $stack.end {
            if crate::yaml_stack_extend(
                core::ptr::addr_of_mut!($stack.start).cast(),
                core::ptr::addr_of_mut!($stack.top).cast(),
                core::ptr::addr_of_mut!($stack.end).cast(),
            ) == crate::FAIL
            {
                $context.error = crate::types::yaml_error_type_t::YAML_MEMORY_ERROR;
                crate::FAIL
            } else {
                $push;
                $stack.top = $stack.top.wrapping_add(1);
                crate::OK
            }
        } else {
            $push;
            $stack.top = $stack.top.wrapping_add(1);
            crate::OK
        }
    }};
    ($context:expr, $stack:expr, *$value:expr) => {
        PUSH!(do $context, $stack, core::ptr::copy_nonoverlapping($value, $stack.top, 1))
    };
    ($context:expr, $stack:expr, $value:expr) => {
        PUSH!(do $context, $stack, core::ptr::write($stack.top, $value))
    };
}

macro_rules! POP {
    ($stack:expr) => {
        *{
            $stack.top = $stack.top.wrapping_sub(1);
            $stack.top
        }
    };
}

macro_rules! QUEUE_INIT {
    ($queue:expr, $type:ty) => {{
        $queue.start = crate::yaml_malloc(crate::INITIAL_QUEUE_SIZE * core::mem::size_of::<$type>())
            as *mut $type;
        if $queue.start.is_null() {
            crate::FAIL
        } else {
            $queue.head = $queue.start;
            $queue.tail = $queue.start;
            $queue.end = $queue.start.wrapping_add(crate::INITIAL_QUEUE_SIZE);
            crate::OK
        }
    }};
}

macro_rules! QUEUE_DEL {
    ($queue:expr) => {{
        crate::yaml_free($queue.start.cast());
        $queue.start = core::ptr::null_mut();
        $queue.head = core::ptr::null_mut();
        $queue.tail = core::ptr::null_mut();
        $queue.end = core::ptr::null_mut();
    }};
}

macro_rules! QUEUE_EMPTY {
    ($queue:expr) => {
        $queue.head == $queue.tail
    };
}

macro_rules! ENQUEUE {
    (do $context:expr, $queue:expr, $enqueue:expr) => {{
        if $queue.tail == $queue.end {
            if crate::yaml_queue_extend(
                core::ptr::addr_of_mut!($queue.start).cast(),
                core::ptr::addr_of_mut!($queue.head).cast(),
                core::ptr::addr_of_mut!($queue.tail).cast(),
                core::ptr::addr_of_mut!($queue.end).cast(),
            ) == crate::FAIL
            {
                $context.error = crate::types::yaml_error_type_t::YAML_MEMORY_ERROR;
                crate::FAIL
            } else {
                $enqueue;
                $queue.tail = $queue.tail.wrapping_add(1);
                crate::OK
            }
        } else {
            $enqueue;
            $queue.tail = $queue.tail.wrapping_add(1);
            crate::OK
        }
    }};
    ($context:expr, $queue:expr, *$value:expr) => {
        ENQUEUE!(do $context, $queue, core::ptr::copy_nonoverlapping($value, $queue.tail, 1))
    };
    ($context:expr, $queue:expr, $value:expr) => {
        ENQUEUE!(do $context, $queue, core::ptr::write($queue.tail, $value))
    };
}

macro_rules! DEQUEUE {
    ($queue:expr) => {
        *{
            let head = $queue.head;
            $queue.head = $queue.head.wrapping_add(1);
            head
        }
    };
}

macro_rules! QUEUE_INSERT {
    ($context:expr, $queue:expr, $index:expr, $value:expr) => {{
        if $queue.tail == $queue.end {
            if crate::yaml_queue_extend(
                core::ptr::addr_of_mut!($queue.start).cast(),
                core::ptr::addr_of_mut!($queue.head).cast(),
                core::ptr::addr_of_mut!($queue.tail).cast(),
                core::ptr::addr_of_mut!($queue.end).cast(),
            ) == crate::FAIL
            {
                $context.error = crate::types::yaml_error_type_t::YAML_MEMORY_ERROR;
                crate::FAIL
            } else {
                crate::alloc::move_bytes(
                    ($queue.head)
                        .wrapping_add($index as usize)
                        .wrapping_add(1)
                        .cast(),
                    ($queue.head).wrapping_add($index as usize).cast(),
                    (crate::PointerExt::c_offset_from($queue.tail, $queue.head) as usize)
                        .saturating_sub($index as usize)
                        .saturating_mul(core::mem::size_of::<crate::types::yaml_token_t>()),
                );
                *($queue.head).wrapping_add($index as usize) = $value;
                $queue.tail = $queue.tail.wrapping_add(1);
                crate::OK
            }
        } else {
            crate::alloc::move_bytes(
                ($queue.head)
                    .wrapping_add($index as usize)
                    .wrapping_add(1)
                    .cast(),
                ($queue.head).wrapping_add($index as usize).cast(),
                (crate::PointerExt::c_offset_from($queue.tail, $queue.head) as usize)
                    .saturating_sub($index as usize)
                    .saturating_mul(core::mem::size_of::<crate::types::yaml_token_t>()),
            );
            *($queue.head).wrapping_add($index as usize) = $value;
            $queue.tail = $queue.tail.wrapping_add(1);
            crate::OK
        }
    }};
}
