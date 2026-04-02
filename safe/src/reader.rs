use core::ffi::{c_char, c_int};

use crate::alloc;
use crate::ffi;
use crate::types::{yaml_encoding_t, yaml_error_type_t, yaml_parser_t};
use crate::{PointerExt, FAIL, OK};

const BOM_UTF8: &[u8] = b"\xEF\xBB\xBF";
const BOM_UTF16LE: &[u8] = b"\xFF\xFE";
const BOM_UTF16BE: &[u8] = b"\xFE\xFF";

const INPUT_ERROR: &[u8] = b"input error\0";
const INVALID_LEADING_UTF8: &[u8] = b"invalid leading UTF-8 octet\0";
const INCOMPLETE_UTF8: &[u8] = b"incomplete UTF-8 octet sequence\0";
const INVALID_TRAILING_UTF8: &[u8] = b"invalid trailing UTF-8 octet\0";
const INVALID_UTF8_LENGTH: &[u8] = b"invalid length of a UTF-8 sequence\0";
const INVALID_UNICODE: &[u8] = b"invalid Unicode character\0";
const INCOMPLETE_UTF16: &[u8] = b"incomplete UTF-16 character\0";
const UNEXPECTED_LOW_SURROGATE: &[u8] = b"unexpected low surrogate area\0";
const INCOMPLETE_SURROGATE_PAIR: &[u8] = b"incomplete UTF-16 surrogate pair\0";
const EXPECTED_LOW_SURROGATE: &[u8] = b"expected low surrogate area\0";
const CONTROL_CHARACTERS: &[u8] = b"control characters are not allowed\0";
const INPUT_TOO_LONG: &[u8] = b"input is too long\0";

#[inline]
unsafe fn yaml_parser_set_reader_error(
    parser: *mut yaml_parser_t,
    problem: &'static [u8],
    offset: usize,
    value: c_int,
) -> c_int {
    (*parser).error = yaml_error_type_t::YAML_READER_ERROR;
    (*parser).problem = problem.as_ptr().cast::<c_char>();
    (*parser).problem_offset = offset;
    (*parser).problem_value = value;
    FAIL
}

unsafe fn yaml_parser_determine_encoding(parser: *mut yaml_parser_t) -> c_int {
    while (*parser).eof == 0
        && ((*parser)
            .raw_buffer
            .last
            .c_offset_from((*parser).raw_buffer.pointer) as usize)
            < 3
    {
        if yaml_parser_update_raw_buffer(parser) == FAIL {
            return FAIL;
        }
    }

    let raw_unread = (*parser)
        .raw_buffer
        .last
        .c_offset_from((*parser).raw_buffer.pointer) as usize;
    if raw_unread >= 2
        && alloc::compare_bytes(
            (*parser).raw_buffer.pointer.cast(),
            BOM_UTF16LE.as_ptr().cast(),
            2,
        ) == 0
    {
        (*parser).encoding = yaml_encoding_t::YAML_UTF16LE_ENCODING;
        (*parser).raw_buffer.pointer = (*parser).raw_buffer.pointer.add(2);
        (*parser).offset = (*parser).offset.wrapping_add(2);
    } else if raw_unread >= 2
        && alloc::compare_bytes(
            (*parser).raw_buffer.pointer.cast(),
            BOM_UTF16BE.as_ptr().cast(),
            2,
        ) == 0
    {
        (*parser).encoding = yaml_encoding_t::YAML_UTF16BE_ENCODING;
        (*parser).raw_buffer.pointer = (*parser).raw_buffer.pointer.add(2);
        (*parser).offset = (*parser).offset.wrapping_add(2);
    } else if raw_unread >= 3
        && alloc::compare_bytes(
            (*parser).raw_buffer.pointer.cast(),
            BOM_UTF8.as_ptr().cast(),
            3,
        ) == 0
    {
        (*parser).encoding = yaml_encoding_t::YAML_UTF8_ENCODING;
        (*parser).raw_buffer.pointer = (*parser).raw_buffer.pointer.add(3);
        (*parser).offset = (*parser).offset.wrapping_add(3);
    } else {
        (*parser).encoding = yaml_encoding_t::YAML_UTF8_ENCODING;
    }

    OK
}

