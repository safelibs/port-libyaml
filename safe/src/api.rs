use core::ffi::{c_int, c_void};
use core::mem::size_of;
use core::ptr;

use crate::alloc;
use crate::ffi;
use crate::types::{
    yaml_char_t, yaml_encoding_t, yaml_error_type_t, yaml_file_t, yaml_mark_t, yaml_parser_state_t,
    yaml_parser_t, yaml_read_handler_t, yaml_simple_key_t, yaml_tag_directive_t, yaml_token_t,
    yaml_token_type_t,
};
pub(crate) use crate::{
    yaml_free, yaml_malloc, yaml_queue_extend, yaml_stack_extend, yaml_string_extend,
    yaml_string_join,
};
use crate::{FAIL, OK, PointerExt};

unsafe extern "C" {
    fn fread(ptr: *mut c_void, size: usize, nmemb: usize, stream: *mut c_void) -> usize;
    fn ferror(stream: *mut c_void) -> c_int;
}

#[inline]
unsafe fn zero_parser(parser: *mut yaml_parser_t) {
    alloc::zero_bytes(parser.cast(), size_of::<yaml_parser_t>());
}

#[inline]
unsafe fn zero_token(token: *mut yaml_token_t) {
    alloc::zero_bytes(token.cast(), size_of::<yaml_token_t>());
}

unsafe extern "C" fn yaml_string_read_handler(
    data: *mut c_void,
    buffer: *mut u8,
    mut size: usize,
    size_read: *mut usize,
) -> c_int {
    let parser = data.cast::<yaml_parser_t>();
    if parser.is_null() || size_read.is_null() {
        return FAIL;
    }

    if unsafe { (*parser).input.string.current == (*parser).input.string.end } {
        unsafe {
            *size_read = 0;
        }
        return OK;
    }

    let remaining = unsafe {
        (*parser)
            .input
            .string
            .end
            .c_offset_from((*parser).input.string.current) as usize
    };
    if size > remaining {
        size = remaining;
    }

    unsafe {
        alloc::copy_bytes(
            buffer.cast(),
            (*parser).input.string.current.cast(),
            size,
        );
        (*parser).input.string.current = (*parser).input.string.current.add(size);
        *size_read = size;
    }
    OK
}

