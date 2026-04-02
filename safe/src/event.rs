use core::ffi::c_int;
use core::mem::size_of;
use core::ptr;

use crate::alloc;
use crate::ffi;
use crate::types::{
    yaml_char_t, yaml_encoding_t, yaml_error_type_t, yaml_event_data_alias_t,
    yaml_event_data_document_end_t, yaml_event_data_document_start_t,
    yaml_event_data_document_start_tag_directives_t, yaml_event_data_mapping_start_t,
    yaml_event_data_scalar_t, yaml_event_data_sequence_start_t, yaml_event_data_stream_start_t,
    yaml_event_data_t, yaml_event_t, yaml_event_type_t, yaml_mapping_style_t, yaml_mark_t,
    yaml_parser_tag_directives_t, yaml_scalar_style_t, yaml_sequence_style_t, yaml_tag_directive_t,
    yaml_version_directive_t,
};
use crate::{yaml_free, yaml_malloc, yaml_strdup, FAIL, OK};

#[inline]
pub(crate) fn zero_mark() -> yaml_mark_t {
    yaml_mark_t {
        index: 0,
        line: 0,
        column: 0,
    }
}

#[inline]
fn empty_event_data() -> yaml_event_data_t {
    unsafe { core::mem::zeroed() }
}

#[inline]
pub(crate) unsafe fn zero_event(event: *mut yaml_event_t) {
    alloc::zero_bytes(event.cast(), size_of::<yaml_event_t>());
}

#[inline]
pub(crate) unsafe fn initialize_stream_start_event(
    event: *mut yaml_event_t,
    encoding: yaml_encoding_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *event = yaml_event_t {
        r#type: yaml_event_type_t::YAML_STREAM_START_EVENT,
        data: yaml_event_data_t {
            stream_start: yaml_event_data_stream_start_t { encoding },
        },
        start_mark,
        end_mark,
    };
}

#[inline]
pub(crate) unsafe fn initialize_stream_end_event(
    event: *mut yaml_event_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *event = yaml_event_t {
        r#type: yaml_event_type_t::YAML_STREAM_END_EVENT,
        data: empty_event_data(),
        start_mark,
        end_mark,
    };
}

#[inline]
pub(crate) unsafe fn initialize_document_start_event(
    event: *mut yaml_event_t,
    version_directive: *mut yaml_version_directive_t,
    tag_directives_start: *mut yaml_tag_directive_t,
    tag_directives_end: *mut yaml_tag_directive_t,
    implicit: c_int,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *event = yaml_event_t {
        r#type: yaml_event_type_t::YAML_DOCUMENT_START_EVENT,
        data: yaml_event_data_t {
            document_start: yaml_event_data_document_start_t {
                version_directive,
                tag_directives: yaml_event_data_document_start_tag_directives_t {
                    start: tag_directives_start,
                    end: tag_directives_end,
                },
                implicit,
            },
        },
        start_mark,
        end_mark,
    };
}

#[inline]
pub(crate) unsafe fn initialize_document_end_event(
    event: *mut yaml_event_t,
    implicit: c_int,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *event = yaml_event_t {
        r#type: yaml_event_type_t::YAML_DOCUMENT_END_EVENT,
        data: yaml_event_data_t {
            document_end: yaml_event_data_document_end_t { implicit },
        },
        start_mark,
        end_mark,
    };
}

#[inline]
pub(crate) unsafe fn initialize_alias_event(
    event: *mut yaml_event_t,
    anchor: *mut yaml_char_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *event = yaml_event_t {
        r#type: yaml_event_type_t::YAML_ALIAS_EVENT,
        data: yaml_event_data_t {
            alias: yaml_event_data_alias_t { anchor },
        },
        start_mark,
        end_mark,
    };
}