unsafe fn yaml_parser_update_raw_buffer(parser: *mut yaml_parser_t) -> c_int {
    let mut size_read = 0usize;

    if (*parser).raw_buffer.start == (*parser).raw_buffer.pointer
        && (*parser).raw_buffer.last == (*parser).raw_buffer.end
    {
        return OK;
    }
    if (*parser).eof != 0 {
        return OK;
    }

    if (*parser).raw_buffer.start < (*parser).raw_buffer.pointer
        && (*parser).raw_buffer.pointer < (*parser).raw_buffer.last
    {
        alloc::move_bytes(
            (*parser).raw_buffer.start.cast(),
            (*parser).raw_buffer.pointer.cast(),
            (*parser)
                .raw_buffer
                .last
                .c_offset_from((*parser).raw_buffer.pointer) as usize,
        );
    }
    (*parser).raw_buffer.last = (*parser).raw_buffer.last.wrapping_offset(
        -((*parser)
            .raw_buffer
            .pointer
            .c_offset_from((*parser).raw_buffer.start)),
    );
    (*parser).raw_buffer.pointer = (*parser).raw_buffer.start;

    let handler = match (*parser).read_handler {
        Some(handler) => handler,
        None => return yaml_parser_set_reader_error(parser, INPUT_ERROR, (*parser).offset, -1),
    };
    if handler(
        (*parser).read_handler_data,
        (*parser).raw_buffer.last,
        (*parser)
            .raw_buffer
            .end
            .c_offset_from((*parser).raw_buffer.last) as usize,
        &mut size_read,
    ) == 0
    {
        return yaml_parser_set_reader_error(parser, INPUT_ERROR, (*parser).offset, -1);
    }

    (*parser).raw_buffer.last = (*parser).raw_buffer.last.add(size_read);
    if size_read == 0 {
        (*parser).eof = 1;
    }

    OK
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_update_buffer(
    parser: *mut yaml_parser_t,
    length: usize,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if parser.is_null() || (*parser).read_handler.is_none() {
            return FAIL;
        }
        if (*parser).offset >= crate::MAX_FILE_SIZE {
            return yaml_parser_set_reader_error(parser, INPUT_TOO_LONG, (*parser).offset, -1);
        }
        if (*parser).eof != 0 && (*parser).raw_buffer.pointer == (*parser).raw_buffer.last {
            return OK;
        }
        if (*parser).unread >= length {
            return OK;
        }
        if (*parser).encoding == yaml_encoding_t::YAML_ANY_ENCODING
            && yaml_parser_determine_encoding(parser) == FAIL
        {
            return FAIL;
        }

        if (*parser).buffer.start < (*parser).buffer.pointer
            && (*parser).buffer.pointer < (*parser).buffer.last
        {
            let size = (*parser)
                .buffer
                .last
                .c_offset_from((*parser).buffer.pointer) as usize;
            alloc::move_bytes(
                (*parser).buffer.start.cast(),
                (*parser).buffer.pointer.cast(),
                size,
            );
            (*parser).buffer.pointer = (*parser).buffer.start;
            (*parser).buffer.last = (*parser).buffer.start.add(size);
        } else if (*parser).buffer.pointer == (*parser).buffer.last {
            (*parser).buffer.pointer = (*parser).buffer.start;
            (*parser).buffer.last = (*parser).buffer.start;
        }

        let mut first = 1;
        while (*parser).unread < length {
            if first == 0 || (*parser).raw_buffer.pointer == (*parser).raw_buffer.last {
                if yaml_parser_update_raw_buffer(parser) == FAIL {
                    return FAIL;
                }
            }
            first = 0;

            while (*parser).raw_buffer.pointer != (*parser).raw_buffer.last {
                let mut value = 0u32;
                let value2: u32;
                let mut incomplete = 0;
                let mut width = 0usize;
                let raw_unread = (*parser)
                    .raw_buffer
                    .last
                    .c_offset_from((*parser).raw_buffer.pointer)
                    as usize;

                match (*parser).encoding {
                    yaml_encoding_t::YAML_UTF8_ENCODING => {
                        let mut octet = *(*parser).raw_buffer.pointer;
                        width = if octet & 0x80 == 0 {
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
                        if width == 0 {
                            return yaml_parser_set_reader_error(
                                parser,
                                INVALID_LEADING_UTF8,
                                (*parser).offset,
                                octet as c_int,
                            );
                        }
                        if width > raw_unread {
                            if (*parser).eof != 0 {
                                return yaml_parser_set_reader_error(
                                    parser,
                                    INCOMPLETE_UTF8,
                                    (*parser).offset,
                                    -1,
                                );
                            }
                            incomplete = 1;
                        } else {
                            value = if octet & 0x80 == 0 {
                                octet & 0x7F
                            } else if octet & 0xE0 == 0xC0 {
                                octet & 0x1F
                            } else if octet & 0xF0 == 0xE0 {
                                octet & 0x0F
                            } else {
                                octet & 0x07
                            } as u32;

                            let mut k = 1usize;
                            while k < width {
                                octet = *(*parser).raw_buffer.pointer.add(k);
                                if octet & 0xC0 != 0x80 {
                                    return yaml_parser_set_reader_error(
                                        parser,
                                        INVALID_TRAILING_UTF8,
                                        (*parser).offset.wrapping_add(k),
                                        octet as c_int,
                                    );
                                }
                                value = (value << 6).wrapping_add((octet & 0x3F) as u32);
                                k += 1;
                            }

                            let valid_length = width == 1
                                || (width == 2 && value >= 0x80)
                                || (width == 3 && value >= 0x800)
                                || (width == 4 && value >= 0x10000);
                            if !valid_length {
                                return yaml_parser_set_reader_error(
                                    parser,
                                    INVALID_UTF8_LENGTH,
                                    (*parser).offset,
                                    -1,
                                );
                            }
                            if (0xD800..=0xDFFF).contains(&value) || value > 0x10FFFF {
                                return yaml_parser_set_reader_error(
                                    parser,
                                    INVALID_UNICODE,
                                    (*parser).offset,
                                    value as c_int,
                                );
                            }
                        }
                    }
                    yaml_encoding_t::YAML_UTF16LE_ENCODING
                    | yaml_encoding_t::YAML_UTF16BE_ENCODING => {
                        let (low, high) =
                            if (*parser).encoding == yaml_encoding_t::YAML_UTF16LE_ENCODING {
                                (0usize, 1usize)
                            } else {
                                (1usize, 0usize)
                            };

                        if raw_unread < 2 {
                            if (*parser).eof != 0 {
                                return yaml_parser_set_reader_error(
                                    parser,
                                    INCOMPLETE_UTF16,
                                    (*parser).offset,
                                    -1,
                                );
                            }
                            incomplete = 1;
                        } else {
                            value = (*(*parser).raw_buffer.pointer.add(low)) as u32
                                + (((*(*parser).raw_buffer.pointer.add(high)) as u32) << 8);
                            if value & 0xFC00 == 0xDC00 {
                                return yaml_parser_set_reader_error(
                                    parser,
                                    UNEXPECTED_LOW_SURROGATE,
                                    (*parser).offset,
                                    value as c_int,
                                );
                            }

                            if value & 0xFC00 == 0xD800 {
                                width = 4;
                                if raw_unread < 4 {
                                    if (*parser).eof != 0 {
                                        return yaml_parser_set_reader_error(
                                            parser,
                                            INCOMPLETE_SURROGATE_PAIR,
                                            (*parser).offset,
                                            -1,
                                        );
                                    }
                                    incomplete = 1;
                                } else {
                                    value2 = (*(*parser).raw_buffer.pointer.add(low + 2)) as u32
                                        + (((*(*parser).raw_buffer.pointer.add(high + 2)) as u32)
                                            << 8);
                                    if value2 & 0xFC00 != 0xDC00 {
                                        return yaml_parser_set_reader_error(
                                            parser,
                                            EXPECTED_LOW_SURROGATE,
                                            (*parser).offset.wrapping_add(2),
                                            value2 as c_int,
                                        );
                                    }
                                    value = 0x10000 + ((value & 0x3FF) << 10) + (value2 & 0x3FF);
                                }
                            } else {
                                width = 2;
                            }
                        }
                    }
                    _ => {}
                }

                if incomplete != 0 {
                    break;
                }

                let printable = value == 0x9
                    || value == 0xA
                    || value == 0xD
                    || (0x20..=0x7E).contains(&value)
                    || value == 0x85
                    || (0xA0..=0xD7FF).contains(&value)
                    || (0xE000..=0xFFFD).contains(&value)
                    || (0x10000..=0x10FFFF).contains(&value);
                if !printable {
                    return yaml_parser_set_reader_error(
                        parser,
                        CONTROL_CHARACTERS,
                        (*parser).offset,
                        value as c_int,
                    );
                }

                (*parser).raw_buffer.pointer = (*parser).raw_buffer.pointer.add(width);
                (*parser).offset = (*parser).offset.wrapping_add(width);

                if value <= 0x7F {
                    *(*parser).buffer.last = value as u8;
                    (*parser).buffer.last = (*parser).buffer.last.add(1);
                } else if value <= 0x7FF {
                    *(*parser).buffer.last = (0xC0 + (value >> 6)) as u8;
                    (*parser).buffer.last = (*parser).buffer.last.add(1);
                    *(*parser).buffer.last = (0x80 + (value & 0x3F)) as u8;
                    (*parser).buffer.last = (*parser).buffer.last.add(1);
                } else if value <= 0xFFFF {
                    *(*parser).buffer.last = (0xE0 + (value >> 12)) as u8;
                    (*parser).buffer.last = (*parser).buffer.last.add(1);
                    *(*parser).buffer.last = (0x80 + ((value >> 6) & 0x3F)) as u8;
                    (*parser).buffer.last = (*parser).buffer.last.add(1);
                    *(*parser).buffer.last = (0x80 + (value & 0x3F)) as u8;
                    (*parser).buffer.last = (*parser).buffer.last.add(1);
                } else {
                    *(*parser).buffer.last = (0xF0 + (value >> 18)) as u8;
                    (*parser).buffer.last = (*parser).buffer.last.add(1);
                    *(*parser).buffer.last = (0x80 + ((value >> 12) & 0x3F)) as u8;
                    (*parser).buffer.last = (*parser).buffer.last.add(1);
                    *(*parser).buffer.last = (0x80 + ((value >> 6) & 0x3F)) as u8;
                    (*parser).buffer.last = (*parser).buffer.last.add(1);
                    *(*parser).buffer.last = (0x80 + (value & 0x3F)) as u8;
                    (*parser).buffer.last = (*parser).buffer.last.add(1);
                }
                (*parser).unread = (*parser).unread.wrapping_add(1);
            }

            if (*parser).eof != 0 {
                *(*parser).buffer.last = 0;
                (*parser).buffer.last = (*parser).buffer.last.add(1);
                (*parser).unread = (*parser).unread.wrapping_add(1);
                if (*parser).offset >= crate::MAX_FILE_SIZE {
                    return yaml_parser_set_reader_error(
                        parser,
                        INPUT_TOO_LONG,
                        (*parser).offset,
                        -1,
                    );
                }
                return OK;
            }
        }

        if (*parser).offset >= crate::MAX_FILE_SIZE {
            return yaml_parser_set_reader_error(parser, INPUT_TOO_LONG, (*parser).offset, -1);
        }
        OK
    })
}