unsafe extern "C" fn yaml_file_read_handler(
    data: *mut c_void,
    buffer: *mut u8,
    size: usize,
    size_read: *mut usize,
) -> c_int {
    let parser = data.cast::<yaml_parser_t>();
    if parser.is_null() || size_read.is_null() {
        return FAIL;
    }

    unsafe {
        *size_read = fread(buffer.cast(), 1, size, (*parser).input.file.cast());
        if ferror((*parser).input.file.cast()) != 0 {
            FAIL
        } else {
            OK
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_initialize(parser: *mut yaml_parser_t) -> c_int {
    ffi::int_boundary(|| unsafe {
        if parser.is_null() {
            return FAIL;
        }

        zero_parser(parser);

        if BUFFER_INIT!((*parser), (*parser).raw_buffer, crate::INPUT_RAW_BUFFER_SIZE) == FAIL {
            return FAIL;
        }
        if BUFFER_INIT!((*parser), (*parser).buffer, crate::INPUT_BUFFER_SIZE) == FAIL {
            BUFFER_DEL!((*parser).raw_buffer);
            return FAIL;
        }
        if QUEUE_INIT!((*parser).tokens, yaml_token_t) == FAIL {
            (*parser).error = yaml_error_type_t::YAML_MEMORY_ERROR;
            BUFFER_DEL!((*parser).raw_buffer);
            BUFFER_DEL!((*parser).buffer);
            return FAIL;
        }
        if STACK_INIT!((*parser).indents, c_int) == FAIL
            || STACK_INIT!((*parser).simple_keys, yaml_simple_key_t) == FAIL
            || STACK_INIT!((*parser).states, yaml_parser_state_t) == FAIL
            || STACK_INIT!((*parser).marks, yaml_mark_t) == FAIL
            || STACK_INIT!((*parser).tag_directives, yaml_tag_directive_t) == FAIL
        {
            (*parser).error = yaml_error_type_t::YAML_MEMORY_ERROR;
            BUFFER_DEL!((*parser).raw_buffer);
            BUFFER_DEL!((*parser).buffer);
            QUEUE_DEL!((*parser).tokens);
            STACK_DEL!((*parser).indents);
            STACK_DEL!((*parser).simple_keys);
            STACK_DEL!((*parser).states);
            STACK_DEL!((*parser).marks);
            STACK_DEL!((*parser).tag_directives);
            return FAIL;
        }

        OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_delete(parser: *mut yaml_parser_t) {
    ffi::void_boundary(|| unsafe {
        if parser.is_null() {
            return;
        }

        BUFFER_DEL!((*parser).raw_buffer);
        BUFFER_DEL!((*parser).buffer);
        while !QUEUE_EMPTY!((*parser).tokens) {
            let token = core::ptr::addr_of_mut!(DEQUEUE!((*parser).tokens));
            yaml_token_delete(token);
        }
        QUEUE_DEL!((*parser).tokens);
        STACK_DEL!((*parser).indents);
        STACK_DEL!((*parser).simple_keys);
        STACK_DEL!((*parser).states);
        STACK_DEL!((*parser).marks);
        while !STACK_EMPTY!((*parser).tag_directives) {
            let tag_directive = POP!((*parser).tag_directives);
            crate::yaml_free(tag_directive.handle.cast());
            crate::yaml_free(tag_directive.prefix.cast());
        }
        STACK_DEL!((*parser).tag_directives);

        zero_parser(parser);
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_set_input_string(
    parser: *mut yaml_parser_t,
    input: *const u8,
    size: usize,
) {
    ffi::void_boundary(|| unsafe {
        if parser.is_null() || input.is_null() || (*parser).read_handler.is_some() {
            return;
        }

        (*parser).read_handler = Some(yaml_string_read_handler);
        (*parser).read_handler_data = parser.cast();
        (*parser).input.string.start = input;
        (*parser).input.string.current = input;
        (*parser).input.string.end = input.add(size);
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_set_input_file(parser: *mut yaml_parser_t, file: *mut yaml_file_t) {
    ffi::void_boundary(|| unsafe {
        if parser.is_null() || file.is_null() || (*parser).read_handler.is_some() {
            return;
        }

        (*parser).read_handler = Some(yaml_file_read_handler);
        (*parser).read_handler_data = parser.cast();
        (*parser).input.file = file;
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_set_input(
    parser: *mut yaml_parser_t,
    handler: yaml_read_handler_t,
    data: *mut c_void,
) {
    ffi::void_boundary(|| unsafe {
        if parser.is_null() || handler.is_none() || (*parser).read_handler.is_some() {
            return;
        }

        (*parser).read_handler = handler;
        (*parser).read_handler_data = data;
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_set_encoding(
    parser: *mut yaml_parser_t,
    encoding: yaml_encoding_t,
) {
    ffi::void_boundary(|| unsafe {
        if parser.is_null() || (*parser).encoding != yaml_encoding_t::YAML_ANY_ENCODING {
            return;
        }
        (*parser).encoding = encoding;
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_token_delete(token: *mut yaml_token_t) {
    ffi::void_boundary(|| unsafe {
        if token.is_null() {
            return;
        }

        match (*token).r#type {
            yaml_token_type_t::YAML_TAG_DIRECTIVE_TOKEN => {
                crate::yaml_free((*token).data.tag_directive.handle.cast());
                crate::yaml_free((*token).data.tag_directive.prefix.cast());
            }
            yaml_token_type_t::YAML_ALIAS_TOKEN => {
                crate::yaml_free((*token).data.alias.value.cast());
            }
            yaml_token_type_t::YAML_ANCHOR_TOKEN => {
                crate::yaml_free((*token).data.anchor.value.cast());
            }
            yaml_token_type_t::YAML_TAG_TOKEN => {
                crate::yaml_free((*token).data.tag.handle.cast());
                crate::yaml_free((*token).data.tag.suffix.cast());
            }
            yaml_token_type_t::YAML_SCALAR_TOKEN => {
                crate::yaml_free((*token).data.scalar.value.cast());
            }
            _ => {}
        }

        zero_token(token);
    });
}