#[inline]
pub(crate) unsafe fn initialize_scalar_event(
    event: *mut yaml_event_t,
    anchor: *mut yaml_char_t,
    tag: *mut yaml_char_t,
    value: *mut yaml_char_t,
    length: usize,
    plain_implicit: c_int,
    quoted_implicit: c_int,
    style: yaml_scalar_style_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *event = yaml_event_t {
        r#type: yaml_event_type_t::YAML_SCALAR_EVENT,
        data: yaml_event_data_t {
            scalar: yaml_event_data_scalar_t {
                anchor,
                tag,
                value,
                length,
                plain_implicit,
                quoted_implicit,
                style,
            },
        },
        start_mark,
        end_mark,
    };
}

#[inline]
pub(crate) unsafe fn initialize_sequence_start_event(
    event: *mut yaml_event_t,
    anchor: *mut yaml_char_t,
    tag: *mut yaml_char_t,
    implicit: c_int,
    style: yaml_sequence_style_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *event = yaml_event_t {
        r#type: yaml_event_type_t::YAML_SEQUENCE_START_EVENT,
        data: yaml_event_data_t {
            sequence_start: yaml_event_data_sequence_start_t {
                anchor,
                tag,
                implicit,
                style,
            },
        },
        start_mark,
        end_mark,
    };
}

#[inline]
pub(crate) unsafe fn initialize_sequence_end_event(
    event: *mut yaml_event_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *event = yaml_event_t {
        r#type: yaml_event_type_t::YAML_SEQUENCE_END_EVENT,
        data: empty_event_data(),
        start_mark,
        end_mark,
    };
}

#[inline]
pub(crate) unsafe fn initialize_mapping_start_event(
    event: *mut yaml_event_t,
    anchor: *mut yaml_char_t,
    tag: *mut yaml_char_t,
    implicit: c_int,
    style: yaml_mapping_style_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *event = yaml_event_t {
        r#type: yaml_event_type_t::YAML_MAPPING_START_EVENT,
        data: yaml_event_data_t {
            mapping_start: yaml_event_data_mapping_start_t {
                anchor,
                tag,
                implicit,
                style,
            },
        },
        start_mark,
        end_mark,
    };
}

#[inline]
pub(crate) unsafe fn initialize_mapping_end_event(
    event: *mut yaml_event_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *event = yaml_event_t {
        r#type: yaml_event_type_t::YAML_MAPPING_END_EVENT,
        data: empty_event_data(),
        start_mark,
        end_mark,
    };
}

pub(crate) unsafe fn yaml_check_utf8(start: *const yaml_char_t, length: usize) -> bool {
    let end = start.add(length);
    let mut pointer = start;

    while pointer < end {
        let octet = *pointer;
        let width = if octet & 0x80 == 0x00 {
            1
        } else if octet & 0xE0 == 0xC0 {
            2
        } else if octet & 0xF0 == 0xE0 {
            3
        } else if octet & 0xF8 == 0xF0 {
            4
        } else {
            0
        };
        let mut value = if octet & 0x80 == 0x00 {
            (octet & 0x7F) as u32
        } else if octet & 0xE0 == 0xC0 {
            (octet & 0x1F) as u32
        } else if octet & 0xF0 == 0xE0 {
            (octet & 0x0F) as u32
        } else if octet & 0xF8 == 0xF0 {
            (octet & 0x07) as u32
        } else {
            0
        };
        if width == 0 {
            return false;
        }
        if pointer.add(width) > end {
            return false;
        }
        for k in 1..width {
            let trailing = *pointer.add(k);
            if trailing & 0xC0 != 0x80 {
                return false;
            }
            value = (value << 6) + (trailing & 0x3F) as u32;
        }
        if !((width == 1)
            || (width == 2 && value >= 0x80)
            || (width == 3 && value >= 0x800)
            || (width == 4 && value >= 0x10000))
        {
            return false;
        }
        pointer = pointer.add(width);
    }

    true
}

pub(crate) unsafe fn duplicate_utf8_c_string(input: *const yaml_char_t) -> *mut yaml_char_t {
    let length = alloc::c_string_len(input.cast());
    if !yaml_check_utf8(input, length) {
        return ptr::null_mut();
    }
    yaml_strdup(input)
}

pub(crate) unsafe fn duplicate_scalar_value(
    value: *const yaml_char_t,
    length: usize,
) -> *mut yaml_char_t {
    if !yaml_check_utf8(value, length) {
        return ptr::null_mut();
    }
    let size = match length.checked_add(1) {
        Some(size) => size,
        None => return ptr::null_mut(),
    };
    let copy = yaml_malloc(size).cast::<yaml_char_t>();
    if copy.is_null() {
        return ptr::null_mut();
    }
    alloc::copy_bytes(copy.cast(), value.cast(), length);
    *copy.add(length) = b'\0';
    copy
}

#[no_mangle]
pub unsafe extern "C" fn yaml_stream_start_event_initialize(
    event: *mut yaml_event_t,
    encoding: yaml_encoding_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if event.is_null() {
            return FAIL;
        }
        initialize_stream_start_event(event, encoding, zero_mark(), zero_mark());
        OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_stream_end_event_initialize(event: *mut yaml_event_t) -> c_int {
    ffi::int_boundary(|| unsafe {
        if event.is_null() {
            return FAIL;
        }
        initialize_stream_end_event(event, zero_mark(), zero_mark());
        OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_document_start_event_initialize(
    event: *mut yaml_event_t,
    version_directive: *mut yaml_version_directive_t,
    tag_directives_start: *mut yaml_tag_directive_t,
    tag_directives_end: *mut yaml_tag_directive_t,
    implicit: c_int,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if event.is_null() {
            return FAIL;
        }
        if (tag_directives_start.is_null() && !tag_directives_end.is_null())
            || (!tag_directives_start.is_null() && tag_directives_end.is_null())
        {
            return FAIL;
        }

        #[repr(C)]
        struct Context {
            error: yaml_error_type_t,
        }

        let mut context = Context {
            error: yaml_error_type_t::YAML_NO_ERROR,
        };
        let mut version_directive_copy = ptr::null_mut::<yaml_version_directive_t>();
        let mut tag_directives_copy = yaml_parser_tag_directives_t {
            start: ptr::null_mut(),
            end: ptr::null_mut(),
            top: ptr::null_mut(),
        };
        let mut value = yaml_tag_directive_t {
            handle: ptr::null_mut(),
            prefix: ptr::null_mut(),
        };

        if !version_directive.is_null() {
            version_directive_copy = yaml_malloc(size_of::<yaml_version_directive_t>())
                .cast::<yaml_version_directive_t>();
            if version_directive_copy.is_null() {
                goto_event_error(version_directive_copy, &mut tag_directives_copy, &mut value);
                return FAIL;
            }
            (*version_directive_copy).major = (*version_directive).major;
            (*version_directive_copy).minor = (*version_directive).minor;
        }

        if tag_directives_start != tag_directives_end {
            if STACK_INIT!(tag_directives_copy, yaml_tag_directive_t) == FAIL {
                yaml_free(version_directive_copy.cast());
                return FAIL;
            }

            let mut tag_directive = tag_directives_start;
            while tag_directive != tag_directives_end {
                if (*tag_directive).handle.is_null()
                    || (*tag_directive).prefix.is_null()
                    || !yaml_check_utf8(
                        (*tag_directive).handle,
                        alloc::c_string_len((*tag_directive).handle.cast()),
                    )
                    || !yaml_check_utf8(
                        (*tag_directive).prefix,
                        alloc::c_string_len((*tag_directive).prefix.cast()),
                    )
                {
                    goto_event_error(version_directive_copy, &mut tag_directives_copy, &mut value);
                    return FAIL;
                }

                value.handle = yaml_strdup((*tag_directive).handle);
                value.prefix = yaml_strdup((*tag_directive).prefix);
                if value.handle.is_null() || value.prefix.is_null() {
                    goto_event_error(version_directive_copy, &mut tag_directives_copy, &mut value);
                    return FAIL;
                }
                if PUSH!(context, tag_directives_copy, value) == FAIL {
                    goto_event_error(version_directive_copy, &mut tag_directives_copy, &mut value);
                    return FAIL;
                }
                value.handle = ptr::null_mut();
                value.prefix = ptr::null_mut();
                tag_directive = tag_directive.add(1);
            }
        }

        initialize_document_start_event(
            event,
            version_directive_copy,
            tag_directives_copy.start,
            tag_directives_copy.top,
            implicit,
            zero_mark(),
            zero_mark(),
        );
        OK
    })
}

#[inline]
unsafe fn goto_event_error(
    version_directive_copy: *mut yaml_version_directive_t,
    tag_directives_copy: &mut yaml_parser_tag_directives_t,
    value: &mut yaml_tag_directive_t,
) {
    yaml_free(version_directive_copy.cast());
    while !STACK_EMPTY!(*tag_directives_copy) {
        let entry = POP!(*tag_directives_copy);
        yaml_free(entry.handle.cast());
        yaml_free(entry.prefix.cast());
    }
    STACK_DEL!(*tag_directives_copy);
    yaml_free(value.handle.cast());
    yaml_free(value.prefix.cast());
    value.handle = ptr::null_mut();
    value.prefix = ptr::null_mut();
}

#[no_mangle]
pub unsafe extern "C" fn yaml_document_end_event_initialize(
    event: *mut yaml_event_t,
    implicit: c_int,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if event.is_null() {
            return FAIL;
        }
        initialize_document_end_event(event, implicit, zero_mark(), zero_mark());
        OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_alias_event_initialize(
    event: *mut yaml_event_t,
    anchor: *const yaml_char_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if event.is_null() || anchor.is_null() {
            return FAIL;
        }
        let anchor_copy = duplicate_utf8_c_string(anchor);
        if anchor_copy.is_null() {
            return FAIL;
        }
        initialize_alias_event(event, anchor_copy, zero_mark(), zero_mark());
        OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_scalar_event_initialize(
    event: *mut yaml_event_t,
    anchor: *const yaml_char_t,
    tag: *const yaml_char_t,
    value: *const yaml_char_t,
    mut length: c_int,
    plain_implicit: c_int,
    quoted_implicit: c_int,
    style: yaml_scalar_style_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if event.is_null() || value.is_null() {
            return FAIL;
        }

        let mut anchor_copy = ptr::null_mut::<yaml_char_t>();
        let mut tag_copy = ptr::null_mut::<yaml_char_t>();
        let mut value_copy = ptr::null_mut::<yaml_char_t>();

        if !anchor.is_null() {
            anchor_copy = duplicate_utf8_c_string(anchor);
            if anchor_copy.is_null() {
                return FAIL;
            }
        }
        if !tag.is_null() {
            tag_copy = duplicate_utf8_c_string(tag);
            if tag_copy.is_null() {
                yaml_free(anchor_copy.cast());
                return FAIL;
            }
        }
        if length < 0 {
            length = alloc::c_string_len(value.cast()) as c_int;
        }
        value_copy = duplicate_scalar_value(value, length as usize);
        if value_copy.is_null() {
            yaml_free(anchor_copy.cast());
            yaml_free(tag_copy.cast());
            return FAIL;
        }

        initialize_scalar_event(
            event,
            anchor_copy,
            tag_copy,
            value_copy,
            length as usize,
            plain_implicit,
            quoted_implicit,
            style,
            zero_mark(),
            zero_mark(),
        );
        OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_sequence_start_event_initialize(
    event: *mut yaml_event_t,
    anchor: *const yaml_char_t,
    tag: *const yaml_char_t,
    implicit: c_int,
    style: yaml_sequence_style_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if event.is_null() {
            return FAIL;
        }

        let mut anchor_copy = ptr::null_mut::<yaml_char_t>();
        let mut tag_copy = ptr::null_mut::<yaml_char_t>();

        if !anchor.is_null() {
            anchor_copy = duplicate_utf8_c_string(anchor);
            if anchor_copy.is_null() {
                return FAIL;
            }
        }
        if !tag.is_null() {
            tag_copy = duplicate_utf8_c_string(tag);
            if tag_copy.is_null() {
                yaml_free(anchor_copy.cast());
                return FAIL;
            }
        }

        initialize_sequence_start_event(
            event,
            anchor_copy,
            tag_copy,
            implicit,
            style,
            zero_mark(),
            zero_mark(),
        );
        OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_sequence_end_event_initialize(event: *mut yaml_event_t) -> c_int {
    ffi::int_boundary(|| unsafe {
        if event.is_null() {
            return FAIL;
        }
        initialize_sequence_end_event(event, zero_mark(), zero_mark());
        OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_mapping_start_event_initialize(
    event: *mut yaml_event_t,
    anchor: *const yaml_char_t,
    tag: *const yaml_char_t,
    implicit: c_int,
    style: yaml_mapping_style_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if event.is_null() {
            return FAIL;
        }

        let mut anchor_copy = ptr::null_mut::<yaml_char_t>();
        let mut tag_copy = ptr::null_mut::<yaml_char_t>();

        if !anchor.is_null() {
            anchor_copy = duplicate_utf8_c_string(anchor);
            if anchor_copy.is_null() {
                return FAIL;
            }
        }
        if !tag.is_null() {
            tag_copy = duplicate_utf8_c_string(tag);
            if tag_copy.is_null() {
                yaml_free(anchor_copy.cast());
                return FAIL;
            }
        }

        initialize_mapping_start_event(
            event,
            anchor_copy,
            tag_copy,
            implicit,
            style,
            zero_mark(),
            zero_mark(),
        );
        OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_mapping_end_event_initialize(event: *mut yaml_event_t) -> c_int {
    ffi::int_boundary(|| unsafe {
        if event.is_null() {
            return FAIL;
        }
        initialize_mapping_end_event(event, zero_mark(), zero_mark());
        OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_event_delete(event: *mut yaml_event_t) {
    ffi::void_boundary(|| unsafe {
        if event.is_null() {
            return;
        }

        match (*event).r#type {
            yaml_event_type_t::YAML_DOCUMENT_START_EVENT => {
                yaml_free((*event).data.document_start.version_directive.cast());
                let mut tag_directive = (*event).data.document_start.tag_directives.start;
                while tag_directive != (*event).data.document_start.tag_directives.end {
                    yaml_free((*tag_directive).handle.cast());
                    yaml_free((*tag_directive).prefix.cast());
                    tag_directive = tag_directive.add(1);
                }
                yaml_free((*event).data.document_start.tag_directives.start.cast());
            }
            yaml_event_type_t::YAML_ALIAS_EVENT => {
                yaml_free((*event).data.alias.anchor.cast());
            }
            yaml_event_type_t::YAML_SCALAR_EVENT => {
                yaml_free((*event).data.scalar.anchor.cast());
                yaml_free((*event).data.scalar.tag.cast());
                yaml_free((*event).data.scalar.value.cast());
            }
            yaml_event_type_t::YAML_SEQUENCE_START_EVENT => {
                yaml_free((*event).data.sequence_start.anchor.cast());
                yaml_free((*event).data.sequence_start.tag.cast());
            }
            yaml_event_type_t::YAML_MAPPING_START_EVENT => {
                yaml_free((*event).data.mapping_start.anchor.cast());
                yaml_free((*event).data.mapping_start.tag.cast());
            }
            _ => {}
        }

        zero_event(event);
    });
}
