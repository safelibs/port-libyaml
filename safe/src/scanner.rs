use crate::api::{
    yaml_free, yaml_malloc, yaml_queue_extend, yaml_stack_extend, yaml_string_extend,
    yaml_string_join,
};
use crate::externs::{memcpy, memmove, memset, strcmp, strlen};
use crate::ffi;
use crate::reader::yaml_parser_update_buffer;
use crate::success::{Success, FAIL, OK};
use crate::yaml::{ptrdiff_t, size_t, yaml_char_t, yaml_string_t, NULL_STRING};
use crate::{
    libc, yaml_mark_t, yaml_parser_t, yaml_simple_key_t, yaml_token_t, yaml_token_type_t,
    PointerExt, YAML_ALIAS_TOKEN, YAML_ANCHOR_TOKEN, YAML_BLOCK_END_TOKEN, YAML_BLOCK_ENTRY_TOKEN,
    YAML_BLOCK_MAPPING_START_TOKEN, YAML_BLOCK_SEQUENCE_START_TOKEN, YAML_DOCUMENT_END_TOKEN,
    YAML_DOCUMENT_START_TOKEN, YAML_DOUBLE_QUOTED_SCALAR_STYLE, YAML_FLOW_ENTRY_TOKEN,
    YAML_FLOW_MAPPING_END_TOKEN, YAML_FLOW_MAPPING_START_TOKEN, YAML_FLOW_SEQUENCE_END_TOKEN,
    YAML_FLOW_SEQUENCE_START_TOKEN, YAML_FOLDED_SCALAR_STYLE, YAML_KEY_TOKEN,
    YAML_LITERAL_SCALAR_STYLE, YAML_MEMORY_ERROR, YAML_NO_ERROR, YAML_PLAIN_SCALAR_STYLE,
    YAML_SCALAR_TOKEN, YAML_SCANNER_ERROR, YAML_SINGLE_QUOTED_SCALAR_STYLE, YAML_STREAM_END_TOKEN,
    YAML_STREAM_START_TOKEN, YAML_TAG_DIRECTIVE_TOKEN, YAML_TAG_TOKEN, YAML_VALUE_TOKEN,
    YAML_VERSION_DIRECTIVE_TOKEN,
};
use core::mem::{size_of, MaybeUninit};
use core::ptr::{self, addr_of_mut};

unsafe fn CACHE(parser: *mut yaml_parser_t, length: size_t) -> Success {
    if (*parser).unread >= length {
        OK
    } else if yaml_parser_update_buffer(parser, length) == crate::OK {
        OK
    } else {
        FAIL
    }
}

unsafe fn SKIP(parser: *mut yaml_parser_t) {
    let width = WIDTH!((*parser).buffer);
    (*parser).mark.index = (*parser).mark.index.wrapping_add(1);
    (*parser).mark.column = (*parser).mark.column.wrapping_add(1);
    (*parser).unread = (*parser).unread.wrapping_sub(1);
    (*parser).buffer.pointer = (*parser).buffer.pointer.wrapping_offset(width as isize);
}

unsafe fn SKIP_LINE(parser: *mut yaml_parser_t) {
    if IS_CRLF!((*parser).buffer) {
        (*parser).mark.index = (*parser).mark.index.wrapping_add(2);
        (*parser).mark.column = 0;
        (*parser).mark.line = (*parser).mark.line.wrapping_add(1);
        (*parser).unread = (*parser).unread.wrapping_sub(2);
        (*parser).buffer.pointer = (*parser).buffer.pointer.wrapping_offset(2);
    } else if IS_BREAK!((*parser).buffer) {
        let width = WIDTH!((*parser).buffer);
        (*parser).mark.index = (*parser).mark.index.wrapping_add(1);
        (*parser).mark.column = 0;
        (*parser).mark.line = (*parser).mark.line.wrapping_add(1);
        (*parser).unread = (*parser).unread.wrapping_sub(1);
        (*parser).buffer.pointer = (*parser).buffer.pointer.wrapping_offset(width as isize);
    };
}

unsafe fn READ(parser: *mut yaml_parser_t, string: *mut yaml_string_t) -> Success {
    if STRING_EXTEND!((*parser), *string) == crate::FAIL {
        return FAIL;
    }
    let width = WIDTH!((*parser).buffer);
    COPY!(*string, (*parser).buffer);
    (*parser).mark.index = (*parser).mark.index.wrapping_add(1);
    (*parser).mark.column = (*parser).mark.column.wrapping_add(1);
    (*parser).unread = (*parser).unread.wrapping_sub(1);
    OK
}

unsafe fn READ_LINE(parser: *mut yaml_parser_t, string: *mut yaml_string_t) -> Success {
    if STRING_EXTEND!((*parser), *string) == crate::FAIL {
        return FAIL;
    }
    if CHECK_AT!((*parser).buffer, b'\r', 0) && CHECK_AT!((*parser).buffer, b'\n', 1) {
        *(*string).pointer = b'\n';
        (*string).pointer = (*string).pointer.wrapping_offset(1);
        (*parser).buffer.pointer = (*parser).buffer.pointer.wrapping_offset(2);
        (*parser).mark.index = (*parser).mark.index.wrapping_add(2);
        (*parser).mark.column = 0;
        (*parser).mark.line = (*parser).mark.line.wrapping_add(1);
        (*parser).unread = (*parser).unread.wrapping_sub(2);
    } else if CHECK_AT!((*parser).buffer, b'\r', 0) || CHECK_AT!((*parser).buffer, b'\n', 0) {
        *(*string).pointer = b'\n';
        (*string).pointer = (*string).pointer.wrapping_offset(1);
        (*parser).buffer.pointer = (*parser).buffer.pointer.wrapping_offset(1);
        (*parser).mark.index = (*parser).mark.index.wrapping_add(1);
        (*parser).mark.column = 0;
        (*parser).mark.line = (*parser).mark.line.wrapping_add(1);
        (*parser).unread = (*parser).unread.wrapping_sub(1);
    } else if CHECK_AT!((*parser).buffer, b'\xC2', 0) && CHECK_AT!((*parser).buffer, b'\x85', 1) {
        *(*string).pointer = b'\n';
        (*string).pointer = (*string).pointer.wrapping_offset(1);
        (*parser).buffer.pointer = (*parser).buffer.pointer.wrapping_offset(2);
        (*parser).mark.index = (*parser).mark.index.wrapping_add(1);
        (*parser).mark.column = 0;
        (*parser).mark.line = (*parser).mark.line.wrapping_add(1);
        (*parser).unread = (*parser).unread.wrapping_sub(1);
    } else if CHECK_AT!((*parser).buffer, b'\xE2', 0)
        && CHECK_AT!((*parser).buffer, b'\x80', 1)
        && (CHECK_AT!((*parser).buffer, b'\xA8', 2) || CHECK_AT!((*parser).buffer, b'\xA9', 2))
    {
        *(*string).pointer = *(*parser).buffer.pointer;
        (*string).pointer = (*string).pointer.wrapping_offset(1);
        (*parser).buffer.pointer = (*parser).buffer.pointer.wrapping_offset(1);
        *(*string).pointer = *(*parser).buffer.pointer;
        (*string).pointer = (*string).pointer.wrapping_offset(1);
        (*parser).buffer.pointer = (*parser).buffer.pointer.wrapping_offset(1);
        *(*string).pointer = *(*parser).buffer.pointer;
        (*string).pointer = (*string).pointer.wrapping_offset(1);
        (*parser).buffer.pointer = (*parser).buffer.pointer.wrapping_offset(1);
        (*parser).mark.index = (*parser).mark.index.wrapping_add(1);
        (*parser).mark.column = 0;
        (*parser).mark.line = (*parser).mark.line.wrapping_add(1);
        (*parser).unread = (*parser).unread.wrapping_sub(1);
    };
    OK
}

macro_rules! READ {
    ($parser:expr, $string:expr) => {
        READ($parser, addr_of_mut!($string))
    };
}

macro_rules! READ_LINE {
    ($parser:expr, $string:expr) => {
        READ_LINE($parser, addr_of_mut!($string))
    };
}

unsafe fn enqueue_token(parser: *mut yaml_parser_t, token: *const yaml_token_t) -> Success {
    if ENQUEUE!((*parser), (*parser).tokens, *token) == crate::FAIL {
        FAIL
    } else {
        OK
    }
}

unsafe fn insert_token(
    parser: *mut yaml_parser_t,
    index: size_t,
    token: *const yaml_token_t,
) -> Success {
    if QUEUE_INSERT!((*parser), (*parser).tokens, index, *token) == crate::FAIL {
        FAIL
    } else {
        OK
    }
}

unsafe fn init_string(parser: *mut yaml_parser_t, string: *mut yaml_string_t) -> Success {
    if STRING_INIT!((*parser), *string) == crate::FAIL {
        FAIL
    } else {
        OK
    }
}

unsafe fn extend_string(parser: *mut yaml_parser_t, string: *mut yaml_string_t) -> Success {
    if STRING_EXTEND!((*parser), *string) == crate::FAIL {
        FAIL
    } else {
        OK
    }
}

unsafe fn join_strings(
    parser: *mut yaml_parser_t,
    string_a: *mut yaml_string_t,
    string_b: *mut yaml_string_t,
) -> Success {
    if JOIN!((*parser), *string_a, *string_b) == crate::FAIL {
        FAIL
    } else {
        OK
    }
}

unsafe fn fail_tag_parts(handle: *mut yaml_char_t, suffix: *mut yaml_char_t) -> Success {
    yaml_free(handle as *mut libc::c_void);
    yaml_free(suffix as *mut libc::c_void);
    FAIL
}

/// Scan the input stream and produce the next token.
///
/// Call the function subsequently to produce a sequence of tokens corresponding
/// to the input stream. The initial token has the type YAML_STREAM_START_TOKEN
/// while the ending token has the type YAML_STREAM_END_TOKEN.
///
/// An application is responsible for freeing any buffers associated with the
/// produced token object using the yaml_token_delete function.
///
/// An application must not alternate the calls of yaml_parser_scan() with the
/// calls of yaml_parser_parse() or yaml_parser_load(). Doing this will break
/// the parser.
unsafe fn yaml_parser_scan_impl(parser: *mut yaml_parser_t, token: *mut yaml_token_t) -> Success {
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    if (*parser).stream_end_produced != 0 || (*parser).error != YAML_NO_ERROR {
        return OK;
    }
    if (*parser).token_available == 0 {
        if yaml_parser_fetch_more_tokens_impl(parser).fail {
            return FAIL;
        }
    }
    *token = DEQUEUE!((*parser).tokens);
    (*parser).token_available = 0;
    let fresh2 = addr_of_mut!((*parser).tokens_parsed);
    *fresh2 = (*fresh2).wrapping_add(1);
    if (*token).r#type == YAML_STREAM_END_TOKEN {
        (*parser).stream_end_produced = 1;
    }
    OK
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_scan(
    parser: *mut yaml_parser_t,
    token: *mut yaml_token_t,
) -> core::ffi::c_int {
    ffi::int_boundary(|| unsafe {
        if parser.is_null() || token.is_null() {
            return crate::FAIL;
        }
        if yaml_parser_scan_impl(parser, token).ok {
            crate::OK
        } else {
            crate::FAIL
        }
    })
}

unsafe fn yaml_parser_set_scanner_error(
    mut parser: *mut yaml_parser_t,
    context: *const libc::c_char,
    context_mark: yaml_mark_t,
    problem: *const libc::c_char,
) {
    (*parser).error = YAML_SCANNER_ERROR;
    let fresh3 = addr_of_mut!((*parser).context);
    *fresh3 = context;
    (*parser).context_mark = context_mark;
    let fresh4 = addr_of_mut!((*parser).problem);
    *fresh4 = problem;
    (*parser).problem_mark = (*parser).mark;
}

pub(crate) unsafe fn yaml_parser_fetch_more_tokens_impl(mut parser: *mut yaml_parser_t) -> Success {
    let mut need_more_tokens: libc::c_int;
    loop {
        need_more_tokens = 0;
        if (*parser).tokens.head == (*parser).tokens.tail {
            need_more_tokens = 1;
        } else {
            let mut simple_key: *mut yaml_simple_key_t;
            if yaml_parser_stale_simple_keys(parser).fail {
                return FAIL;
            }
            simple_key = (*parser).simple_keys.start;
            while simple_key != (*parser).simple_keys.top {
                if (*simple_key).possible != 0
                    && (*simple_key).token_number == (*parser).tokens_parsed
                {
                    need_more_tokens = 1;
                    break;
                } else {
                    simple_key = simple_key.wrapping_offset(1);
                }
            }
        }
        if need_more_tokens == 0 {
            break;
        }
        if yaml_parser_fetch_next_token(parser).fail {
            return FAIL;
        }
    }
    (*parser).token_available = 1;
    OK
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_fetch_more_tokens(
    parser: *mut yaml_parser_t,
) -> core::ffi::c_int {
    ffi::int_boundary(|| unsafe {
        if parser.is_null() {
            return crate::FAIL;
        }
        if yaml_parser_fetch_more_tokens_impl(parser).ok {
            crate::OK
        } else {
            crate::FAIL
        }
    })
}

unsafe fn yaml_parser_fetch_next_token(parser: *mut yaml_parser_t) -> Success {
    if CACHE(parser, 1_usize).fail {
        return FAIL;
    }
    if (*parser).stream_start_produced == 0 {
        if yaml_parser_fetch_stream_start(parser).fail {
            return FAIL;
        }
        return OK;
    }
    if yaml_parser_scan_to_next_token(parser).fail {
        return FAIL;
    }
    if yaml_parser_stale_simple_keys(parser).fail {
        return FAIL;
    }
    if yaml_parser_unroll_indent(parser, (*parser).mark.column as ptrdiff_t).fail {
        return FAIL;
    }
    if CACHE(parser, 4_usize).fail {
        return FAIL;
    }
    if IS_Z!((*parser).buffer) {
        return yaml_parser_fetch_stream_end(parser);
    }
    if (*parser).mark.column == 0_usize && CHECK!((*parser).buffer, b'%') {
        return yaml_parser_fetch_directive(parser);
    }
    if (*parser).mark.column == 0_usize
        && CHECK_AT!((*parser).buffer, b'-', 0)
        && CHECK_AT!((*parser).buffer, b'-', 1)
        && CHECK_AT!((*parser).buffer, b'-', 2)
        && IS_BLANKZ_AT!((*parser).buffer, 3)
    {
        return yaml_parser_fetch_document_indicator(parser, YAML_DOCUMENT_START_TOKEN);
    }
    if (*parser).mark.column == 0_usize
        && CHECK_AT!((*parser).buffer, b'.', 0)
        && CHECK_AT!((*parser).buffer, b'.', 1)
        && CHECK_AT!((*parser).buffer, b'.', 2)
        && IS_BLANKZ_AT!((*parser).buffer, 3)
    {
        return yaml_parser_fetch_document_indicator(parser, YAML_DOCUMENT_END_TOKEN);
    }
    if CHECK!((*parser).buffer, b'[') {
        return yaml_parser_fetch_flow_collection_start(parser, YAML_FLOW_SEQUENCE_START_TOKEN);
    }
    if CHECK!((*parser).buffer, b'{') {
        return yaml_parser_fetch_flow_collection_start(parser, YAML_FLOW_MAPPING_START_TOKEN);
    }
    if CHECK!((*parser).buffer, b']') {
        return yaml_parser_fetch_flow_collection_end(parser, YAML_FLOW_SEQUENCE_END_TOKEN);
    }
    if CHECK!((*parser).buffer, b'}') {
        return yaml_parser_fetch_flow_collection_end(parser, YAML_FLOW_MAPPING_END_TOKEN);
    }
    if CHECK!((*parser).buffer, b',') {
        return yaml_parser_fetch_flow_entry(parser);
    }
    if CHECK!((*parser).buffer, b'-') && IS_BLANKZ_AT!((*parser).buffer, 1) {
        return yaml_parser_fetch_block_entry(parser);
    }
    if CHECK!((*parser).buffer, b'?')
        && ((*parser).flow_level != 0 || IS_BLANKZ_AT!((*parser).buffer, 1))
    {
        return yaml_parser_fetch_key(parser);
    }
    if CHECK!((*parser).buffer, b':')
        && ((*parser).flow_level != 0 || IS_BLANKZ_AT!((*parser).buffer, 1))
    {
        return yaml_parser_fetch_value(parser);
    }
    if CHECK!((*parser).buffer, b'*') {
        return yaml_parser_fetch_anchor(parser, YAML_ALIAS_TOKEN);
    }
    if CHECK!((*parser).buffer, b'&') {
        return yaml_parser_fetch_anchor(parser, YAML_ANCHOR_TOKEN);
    }
    if CHECK!((*parser).buffer, b'!') {
        return yaml_parser_fetch_tag(parser);
    }
    if CHECK!((*parser).buffer, b'|') && (*parser).flow_level == 0 {
        return yaml_parser_fetch_block_scalar(parser, true);
    }
    if CHECK!((*parser).buffer, b'>') && (*parser).flow_level == 0 {
        return yaml_parser_fetch_block_scalar(parser, false);
    }
    if CHECK!((*parser).buffer, b'\'') {
        return yaml_parser_fetch_flow_scalar(parser, true);
    }
    if CHECK!((*parser).buffer, b'"') {
        return yaml_parser_fetch_flow_scalar(parser, false);
    }
    if !(IS_BLANKZ!((*parser).buffer)
        || CHECK!((*parser).buffer, b'-')
        || CHECK!((*parser).buffer, b'?')
        || CHECK!((*parser).buffer, b':')
        || CHECK!((*parser).buffer, b',')
        || CHECK!((*parser).buffer, b'[')
        || CHECK!((*parser).buffer, b']')
        || CHECK!((*parser).buffer, b'{')
        || CHECK!((*parser).buffer, b'}')
        || CHECK!((*parser).buffer, b'#')
        || CHECK!((*parser).buffer, b'&')
        || CHECK!((*parser).buffer, b'*')
        || CHECK!((*parser).buffer, b'!')
        || CHECK!((*parser).buffer, b'|')
        || CHECK!((*parser).buffer, b'>')
        || CHECK!((*parser).buffer, b'\'')
        || CHECK!((*parser).buffer, b'"')
        || CHECK!((*parser).buffer, b'%')
        || CHECK!((*parser).buffer, b'@')
        || CHECK!((*parser).buffer, b'`'))
        || CHECK!((*parser).buffer, b'-') && !IS_BLANK_AT!((*parser).buffer, 1)
        || (*parser).flow_level == 0
            && (CHECK!((*parser).buffer, b'?') || CHECK!((*parser).buffer, b':'))
            && !IS_BLANKZ_AT!((*parser).buffer, 1)
    {
        return yaml_parser_fetch_plain_scalar(parser);
    }
    yaml_parser_set_scanner_error(
        parser,
        b"while scanning for the next token\0" as *const u8 as *const libc::c_char,
        (*parser).mark,
        b"found character that cannot start any token\0" as *const u8 as *const libc::c_char,
    );
    FAIL
}

unsafe fn yaml_parser_stale_simple_keys(parser: *mut yaml_parser_t) -> Success {
    let mut simple_key: *mut yaml_simple_key_t;
    simple_key = (*parser).simple_keys.start;
    while simple_key != (*parser).simple_keys.top {
        if (*simple_key).possible != 0
            && ((*simple_key).mark.line < (*parser).mark.line
                || (*simple_key).mark.index.wrapping_add(1024_usize) < (*parser).mark.index)
        {
            if (*simple_key).required != 0 {
                yaml_parser_set_scanner_error(
                    parser,
                    b"while scanning a simple key\0" as *const u8 as *const libc::c_char,
                    (*simple_key).mark,
                    b"could not find expected ':'\0" as *const u8 as *const libc::c_char,
                );
                return FAIL;
            }
            (*simple_key).possible = 0;
        }
        simple_key = simple_key.wrapping_offset(1);
    }
    OK
}

unsafe fn yaml_parser_save_simple_key(parser: *mut yaml_parser_t) -> Success {
    let required = if (*parser).flow_level == 0
        && (*parser).indent as libc::c_long == (*parser).mark.column as ptrdiff_t
    {
        1
    } else {
        0
    };
    if (*parser).simple_key_allowed != 0 {
        let simple_key = yaml_simple_key_t {
            possible: 1,
            required,
            token_number: (*parser)
                .tokens_parsed
                .wrapping_add((*parser).tokens.tail.c_offset_from((*parser).tokens.head)
                    as libc::c_long as libc::c_ulong),
            mark: (*parser).mark,
        };
        if yaml_parser_remove_simple_key(parser).fail {
            return FAIL;
        }
        *(*parser).simple_keys.top.wrapping_offset(-1_isize) = simple_key;
    }
    OK
}

unsafe fn yaml_parser_remove_simple_key(parser: *mut yaml_parser_t) -> Success {
    let mut simple_key: *mut yaml_simple_key_t =
        (*parser).simple_keys.top.wrapping_offset(-1_isize);
    if (*simple_key).possible != 0 {
        if (*simple_key).required != 0 {
            yaml_parser_set_scanner_error(
                parser,
                b"while scanning a simple key\0" as *const u8 as *const libc::c_char,
                (*simple_key).mark,
                b"could not find expected ':'\0" as *const u8 as *const libc::c_char,
            );
            return FAIL;
        }
    }
    (*simple_key).possible = 0;
    OK
}

unsafe fn yaml_parser_increase_flow_level(mut parser: *mut yaml_parser_t) -> Success {
    let empty_simple_key = yaml_simple_key_t {
        possible: 0,
        required: 0,
        token_number: 0_usize,
        mark: yaml_mark_t {
            index: 0_usize,
            line: 0_usize,
            column: 0_usize,
        },
    };
    if PUSH!((*parser), (*parser).simple_keys, empty_simple_key) == crate::FAIL {
        return FAIL;
    }
    if (*parser).flow_level == 2147483647 {
        (*parser).error = YAML_MEMORY_ERROR;
        return FAIL;
    }
    let fresh7 = addr_of_mut!((*parser).flow_level);
    *fresh7 += 1;
    OK
}

unsafe fn yaml_parser_decrease_flow_level(parser: *mut yaml_parser_t) -> Success {
    if (*parser).flow_level != 0 {
        let fresh8 = addr_of_mut!((*parser).flow_level);
        *fresh8 -= 1;
        let _ = POP!((*parser).simple_keys);
    }
    OK
}

unsafe fn yaml_parser_roll_indent(
    mut parser: *mut yaml_parser_t,
    column: ptrdiff_t,
    number: ptrdiff_t,
    r#type: yaml_token_type_t,
    mark: yaml_mark_t,
) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if (*parser).flow_level != 0 {
        return OK;
    }
    if ((*parser).indent as libc::c_long) < column {
        if PUSH!((*parser), (*parser).indents, (*parser).indent) == crate::FAIL {
            return FAIL;
        }
        if column > 2147483647_i64 {
            (*parser).error = YAML_MEMORY_ERROR;
            return FAIL;
        }
        (*parser).indent = column as libc::c_int;
        memset(
            token as *mut libc::c_void,
            0,
            size_of::<yaml_token_t>() as libc::c_ulong,
        );
        (*token).r#type = r#type;
        (*token).start_mark = mark;
        (*token).end_mark = mark;
        if number == -1_i64 {
            if enqueue_token(parser, token).fail {
                return FAIL;
            }
        } else {
            if insert_token(
                parser,
                (number as libc::c_ulong).wrapping_sub((*parser).tokens_parsed),
                token,
            )
            .fail
            {
                return FAIL;
            }
        }
    }
    OK
}

unsafe fn yaml_parser_unroll_indent(mut parser: *mut yaml_parser_t, column: ptrdiff_t) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if (*parser).flow_level != 0 {
        return OK;
    }
    while (*parser).indent as libc::c_long > column {
        memset(
            token as *mut libc::c_void,
            0,
            size_of::<yaml_token_t>() as libc::c_ulong,
        );
        (*token).r#type = YAML_BLOCK_END_TOKEN;
        (*token).start_mark = (*parser).mark;
        (*token).end_mark = (*parser).mark;
        if enqueue_token(parser, token).fail {
            return FAIL;
        }
        (*parser).indent = POP!((*parser).indents);
    }
    OK
}

unsafe fn yaml_parser_fetch_stream_start(mut parser: *mut yaml_parser_t) -> Success {
    let simple_key = yaml_simple_key_t {
        possible: 0,
        required: 0,
        token_number: 0_usize,
        mark: yaml_mark_t {
            index: 0_usize,
            line: 0_usize,
            column: 0_usize,
        },
    };
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    (*parser).indent = -1;
    if PUSH!((*parser), (*parser).simple_keys, simple_key) == crate::FAIL {
        return FAIL;
    }
    (*parser).simple_key_allowed = 1;
    (*parser).stream_start_produced = 1;
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    (*token).r#type = YAML_STREAM_START_TOKEN;
    (*token).start_mark = (*parser).mark;
    (*token).end_mark = (*parser).mark;
    (*token).data.stream_start.encoding = (*parser).encoding;
    enqueue_token(parser, token)
}

unsafe fn yaml_parser_fetch_stream_end(mut parser: *mut yaml_parser_t) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if (*parser).mark.column != 0_usize {
        (*parser).mark.column = 0_usize;
        let fresh22 = addr_of_mut!((*parser).mark.line);
        *fresh22 = (*fresh22).wrapping_add(1);
    }
    if yaml_parser_unroll_indent(parser, -1_i64).fail {
        return FAIL;
    }
    if yaml_parser_remove_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 0;
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    (*token).r#type = YAML_STREAM_END_TOKEN;
    (*token).start_mark = (*parser).mark;
    (*token).end_mark = (*parser).mark;
    enqueue_token(parser, token)
}

unsafe fn yaml_parser_fetch_directive(mut parser: *mut yaml_parser_t) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if yaml_parser_unroll_indent(parser, -1_i64).fail {
        return FAIL;
    }
    if yaml_parser_remove_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 0;
    if yaml_parser_scan_directive(parser, token).fail {
        return FAIL;
    }
    if enqueue_token(parser, token).fail {
        crate::yaml_token_delete(token);
        return FAIL;
    }
    OK
}

unsafe fn yaml_parser_fetch_document_indicator(
    mut parser: *mut yaml_parser_t,
    r#type: yaml_token_type_t,
) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if yaml_parser_unroll_indent(parser, -1_i64).fail {
        return FAIL;
    }
    if yaml_parser_remove_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 0;
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    SKIP(parser);
    SKIP(parser);
    let end_mark: yaml_mark_t = (*parser).mark;
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    (*token).r#type = r#type;
    (*token).start_mark = start_mark;
    (*token).end_mark = end_mark;
    enqueue_token(parser, token)
}

unsafe fn yaml_parser_fetch_flow_collection_start(
    mut parser: *mut yaml_parser_t,
    r#type: yaml_token_type_t,
) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if yaml_parser_save_simple_key(parser).fail {
        return FAIL;
    }
    if yaml_parser_increase_flow_level(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 1;
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    let end_mark: yaml_mark_t = (*parser).mark;
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    (*token).r#type = r#type;
    (*token).start_mark = start_mark;
    (*token).end_mark = end_mark;
    enqueue_token(parser, token)
}

unsafe fn yaml_parser_fetch_flow_collection_end(
    mut parser: *mut yaml_parser_t,
    r#type: yaml_token_type_t,
) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if yaml_parser_remove_simple_key(parser).fail {
        return FAIL;
    }
    if yaml_parser_decrease_flow_level(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 0;
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    let end_mark: yaml_mark_t = (*parser).mark;
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    (*token).r#type = r#type;
    (*token).start_mark = start_mark;
    (*token).end_mark = end_mark;
    enqueue_token(parser, token)
}

unsafe fn yaml_parser_fetch_flow_entry(mut parser: *mut yaml_parser_t) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if yaml_parser_remove_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 1;
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    let end_mark: yaml_mark_t = (*parser).mark;
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    (*token).r#type = YAML_FLOW_ENTRY_TOKEN;
    (*token).start_mark = start_mark;
    (*token).end_mark = end_mark;
    enqueue_token(parser, token)
}

unsafe fn yaml_parser_fetch_block_entry(mut parser: *mut yaml_parser_t) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if (*parser).flow_level == 0 {
        if (*parser).simple_key_allowed == 0 {
            yaml_parser_set_scanner_error(
                parser,
                ptr::null::<libc::c_char>(),
                (*parser).mark,
                b"block sequence entries are not allowed in this context\0" as *const u8
                    as *const libc::c_char,
            );
            return FAIL;
        }
        if yaml_parser_roll_indent(
            parser,
            (*parser).mark.column as ptrdiff_t,
            -1_i64,
            YAML_BLOCK_SEQUENCE_START_TOKEN,
            (*parser).mark,
        )
        .fail
        {
            return FAIL;
        }
    }
    if yaml_parser_remove_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 1;
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    let end_mark: yaml_mark_t = (*parser).mark;
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    (*token).r#type = YAML_BLOCK_ENTRY_TOKEN;
    (*token).start_mark = start_mark;
    (*token).end_mark = end_mark;
    enqueue_token(parser, token)
}

unsafe fn yaml_parser_fetch_key(mut parser: *mut yaml_parser_t) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if (*parser).flow_level == 0 {
        if (*parser).simple_key_allowed == 0 {
            yaml_parser_set_scanner_error(
                parser,
                ptr::null::<libc::c_char>(),
                (*parser).mark,
                b"mapping keys are not allowed in this context\0" as *const u8
                    as *const libc::c_char,
            );
            return FAIL;
        }
        if yaml_parser_roll_indent(
            parser,
            (*parser).mark.column as ptrdiff_t,
            -1_i64,
            YAML_BLOCK_MAPPING_START_TOKEN,
            (*parser).mark,
        )
        .fail
        {
            return FAIL;
        }
    }
    if yaml_parser_remove_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = if (*parser).flow_level == 0 { 1 } else { 0 };
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    let end_mark: yaml_mark_t = (*parser).mark;
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    (*token).r#type = YAML_KEY_TOKEN;
    (*token).start_mark = start_mark;
    (*token).end_mark = end_mark;
    enqueue_token(parser, token)
}

unsafe fn yaml_parser_fetch_value(mut parser: *mut yaml_parser_t) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    let mut simple_key: *mut yaml_simple_key_t =
        (*parser).simple_keys.top.wrapping_offset(-1_isize);
    if (*simple_key).possible != 0 {
        memset(
            token as *mut libc::c_void,
            0,
            size_of::<yaml_token_t>() as libc::c_ulong,
        );
        (*token).r#type = YAML_KEY_TOKEN;
        (*token).start_mark = (*simple_key).mark;
        (*token).end_mark = (*simple_key).mark;
        if insert_token(
            parser,
            ((*simple_key).token_number).wrapping_sub((*parser).tokens_parsed),
            token,
        )
        .fail
        {
            return FAIL;
        }
        if yaml_parser_roll_indent(
            parser,
            (*simple_key).mark.column as ptrdiff_t,
            (*simple_key).token_number as ptrdiff_t,
            YAML_BLOCK_MAPPING_START_TOKEN,
            (*simple_key).mark,
        )
        .fail
        {
            return FAIL;
        }
        (*simple_key).possible = 0;
        (*parser).simple_key_allowed = 0;
    } else {
        if (*parser).flow_level == 0 {
            if (*parser).simple_key_allowed == 0 {
                yaml_parser_set_scanner_error(
                    parser,
                    ptr::null::<libc::c_char>(),
                    (*parser).mark,
                    b"mapping values are not allowed in this context\0" as *const u8
                        as *const libc::c_char,
                );
                return FAIL;
            }
            if yaml_parser_roll_indent(
                parser,
                (*parser).mark.column as ptrdiff_t,
                -1_i64,
                YAML_BLOCK_MAPPING_START_TOKEN,
                (*parser).mark,
            )
            .fail
            {
                return FAIL;
            }
        }
        (*parser).simple_key_allowed = if (*parser).flow_level == 0 { 1 } else { 0 };
    }
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    let end_mark: yaml_mark_t = (*parser).mark;
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    (*token).r#type = YAML_VALUE_TOKEN;
    (*token).start_mark = start_mark;
    (*token).end_mark = end_mark;
    enqueue_token(parser, token)
}

unsafe fn yaml_parser_fetch_anchor(
    mut parser: *mut yaml_parser_t,
    r#type: yaml_token_type_t,
) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if yaml_parser_save_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 0;
    if yaml_parser_scan_anchor(parser, token, r#type).fail {
        return FAIL;
    }
    if enqueue_token(parser, token).fail {
        crate::yaml_token_delete(token);
        return FAIL;
    }
    OK
}

unsafe fn yaml_parser_fetch_tag(mut parser: *mut yaml_parser_t) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if yaml_parser_save_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 0;
    if yaml_parser_scan_tag(parser, token).fail {
        return FAIL;
    }
    if enqueue_token(parser, token).fail {
        crate::yaml_token_delete(token);
        return FAIL;
    }
    OK
}

unsafe fn yaml_parser_fetch_block_scalar(mut parser: *mut yaml_parser_t, literal: bool) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if yaml_parser_remove_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 1;
    if yaml_parser_scan_block_scalar(parser, token, literal).fail {
        return FAIL;
    }
    if enqueue_token(parser, token).fail {
        crate::yaml_token_delete(token);
        return FAIL;
    }
    OK
}

unsafe fn yaml_parser_fetch_flow_scalar(mut parser: *mut yaml_parser_t, single: bool) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if yaml_parser_save_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 0;
    if yaml_parser_scan_flow_scalar(parser, token, single).fail {
        return FAIL;
    }
    if enqueue_token(parser, token).fail {
        crate::yaml_token_delete(token);
        return FAIL;
    }
    OK
}

unsafe fn yaml_parser_fetch_plain_scalar(mut parser: *mut yaml_parser_t) -> Success {
    let mut token = MaybeUninit::<yaml_token_t>::uninit();
    let token = token.as_mut_ptr();
    if yaml_parser_save_simple_key(parser).fail {
        return FAIL;
    }
    (*parser).simple_key_allowed = 0;
    if yaml_parser_scan_plain_scalar(parser, token).fail {
        return FAIL;
    }
    if enqueue_token(parser, token).fail {
        crate::yaml_token_delete(token);
        return FAIL;
    }
    OK
}

unsafe fn yaml_parser_scan_to_next_token(mut parser: *mut yaml_parser_t) -> Success {
    loop {
        if CACHE(parser, 1_usize).fail {
            return FAIL;
        }
        if (*parser).mark.column == 0_usize && IS_BOM!((*parser).buffer) {
            SKIP(parser);
        }
        if CACHE(parser, 1_usize).fail {
            return FAIL;
        }
        while CHECK!((*parser).buffer, b' ')
            || ((*parser).flow_level != 0 || (*parser).simple_key_allowed == 0)
                && CHECK!((*parser).buffer, b'\t')
        {
            SKIP(parser);
            if CACHE(parser, 1_usize).fail {
                return FAIL;
            }
        }
        if CHECK!((*parser).buffer, b'#') {
            while !IS_BREAKZ!((*parser).buffer) {
                SKIP(parser);
                if CACHE(parser, 1_usize).fail {
                    return FAIL;
                }
            }
        }
        if !IS_BREAK!((*parser).buffer) {
            break;
        }
        if CACHE(parser, 2_usize).fail {
            return FAIL;
        }
        SKIP_LINE(parser);
        if (*parser).flow_level == 0 {
            (*parser).simple_key_allowed = 1;
        }
    }
    OK
}

unsafe fn yaml_parser_scan_directive(
    parser: *mut yaml_parser_t,
    mut token: *mut yaml_token_t,
) -> Success {
    let mut current_block: u64;
    let end_mark: yaml_mark_t;
    let mut name: *mut yaml_char_t = ptr::null_mut::<yaml_char_t>();
    let mut major: libc::c_int = 0;
    let mut minor: libc::c_int = 0;
    let mut handle: *mut yaml_char_t = ptr::null_mut::<yaml_char_t>();
    let mut prefix: *mut yaml_char_t = ptr::null_mut::<yaml_char_t>();
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    if yaml_parser_scan_directive_name(parser, start_mark, addr_of_mut!(name)).ok {
        if strcmp(
            name as *mut libc::c_char,
            b"YAML\0" as *const u8 as *const libc::c_char,
        ) == 0
        {
            if yaml_parser_scan_version_directive_value(
                parser,
                start_mark,
                addr_of_mut!(major),
                addr_of_mut!(minor),
            )
            .fail
            {
                current_block = 11397968426844348457;
            } else {
                end_mark = (*parser).mark;
                memset(
                    token as *mut libc::c_void,
                    0,
                    size_of::<yaml_token_t>() as libc::c_ulong,
                );
                (*token).r#type = YAML_VERSION_DIRECTIVE_TOKEN;
                (*token).start_mark = start_mark;
                (*token).end_mark = end_mark;
                (*token).data.version_directive.major = major;
                (*token).data.version_directive.minor = minor;
                current_block = 17407779659766490442;
            }
        } else if strcmp(
            name as *mut libc::c_char,
            b"TAG\0" as *const u8 as *const libc::c_char,
        ) == 0
        {
            if yaml_parser_scan_tag_directive_value(
                parser,
                start_mark,
                addr_of_mut!(handle),
                addr_of_mut!(prefix),
            )
            .fail
            {
                current_block = 11397968426844348457;
            } else {
                end_mark = (*parser).mark;
                memset(
                    token as *mut libc::c_void,
                    0,
                    size_of::<yaml_token_t>() as libc::c_ulong,
                );
                (*token).r#type = YAML_TAG_DIRECTIVE_TOKEN;
                (*token).start_mark = start_mark;
                (*token).end_mark = end_mark;
                let fresh112 = addr_of_mut!((*token).data.tag_directive.handle);
                *fresh112 = handle;
                let fresh113 = addr_of_mut!((*token).data.tag_directive.prefix);
                *fresh113 = prefix;
                current_block = 17407779659766490442;
            }
        } else {
            yaml_parser_set_scanner_error(
                parser,
                b"while scanning a directive\0" as *const u8 as *const libc::c_char,
                start_mark,
                b"found unknown directive name\0" as *const u8 as *const libc::c_char,
            );
            current_block = 11397968426844348457;
        }
        if current_block != 11397968426844348457 {
            if CACHE(parser, 1_usize).ok {
                loop {
                    if !IS_BLANK!((*parser).buffer) {
                        current_block = 11584701595673473500;
                        break;
                    }
                    SKIP(parser);
                    if CACHE(parser, 1_usize).fail {
                        current_block = 11397968426844348457;
                        break;
                    }
                }
                if current_block != 11397968426844348457 {
                    if CHECK!((*parser).buffer, b'#') {
                        loop {
                            if IS_BREAKZ!((*parser).buffer) {
                                current_block = 6669252993407410313;
                                break;
                            }
                            SKIP(parser);
                            if CACHE(parser, 1_usize).fail {
                                current_block = 11397968426844348457;
                                break;
                            }
                        }
                    } else {
                        current_block = 6669252993407410313;
                    }
                    if current_block != 11397968426844348457 {
                        if !IS_BREAKZ!((*parser).buffer) {
                            yaml_parser_set_scanner_error(
                                parser,
                                b"while scanning a directive\0" as *const u8 as *const libc::c_char,
                                start_mark,
                                b"did not find expected comment or line break\0" as *const u8
                                    as *const libc::c_char,
                            );
                        } else {
                            if IS_BREAK!((*parser).buffer) {
                                if CACHE(parser, 2_usize).fail {
                                    current_block = 11397968426844348457;
                                } else {
                                    SKIP_LINE(parser);
                                    current_block = 652864300344834934;
                                }
                            } else {
                                current_block = 652864300344834934;
                            }
                            if current_block != 11397968426844348457 {
                                yaml_free(name as *mut libc::c_void);
                                return OK;
                            }
                        }
                    }
                }
            }
        }
    }
    yaml_free(prefix as *mut libc::c_void);
    yaml_free(handle as *mut libc::c_void);
    yaml_free(name as *mut libc::c_void);
    FAIL
}

unsafe fn yaml_parser_scan_directive_name(
    parser: *mut yaml_parser_t,
    start_mark: yaml_mark_t,
    name: *mut *mut yaml_char_t,
) -> Success {
    let mut string = NULL_STRING;
    if init_string(parser, addr_of_mut!(string)).fail {
        return FAIL;
    }
    if CACHE(parser, 1_usize).fail {
        STRING_DEL!(string);
        return FAIL;
    }
    while IS_ALPHA!((*parser).buffer) {
        if READ!(parser, string).fail {
            STRING_DEL!(string);
            return FAIL;
        }
        if CACHE(parser, 1_usize).fail {
            STRING_DEL!(string);
            return FAIL;
        }
    }
    if string.start == string.pointer {
        yaml_parser_set_scanner_error(
            parser,
            b"while scanning a directive\0" as *const u8 as *const libc::c_char,
            start_mark,
            b"could not find expected directive name\0" as *const u8 as *const libc::c_char,
        );
    } else if !IS_BLANKZ!((*parser).buffer) {
        yaml_parser_set_scanner_error(
            parser,
            b"while scanning a directive\0" as *const u8 as *const libc::c_char,
            start_mark,
            b"found unexpected non-alphabetical character\0" as *const u8 as *const libc::c_char,
        );
    } else {
        *name = string.start;
        return OK;
    }
    STRING_DEL!(string);
    FAIL
}

unsafe fn yaml_parser_scan_version_directive_value(
    parser: *mut yaml_parser_t,
    start_mark: yaml_mark_t,
    major: *mut libc::c_int,
    minor: *mut libc::c_int,
) -> Success {
    if CACHE(parser, 1_usize).fail {
        return FAIL;
    }
    while IS_BLANK!((*parser).buffer) {
        SKIP(parser);
        if CACHE(parser, 1_usize).fail {
            return FAIL;
        }
    }
    if yaml_parser_scan_version_directive_number(parser, start_mark, major).fail {
        return FAIL;
    }
    if !CHECK!((*parser).buffer, b'.') {
        yaml_parser_set_scanner_error(
            parser,
            b"while scanning a %YAML directive\0" as *const u8 as *const libc::c_char,
            start_mark,
            b"did not find expected digit or '.' character\0" as *const u8 as *const libc::c_char,
        );
        return FAIL;
    }
    SKIP(parser);
    yaml_parser_scan_version_directive_number(parser, start_mark, minor)
}

const MAX_NUMBER_LENGTH: size_t = 9_usize;

unsafe fn yaml_parser_scan_version_directive_number(
    parser: *mut yaml_parser_t,
    start_mark: yaml_mark_t,
    number: *mut libc::c_int,
) -> Success {
    let mut value: libc::c_int = 0;
    let mut length: size_t = 0_usize;
    if CACHE(parser, 1_usize).fail {
        return FAIL;
    }
    while IS_DIGIT!((*parser).buffer) {
        length = length.wrapping_add(1);
        if length > MAX_NUMBER_LENGTH {
            yaml_parser_set_scanner_error(
                parser,
                b"while scanning a %YAML directive\0" as *const u8 as *const libc::c_char,
                start_mark,
                b"found extremely long version number\0" as *const u8 as *const libc::c_char,
            );
            return FAIL;
        }
        value = value * 10 + AS_DIGIT!((*parser).buffer);
        SKIP(parser);
        if CACHE(parser, 1_usize).fail {
            return FAIL;
        }
    }
    if length == 0 {
        yaml_parser_set_scanner_error(
            parser,
            b"while scanning a %YAML directive\0" as *const u8 as *const libc::c_char,
            start_mark,
            b"did not find expected version number\0" as *const u8 as *const libc::c_char,
        );
        return FAIL;
    }
    *number = value;
    OK
}

unsafe fn yaml_parser_scan_tag_directive_value(
    parser: *mut yaml_parser_t,
    start_mark: yaml_mark_t,
    handle: *mut *mut yaml_char_t,
    prefix: *mut *mut yaml_char_t,
) -> Success {
    let mut current_block: u64;
    let mut handle_value: *mut yaml_char_t = ptr::null_mut::<yaml_char_t>();
    let mut prefix_value: *mut yaml_char_t = ptr::null_mut::<yaml_char_t>();
    if CACHE(parser, 1_usize).fail {
        current_block = 5231181710497607163;
    } else {
        current_block = 14916268686031723178;
    }
    'c_34337: loop {
        match current_block {
            5231181710497607163 => {
                yaml_free(handle_value as *mut libc::c_void);
                yaml_free(prefix_value as *mut libc::c_void);
                return FAIL;
            }
            _ => {
                if IS_BLANK!((*parser).buffer) {
                    SKIP(parser);
                    if CACHE(parser, 1_usize).fail {
                        current_block = 5231181710497607163;
                    } else {
                        current_block = 14916268686031723178;
                    }
                } else {
                    if yaml_parser_scan_tag_handle(
                        parser,
                        true,
                        start_mark,
                        addr_of_mut!(handle_value),
                    )
                    .fail
                    {
                        current_block = 5231181710497607163;
                        continue;
                    }
                    if CACHE(parser, 1_usize).fail {
                        current_block = 5231181710497607163;
                        continue;
                    }
                    if !IS_BLANK!((*parser).buffer) {
                        yaml_parser_set_scanner_error(
                            parser,
                            b"while scanning a %TAG directive\0" as *const u8
                                as *const libc::c_char,
                            start_mark,
                            b"did not find expected whitespace\0" as *const u8
                                as *const libc::c_char,
                        );
                        current_block = 5231181710497607163;
                    } else {
                        while IS_BLANK!((*parser).buffer) {
                            SKIP(parser);
                            if CACHE(parser, 1_usize).fail {
                                current_block = 5231181710497607163;
                                continue 'c_34337;
                            }
                        }
                        if yaml_parser_scan_tag_uri(
                            parser,
                            true,
                            true,
                            ptr::null_mut::<yaml_char_t>(),
                            start_mark,
                            addr_of_mut!(prefix_value),
                        )
                        .fail
                        {
                            current_block = 5231181710497607163;
                            continue;
                        }
                        if CACHE(parser, 1_usize).fail {
                            current_block = 5231181710497607163;
                            continue;
                        }
                        if !IS_BLANKZ!((*parser).buffer) {
                            yaml_parser_set_scanner_error(
                                parser,
                                b"while scanning a %TAG directive\0" as *const u8
                                    as *const libc::c_char,
                                start_mark,
                                b"did not find expected whitespace or line break\0" as *const u8
                                    as *const libc::c_char,
                            );
                            current_block = 5231181710497607163;
                        } else {
                            *handle = handle_value;
                            *prefix = prefix_value;
                            return OK;
                        }
                    }
                }
            }
        }
    }
}

unsafe fn yaml_parser_scan_anchor(
    parser: *mut yaml_parser_t,
    mut token: *mut yaml_token_t,
    r#type: yaml_token_type_t,
) -> Success {
    let mut length: libc::c_int = 0;
    let mut string = NULL_STRING;
    let end_mark: yaml_mark_t;
    if init_string(parser, addr_of_mut!(string)).fail {
        return FAIL;
    }
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    if CACHE(parser, 1_usize).fail {
        STRING_DEL!(string);
        return FAIL;
    }
    while IS_ALPHA!((*parser).buffer) {
        if READ!(parser, string).fail {
            STRING_DEL!(string);
            return FAIL;
        }
        if CACHE(parser, 1_usize).fail {
            STRING_DEL!(string);
            return FAIL;
        }
        length += 1;
    }
    end_mark = (*parser).mark;
    if length == 0
        || !(IS_BLANKZ!((*parser).buffer)
            || CHECK!((*parser).buffer, b'?')
            || CHECK!((*parser).buffer, b':')
            || CHECK!((*parser).buffer, b',')
            || CHECK!((*parser).buffer, b']')
            || CHECK!((*parser).buffer, b'}')
            || CHECK!((*parser).buffer, b'%')
            || CHECK!((*parser).buffer, b'@')
            || CHECK!((*parser).buffer, b'`'))
    {
        yaml_parser_set_scanner_error(
            parser,
            if r#type == YAML_ANCHOR_TOKEN {
                b"while scanning an anchor\0" as *const u8 as *const libc::c_char
            } else {
                b"while scanning an alias\0" as *const u8 as *const libc::c_char
            },
            start_mark,
            b"did not find expected alphabetic or numeric character\0" as *const u8
                as *const libc::c_char,
        );
    } else {
        memset(
            token as *mut libc::c_void,
            0,
            size_of::<yaml_token_t>() as libc::c_ulong,
        );
        (*token).r#type = r#type;
        (*token).start_mark = start_mark;
        (*token).end_mark = end_mark;
        if r#type == YAML_ANCHOR_TOKEN {
            (*token).data.anchor.value = string.start;
        } else {
            (*token).data.alias.value = string.start;
        }
        return OK;
    }
    STRING_DEL!(string);
    FAIL
}

unsafe fn yaml_parser_scan_tag(
    parser: *mut yaml_parser_t,
    mut token: *mut yaml_token_t,
) -> Success {
    let mut handle: *mut yaml_char_t = ptr::null_mut::<yaml_char_t>();
    let mut suffix: *mut yaml_char_t = ptr::null_mut::<yaml_char_t>();
    let start_mark: yaml_mark_t = (*parser).mark;
    let end_mark: yaml_mark_t;
    if CACHE(parser, 2_usize).fail {
        return fail_tag_parts(handle, suffix);
    }
    if CHECK_AT!((*parser).buffer, b'<', 1) {
        handle = yaml_malloc(1_usize) as *mut yaml_char_t;
        if handle.is_null() {
            (*parser).error = YAML_MEMORY_ERROR;
            return fail_tag_parts(handle, suffix);
        }
        *handle = b'\0';
        SKIP(parser);
        SKIP(parser);
        if yaml_parser_scan_tag_uri(
            parser,
            true,
            false,
            ptr::null_mut::<yaml_char_t>(),
            start_mark,
            addr_of_mut!(suffix),
        )
        .fail
        {
            return fail_tag_parts(handle, suffix);
        }
        if !CHECK!((*parser).buffer, b'>') {
            yaml_parser_set_scanner_error(
                parser,
                b"while scanning a tag\0" as *const u8 as *const libc::c_char,
                start_mark,
                b"did not find the expected '>'\0" as *const u8 as *const libc::c_char,
            );
            return fail_tag_parts(handle, suffix);
        }
        SKIP(parser);
    } else {
        if yaml_parser_scan_tag_handle(parser, false, start_mark, addr_of_mut!(handle)).fail {
            return fail_tag_parts(handle, suffix);
        }
        if *handle == b'!'
            && *handle.wrapping_offset(1_isize) != b'\0'
            && *handle
                .wrapping_offset(strlen(handle as *mut libc::c_char).wrapping_sub(1_usize) as isize)
                == b'!'
        {
            if yaml_parser_scan_tag_uri(
                parser,
                false,
                false,
                ptr::null_mut::<yaml_char_t>(),
                start_mark,
                addr_of_mut!(suffix),
            )
            .fail
            {
                return fail_tag_parts(handle, suffix);
            }
        } else {
            if yaml_parser_scan_tag_uri(
                parser,
                false,
                false,
                handle,
                start_mark,
                addr_of_mut!(suffix),
            )
            .fail
            {
                return fail_tag_parts(handle, suffix);
            }
            yaml_free(handle as *mut libc::c_void);
            handle = yaml_malloc(2_usize) as *mut yaml_char_t;
            if handle.is_null() {
                (*parser).error = YAML_MEMORY_ERROR;
                return fail_tag_parts(handle, suffix);
            }
            *handle = b'!';
            *handle.wrapping_offset(1_isize) = b'\0';
            if *suffix == b'\0' {
                let tmp = handle;
                handle = suffix;
                suffix = tmp;
            }
        }
    }
    if CACHE(parser, 1_usize).fail {
        return fail_tag_parts(handle, suffix);
    }
    if !IS_BLANKZ!((*parser).buffer)
        && ((*parser).flow_level == 0 || !CHECK!((*parser).buffer, b','))
    {
        yaml_parser_set_scanner_error(
            parser,
            b"while scanning a tag\0" as *const u8 as *const libc::c_char,
            start_mark,
            b"did not find expected whitespace or line break\0" as *const u8 as *const libc::c_char,
        );
        return fail_tag_parts(handle, suffix);
    }
    end_mark = (*parser).mark;
    memset(
        token as *mut libc::c_void,
        0,
        size_of::<yaml_token_t>() as libc::c_ulong,
    );
    (*token).r#type = YAML_TAG_TOKEN;
    (*token).start_mark = start_mark;
    (*token).end_mark = end_mark;
    (*token).data.tag.handle = handle;
    (*token).data.tag.suffix = suffix;
    return OK;
}

unsafe fn yaml_parser_scan_tag_handle(
    parser: *mut yaml_parser_t,
    directive: bool,
    start_mark: yaml_mark_t,
    handle: *mut *mut yaml_char_t,
) -> Success {
    let mut string = NULL_STRING;
    if init_string(parser, addr_of_mut!(string)).fail {
        return FAIL;
    }
    if CACHE(parser, 1_usize).fail {
        STRING_DEL!(string);
        return FAIL;
    }
    if !CHECK!((*parser).buffer, b'!') {
        yaml_parser_set_scanner_error(
            parser,
            if directive {
                b"while scanning a tag directive\0" as *const u8 as *const libc::c_char
            } else {
                b"while scanning a tag\0" as *const u8 as *const libc::c_char
            },
            start_mark,
            b"did not find expected '!'\0" as *const u8 as *const libc::c_char,
        );
    } else {
        if READ!(parser, string).fail {
            STRING_DEL!(string);
            return FAIL;
        }
        if CACHE(parser, 1_usize).fail {
            STRING_DEL!(string);
            return FAIL;
        }
        while IS_ALPHA!((*parser).buffer) {
            if READ!(parser, string).fail {
                STRING_DEL!(string);
                return FAIL;
            }
            if CACHE(parser, 1_usize).fail {
                STRING_DEL!(string);
                return FAIL;
            }
        }
        if CHECK!((*parser).buffer, b'!') {
            if READ!(parser, string).fail {
                STRING_DEL!(string);
                return FAIL;
            }
        } else if directive
            && !(*string.start == b'!' && *string.start.wrapping_offset(1_isize) == b'\0')
        {
            yaml_parser_set_scanner_error(
                parser,
                b"while parsing a tag directive\0" as *const u8 as *const libc::c_char,
                start_mark,
                b"did not find expected '!'\0" as *const u8 as *const libc::c_char,
            );
            STRING_DEL!(string);
            return FAIL;
        }
        *handle = string.start;
        return OK;
    }
    STRING_DEL!(string);
    FAIL
}

unsafe fn yaml_parser_scan_tag_uri(
    parser: *mut yaml_parser_t,
    uri_char: bool,
    directive: bool,
    head: *mut yaml_char_t,
    start_mark: yaml_mark_t,
    uri: *mut *mut yaml_char_t,
) -> Success {
    let mut length: size_t = if !head.is_null() {
        strlen(head as *mut libc::c_char)
    } else {
        0_usize
    };
    let mut string = NULL_STRING;
    if init_string(parser, addr_of_mut!(string)).fail {
        return FAIL;
    }
    while string.end.c_offset_from(string.start) as libc::c_long as size_t <= length {
        if yaml_string_extend(
            addr_of_mut!(string.start),
            addr_of_mut!(string.pointer),
            addr_of_mut!(string.end),
        ) == crate::FAIL
        {
            (*parser).error = YAML_MEMORY_ERROR;
            STRING_DEL!(string);
            return FAIL;
        }
    }
    if length > 1_usize {
        memcpy(
            string.start as *mut libc::c_void,
            head.wrapping_offset(1_isize) as *const libc::c_void,
            length.wrapping_sub(1_usize),
        );
        string.pointer = string
            .pointer
            .wrapping_offset(length.wrapping_sub(1_usize) as isize);
    }
    if CACHE(parser, 1_usize).fail {
        STRING_DEL!(string);
        return FAIL;
    }
    while IS_ALPHA!((*parser).buffer)
        || CHECK!((*parser).buffer, b';')
        || CHECK!((*parser).buffer, b'/')
        || CHECK!((*parser).buffer, b'?')
        || CHECK!((*parser).buffer, b':')
        || CHECK!((*parser).buffer, b'@')
        || CHECK!((*parser).buffer, b'&')
        || CHECK!((*parser).buffer, b'=')
        || CHECK!((*parser).buffer, b'+')
        || CHECK!((*parser).buffer, b'$')
        || CHECK!((*parser).buffer, b'.')
        || CHECK!((*parser).buffer, b'%')
        || CHECK!((*parser).buffer, b'!')
        || CHECK!((*parser).buffer, b'~')
        || CHECK!((*parser).buffer, b'*')
        || CHECK!((*parser).buffer, b'\'')
        || CHECK!((*parser).buffer, b'(')
        || CHECK!((*parser).buffer, b')')
        || uri_char
            && (CHECK!((*parser).buffer, b',')
                || CHECK!((*parser).buffer, b'[')
                || CHECK!((*parser).buffer, b']'))
    {
        if CHECK!((*parser).buffer, b'%') {
            if extend_string(parser, addr_of_mut!(string)).fail
                || yaml_parser_scan_uri_escapes(parser, directive, start_mark, addr_of_mut!(string))
                    .fail
            {
                STRING_DEL!(string);
                return FAIL;
            }
        } else if READ!(parser, string).fail {
            STRING_DEL!(string);
            return FAIL;
        }
        length = length.wrapping_add(1);
        if CACHE(parser, 1_usize).fail {
            STRING_DEL!(string);
            return FAIL;
        }
    }
    if length == 0 {
        if extend_string(parser, addr_of_mut!(string)).fail {
            STRING_DEL!(string);
            return FAIL;
        }
        yaml_parser_set_scanner_error(
            parser,
            if directive {
                b"while parsing a %TAG directive\0" as *const u8 as *const libc::c_char
            } else {
                b"while parsing a tag\0" as *const u8 as *const libc::c_char
            },
            start_mark,
            b"did not find expected tag URI\0" as *const u8 as *const libc::c_char,
        );
        STRING_DEL!(string);
        return FAIL;
    }
    *uri = string.start;
    OK
}

unsafe fn yaml_parser_scan_uri_escapes(
    parser: *mut yaml_parser_t,
    directive: bool,
    start_mark: yaml_mark_t,
    string: *mut yaml_string_t,
) -> Success {
    let mut width: libc::c_int = 0;
    loop {
        if CACHE(parser, 3_usize).fail {
            return FAIL;
        }
        if !(CHECK!((*parser).buffer, b'%')
            && IS_HEX_AT!((*parser).buffer, 1)
            && IS_HEX_AT!((*parser).buffer, 2))
        {
            yaml_parser_set_scanner_error(
                parser,
                if directive {
                    b"while parsing a %TAG directive\0" as *const u8 as *const libc::c_char
                } else {
                    b"while parsing a tag\0" as *const u8 as *const libc::c_char
                },
                start_mark,
                b"did not find URI escaped octet\0" as *const u8 as *const libc::c_char,
            );
            return FAIL;
        }
        let octet: libc::c_uchar = ((AS_HEX_AT!((*parser).buffer, 1) << 4)
            + AS_HEX_AT!((*parser).buffer, 2)) as libc::c_uchar;
        if width == 0 {
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
                yaml_parser_set_scanner_error(
                    parser,
                    if directive {
                        b"while parsing a %TAG directive\0" as *const u8 as *const libc::c_char
                    } else {
                        b"while parsing a tag\0" as *const u8 as *const libc::c_char
                    },
                    start_mark,
                    b"found an incorrect leading UTF-8 octet\0" as *const u8 as *const libc::c_char,
                );
                return FAIL;
            }
        } else if octet & 0xC0 != 0x80 {
            yaml_parser_set_scanner_error(
                parser,
                if directive {
                    b"while parsing a %TAG directive\0" as *const u8 as *const libc::c_char
                } else {
                    b"while parsing a tag\0" as *const u8 as *const libc::c_char
                },
                start_mark,
                b"found an incorrect trailing UTF-8 octet\0" as *const u8 as *const libc::c_char,
            );
            return FAIL;
        }
        let fresh368 = addr_of_mut!((*string).pointer);
        let fresh369 = *fresh368;
        *fresh368 = (*fresh368).wrapping_offset(1);
        *fresh369 = octet;
        SKIP(parser);
        SKIP(parser);
        SKIP(parser);
        width -= 1;
        if !(width != 0) {
            break;
        }
    }
    OK
}

unsafe fn yaml_parser_scan_block_scalar(
    parser: *mut yaml_parser_t,
    mut token: *mut yaml_token_t,
    literal: bool,
) -> Success {
    let mut current_block: u64;
    let mut end_mark: yaml_mark_t;
    let mut string = NULL_STRING;
    let mut leading_break = NULL_STRING;
    let mut trailing_breaks = NULL_STRING;
    let mut chomping: libc::c_int = 0;
    let mut increment: libc::c_int = 0;
    let mut indent: libc::c_int = 0;
    let mut leading_blank: libc::c_int = 0;
    let mut trailing_blank: libc::c_int;
    if init_string(parser, addr_of_mut!(string)).fail {
        return FAIL;
    }
    if init_string(parser, addr_of_mut!(leading_break)).fail {
        STRING_DEL!(string);
        return FAIL;
    }
    if init_string(parser, addr_of_mut!(trailing_breaks)).fail {
        STRING_DEL!(string);
        STRING_DEL!(leading_break);
        return FAIL;
    }
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    if CACHE(parser, 1_usize).ok {
        if CHECK!((*parser).buffer, b'+') || CHECK!((*parser).buffer, b'-') {
            chomping = if CHECK!((*parser).buffer, b'+') {
                1
            } else {
                -1
            };
            SKIP(parser);
            if CACHE(parser, 1_usize).fail {
                current_block = 14984465786483313892;
            } else if IS_DIGIT!((*parser).buffer) {
                if CHECK!((*parser).buffer, b'0') {
                    yaml_parser_set_scanner_error(
                        parser,
                        b"while scanning a block scalar\0" as *const u8 as *const libc::c_char,
                        start_mark,
                        b"found an indentation indicator equal to 0\0" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 14984465786483313892;
                } else {
                    increment = AS_DIGIT!((*parser).buffer);
                    SKIP(parser);
                    current_block = 11913429853522160501;
                }
            } else {
                current_block = 11913429853522160501;
            }
        } else if IS_DIGIT!((*parser).buffer) {
            if CHECK!((*parser).buffer, b'0') {
                yaml_parser_set_scanner_error(
                    parser,
                    b"while scanning a block scalar\0" as *const u8 as *const libc::c_char,
                    start_mark,
                    b"found an indentation indicator equal to 0\0" as *const u8
                        as *const libc::c_char,
                );
                current_block = 14984465786483313892;
            } else {
                increment = AS_DIGIT!((*parser).buffer);
                SKIP(parser);
                if CACHE(parser, 1_usize).fail {
                    current_block = 14984465786483313892;
                } else {
                    if CHECK!((*parser).buffer, b'+') || CHECK!((*parser).buffer, b'-') {
                        chomping = if CHECK!((*parser).buffer, b'+') {
                            1
                        } else {
                            -1
                        };
                        SKIP(parser);
                    }
                    current_block = 11913429853522160501;
                }
            }
        } else {
            current_block = 11913429853522160501;
        }
        if current_block != 14984465786483313892 {
            if CACHE(parser, 1_usize).ok {
                loop {
                    if !IS_BLANK!((*parser).buffer) {
                        current_block = 4090602189656566074;
                        break;
                    }
                    SKIP(parser);
                    if CACHE(parser, 1_usize).fail {
                        current_block = 14984465786483313892;
                        break;
                    }
                }
                if current_block != 14984465786483313892 {
                    if CHECK!((*parser).buffer, b'#') {
                        loop {
                            if IS_BREAKZ!((*parser).buffer) {
                                current_block = 12997042908615822766;
                                break;
                            }
                            SKIP(parser);
                            if CACHE(parser, 1_usize).fail {
                                current_block = 14984465786483313892;
                                break;
                            }
                        }
                    } else {
                        current_block = 12997042908615822766;
                    }
                    if current_block != 14984465786483313892 {
                        if !IS_BREAKZ!((*parser).buffer) {
                            yaml_parser_set_scanner_error(
                                parser,
                                b"while scanning a block scalar\0" as *const u8
                                    as *const libc::c_char,
                                start_mark,
                                b"did not find expected comment or line break\0" as *const u8
                                    as *const libc::c_char,
                            );
                        } else {
                            if IS_BREAK!((*parser).buffer) {
                                if CACHE(parser, 2_usize).fail {
                                    current_block = 14984465786483313892;
                                } else {
                                    SKIP_LINE(parser);
                                    current_block = 13619784596304402172;
                                }
                            } else {
                                current_block = 13619784596304402172;
                            }
                            if current_block != 14984465786483313892 {
                                end_mark = (*parser).mark;
                                if increment != 0 {
                                    indent = if (*parser).indent >= 0 {
                                        (*parser).indent + increment
                                    } else {
                                        increment
                                    };
                                }
                                if yaml_parser_scan_block_scalar_breaks(
                                    parser,
                                    addr_of_mut!(indent),
                                    addr_of_mut!(trailing_breaks),
                                    start_mark,
                                    addr_of_mut!(end_mark),
                                )
                                .ok
                                {
                                    if CACHE(parser, 1_usize).ok {
                                        's_281: loop {
                                            if !((*parser).mark.column as libc::c_int == indent
                                                && !IS_Z!((*parser).buffer))
                                            {
                                                current_block = 5793491756164225964;
                                                break;
                                            }
                                            trailing_blank =
                                                IS_BLANK!((*parser).buffer) as libc::c_int;
                                            if !literal
                                                && *leading_break.start == b'\n'
                                                && leading_blank == 0
                                                && trailing_blank == 0
                                            {
                                                if *trailing_breaks.start == b'\0' {
                                                    if extend_string(parser, addr_of_mut!(string))
                                                        .fail
                                                    {
                                                        current_block = 14984465786483313892;
                                                        break 's_281;
                                                    }
                                                    let fresh418 = string.pointer;
                                                    string.pointer =
                                                        string.pointer.wrapping_offset(1);
                                                    *fresh418 = b' ';
                                                }
                                                CLEAR!(leading_break);
                                            } else {
                                                if join_strings(
                                                    parser,
                                                    addr_of_mut!(string),
                                                    addr_of_mut!(leading_break),
                                                )
                                                .fail
                                                {
                                                    current_block = 14984465786483313892;
                                                    break 's_281;
                                                }
                                                CLEAR!(leading_break);
                                            }
                                            if join_strings(
                                                parser,
                                                addr_of_mut!(string),
                                                addr_of_mut!(trailing_breaks),
                                            )
                                            .fail
                                            {
                                                current_block = 14984465786483313892;
                                                break 's_281;
                                            }
                                            CLEAR!(trailing_breaks);
                                            leading_blank =
                                                IS_BLANK!((*parser).buffer) as libc::c_int;
                                            while !IS_BREAKZ!((*parser).buffer) {
                                                if READ!(parser, string).fail {
                                                    current_block = 14984465786483313892;
                                                    break 's_281;
                                                }
                                                if CACHE(parser, 1_usize).fail {
                                                    current_block = 14984465786483313892;
                                                    break 's_281;
                                                }
                                            }
                                            if CACHE(parser, 2_usize).fail {
                                                current_block = 14984465786483313892;
                                                break;
                                            }
                                            if READ_LINE!(parser, leading_break).fail {
                                                current_block = 14984465786483313892;
                                                break 's_281;
                                            }
                                            if yaml_parser_scan_block_scalar_breaks(
                                                parser,
                                                addr_of_mut!(indent),
                                                addr_of_mut!(trailing_breaks),
                                                start_mark,
                                                addr_of_mut!(end_mark),
                                            )
                                            .fail
                                            {
                                                current_block = 14984465786483313892;
                                                break;
                                            }
                                        }
                                        if current_block != 14984465786483313892 {
                                            if chomping != -1 {
                                                if join_strings(
                                                    parser,
                                                    addr_of_mut!(string),
                                                    addr_of_mut!(leading_break),
                                                )
                                                .fail
                                                {
                                                    current_block = 14984465786483313892;
                                                } else {
                                                    current_block = 17787701279558130514;
                                                }
                                            } else {
                                                current_block = 17787701279558130514;
                                            }
                                            if current_block != 14984465786483313892 {
                                                if chomping == 1 {
                                                    if join_strings(
                                                        parser,
                                                        addr_of_mut!(string),
                                                        addr_of_mut!(trailing_breaks),
                                                    )
                                                    .fail
                                                    {
                                                        current_block = 14984465786483313892;
                                                    }
                                                }
                                                if current_block != 14984465786483313892 {
                                                    memset(
                                                        token as *mut libc::c_void,
                                                        0,
                                                        size_of::<yaml_token_t>() as libc::c_ulong,
                                                    );
                                                    (*token).r#type = YAML_SCALAR_TOKEN;
                                                    (*token).start_mark = start_mark;
                                                    (*token).end_mark = end_mark;
                                                    let fresh479 =
                                                        addr_of_mut!((*token).data.scalar.value);
                                                    *fresh479 = string.start;
                                                    (*token).data.scalar.length =
                                                        string.pointer.c_offset_from(string.start)
                                                            as libc::c_long
                                                            as size_t;
                                                    (*token).data.scalar.style = if literal {
                                                        YAML_LITERAL_SCALAR_STYLE
                                                    } else {
                                                        YAML_FOLDED_SCALAR_STYLE
                                                    };
                                                    STRING_DEL!(leading_break);
                                                    STRING_DEL!(trailing_breaks);
                                                    return OK;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    STRING_DEL!(string);
    STRING_DEL!(leading_break);
    STRING_DEL!(trailing_breaks);
    FAIL
}

unsafe fn yaml_parser_scan_block_scalar_breaks(
    parser: *mut yaml_parser_t,
    indent: *mut libc::c_int,
    breaks: *mut yaml_string_t,
    start_mark: yaml_mark_t,
    end_mark: *mut yaml_mark_t,
) -> Success {
    let mut max_indent: libc::c_int = 0;
    *end_mark = (*parser).mark;
    loop {
        if CACHE(parser, 1_usize).fail {
            return FAIL;
        }
        while (*indent == 0 || ((*parser).mark.column as libc::c_int) < *indent)
            && IS_SPACE!((*parser).buffer)
        {
            SKIP(parser);
            if CACHE(parser, 1_usize).fail {
                return FAIL;
            }
        }
        if (*parser).mark.column as libc::c_int > max_indent {
            max_indent = (*parser).mark.column as libc::c_int;
        }
        if (*indent == 0 || ((*parser).mark.column as libc::c_int) < *indent)
            && IS_TAB!((*parser).buffer)
        {
            yaml_parser_set_scanner_error(
                parser,
                b"while scanning a block scalar\0" as *const u8 as *const libc::c_char,
                start_mark,
                b"found a tab character where an indentation space is expected\0" as *const u8
                    as *const libc::c_char,
            );
            return FAIL;
        }
        if !IS_BREAK!((*parser).buffer) {
            break;
        }
        if CACHE(parser, 2_usize).fail {
            return FAIL;
        }
        if READ_LINE!(parser, *breaks).fail {
            return FAIL;
        }
        *end_mark = (*parser).mark;
    }
    if *indent == 0 {
        *indent = max_indent;
        if *indent < (*parser).indent + 1 {
            *indent = (*parser).indent + 1;
        }
        if *indent < 1 {
            *indent = 1;
        }
    }
    OK
}

unsafe fn yaml_parser_scan_flow_scalar(
    parser: *mut yaml_parser_t,
    mut token: *mut yaml_token_t,
    single: bool,
) -> Success {
    let mut current_block: u64;
    let end_mark: yaml_mark_t;
    let mut string = NULL_STRING;
    let mut leading_break = NULL_STRING;
    let mut trailing_breaks = NULL_STRING;
    let mut whitespaces = NULL_STRING;
    let mut leading_blanks: libc::c_int;
    if init_string(parser, addr_of_mut!(string)).fail {
        return FAIL;
    }
    if init_string(parser, addr_of_mut!(leading_break)).fail {
        STRING_DEL!(string);
        return FAIL;
    }
    if init_string(parser, addr_of_mut!(trailing_breaks)).fail {
        STRING_DEL!(string);
        STRING_DEL!(leading_break);
        return FAIL;
    }
    if init_string(parser, addr_of_mut!(whitespaces)).fail {
        STRING_DEL!(string);
        STRING_DEL!(leading_break);
        STRING_DEL!(trailing_breaks);
        return FAIL;
    }
    let start_mark: yaml_mark_t = (*parser).mark;
    SKIP(parser);
    's_58: loop {
        if CACHE(parser, 4_usize).fail {
            current_block = 8114179180390253173;
            break;
        }
        if (*parser).mark.column == 0_usize
            && (CHECK_AT!((*parser).buffer, b'-', 0)
                && CHECK_AT!((*parser).buffer, b'-', 1)
                && CHECK_AT!((*parser).buffer, b'-', 2)
                || CHECK_AT!((*parser).buffer, b'.', 0)
                    && CHECK_AT!((*parser).buffer, b'.', 1)
                    && CHECK_AT!((*parser).buffer, b'.', 2))
            && IS_BLANKZ_AT!((*parser).buffer, 3)
        {
            yaml_parser_set_scanner_error(
                parser,
                b"while scanning a quoted scalar\0" as *const u8 as *const libc::c_char,
                start_mark,
                b"found unexpected document indicator\0" as *const u8 as *const libc::c_char,
            );
            current_block = 8114179180390253173;
            break;
        } else if IS_Z!((*parser).buffer) {
            yaml_parser_set_scanner_error(
                parser,
                b"while scanning a quoted scalar\0" as *const u8 as *const libc::c_char,
                start_mark,
                b"found unexpected end of stream\0" as *const u8 as *const libc::c_char,
            );
            current_block = 8114179180390253173;
            break;
        } else {
            if CACHE(parser, 2_usize).fail {
                current_block = 8114179180390253173;
                break;
            }
            leading_blanks = 0;
            while !IS_BLANKZ!((*parser).buffer) {
                if single
                    && CHECK_AT!((*parser).buffer, b'\'', 0)
                    && CHECK_AT!((*parser).buffer, b'\'', 1)
                {
                    if extend_string(parser, addr_of_mut!(string)).fail {
                        current_block = 8114179180390253173;
                        break 's_58;
                    }
                    let fresh521 = string.pointer;
                    string.pointer = string.pointer.wrapping_offset(1);
                    *fresh521 = b'\'';
                    SKIP(parser);
                    SKIP(parser);
                } else {
                    if CHECK!((*parser).buffer, if single { b'\'' } else { b'"' }) {
                        break;
                    }
                    if !single
                        && CHECK!((*parser).buffer, b'\\')
                        && IS_BREAK_AT!((*parser).buffer, 1)
                    {
                        if CACHE(parser, 3_usize).fail {
                            current_block = 8114179180390253173;
                            break 's_58;
                        }
                        SKIP(parser);
                        SKIP_LINE(parser);
                        leading_blanks = 1;
                        break;
                    } else if !single && CHECK!((*parser).buffer, b'\\') {
                        let mut code_length: size_t = 0_usize;
                        if extend_string(parser, addr_of_mut!(string)).fail {
                            current_block = 8114179180390253173;
                            break 's_58;
                        }
                        match *(*parser).buffer.pointer.wrapping_offset(1_isize) {
                            48 => {
                                let fresh542 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh542 = b'\0';
                            }
                            97 => {
                                let fresh543 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh543 = b'\x07';
                            }
                            98 => {
                                let fresh544 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh544 = b'\x08';
                            }
                            116 | 9 => {
                                let fresh545 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh545 = b'\t';
                            }
                            110 => {
                                let fresh546 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh546 = b'\n';
                            }
                            118 => {
                                let fresh547 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh547 = b'\x0B';
                            }
                            102 => {
                                let fresh548 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh548 = b'\x0C';
                            }
                            114 => {
                                let fresh549 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh549 = b'\r';
                            }
                            101 => {
                                let fresh550 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh550 = b'\x1B';
                            }
                            32 => {
                                let fresh551 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh551 = b' ';
                            }
                            34 => {
                                let fresh552 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh552 = b'"';
                            }
                            47 => {
                                let fresh553 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh553 = b'/';
                            }
                            92 => {
                                let fresh554 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh554 = b'\\';
                            }
                            78 => {
                                let fresh555 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh555 = b'\xC2';
                                let fresh556 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh556 = b'\x85';
                            }
                            95 => {
                                let fresh557 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh557 = b'\xC2';
                                let fresh558 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh558 = b'\xA0';
                            }
                            76 => {
                                let fresh559 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh559 = b'\xE2';
                                let fresh560 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh560 = b'\x80';
                                let fresh561 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh561 = b'\xA8';
                            }
                            80 => {
                                let fresh562 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh562 = b'\xE2';
                                let fresh563 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh563 = b'\x80';
                                let fresh564 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh564 = b'\xA9';
                            }
                            120 => {
                                code_length = 2_usize;
                            }
                            117 => {
                                code_length = 4_usize;
                            }
                            85 => {
                                code_length = 8_usize;
                            }
                            _ => {
                                yaml_parser_set_scanner_error(
                                    parser,
                                    b"while parsing a quoted scalar\0" as *const u8
                                        as *const libc::c_char,
                                    start_mark,
                                    b"found unknown escape character\0" as *const u8
                                        as *const libc::c_char,
                                );
                                current_block = 8114179180390253173;
                                break 's_58;
                            }
                        }
                        SKIP(parser);
                        SKIP(parser);
                        if code_length != 0 {
                            let mut value: libc::c_uint = 0;
                            let mut k: size_t;
                            if CACHE(parser, code_length).fail {
                                current_block = 8114179180390253173;
                                break 's_58;
                            }
                            k = 0_usize;
                            while k < code_length {
                                if !IS_HEX_AT!((*parser).buffer, k as isize) {
                                    yaml_parser_set_scanner_error(
                                        parser,
                                        b"while parsing a quoted scalar\0" as *const u8
                                            as *const libc::c_char,
                                        start_mark,
                                        b"did not find expected hexdecimal number\0" as *const u8
                                            as *const libc::c_char,
                                    );
                                    current_block = 8114179180390253173;
                                    break 's_58;
                                } else {
                                    value = (value << 4).wrapping_add(AS_HEX_AT!(
                                        (*parser).buffer,
                                        k as isize
                                    )
                                        as libc::c_uint);
                                    k = k.wrapping_add(1);
                                }
                            }
                            if value >= 0xD800 && value <= 0xDFFF || value > 0x10FFFF {
                                yaml_parser_set_scanner_error(
                                    parser,
                                    b"while parsing a quoted scalar\0" as *const u8
                                        as *const libc::c_char,
                                    start_mark,
                                    b"found invalid Unicode character escape code\0" as *const u8
                                        as *const libc::c_char,
                                );
                                current_block = 8114179180390253173;
                                break 's_58;
                            } else {
                                if value <= 0x7F {
                                    let fresh573 = string.pointer;
                                    string.pointer = string.pointer.wrapping_offset(1);
                                    *fresh573 = value as yaml_char_t;
                                } else if value <= 0x7FF {
                                    let fresh574 = string.pointer;
                                    string.pointer = string.pointer.wrapping_offset(1);
                                    *fresh574 = 0xC0_u32.wrapping_add(value >> 6) as yaml_char_t;
                                    let fresh575 = string.pointer;
                                    string.pointer = string.pointer.wrapping_offset(1);
                                    *fresh575 = 0x80_u32.wrapping_add(value & 0x3F) as yaml_char_t;
                                } else if value <= 0xFFFF {
                                    let fresh576 = string.pointer;
                                    string.pointer = string.pointer.wrapping_offset(1);
                                    *fresh576 = 0xE0_u32.wrapping_add(value >> 12) as yaml_char_t;
                                    let fresh577 = string.pointer;
                                    string.pointer = string.pointer.wrapping_offset(1);
                                    *fresh577 =
                                        0x80_u32.wrapping_add(value >> 6 & 0x3F) as yaml_char_t;
                                    let fresh578 = string.pointer;
                                    string.pointer = string.pointer.wrapping_offset(1);
                                    *fresh578 = 0x80_u32.wrapping_add(value & 0x3F) as yaml_char_t;
                                } else {
                                    let fresh579 = string.pointer;
                                    string.pointer = string.pointer.wrapping_offset(1);
                                    *fresh579 = 0xF0_u32.wrapping_add(value >> 18) as yaml_char_t;
                                    let fresh580 = string.pointer;
                                    string.pointer = string.pointer.wrapping_offset(1);
                                    *fresh580 =
                                        0x80_u32.wrapping_add(value >> 12 & 0x3F) as yaml_char_t;
                                    let fresh581 = string.pointer;
                                    string.pointer = string.pointer.wrapping_offset(1);
                                    *fresh581 =
                                        0x80_u32.wrapping_add(value >> 6 & 0x3F) as yaml_char_t;
                                    let fresh582 = string.pointer;
                                    string.pointer = string.pointer.wrapping_offset(1);
                                    *fresh582 = 0x80_u32.wrapping_add(value & 0x3F) as yaml_char_t;
                                }
                                k = 0_usize;
                                while k < code_length {
                                    SKIP(parser);
                                    k = k.wrapping_add(1);
                                }
                            }
                        }
                    } else {
                        if READ!(parser, string).fail {
                            current_block = 8114179180390253173;
                            break 's_58;
                        }
                    }
                }
                if CACHE(parser, 2_usize).fail {
                    current_block = 8114179180390253173;
                    break 's_58;
                }
            }
            if CACHE(parser, 1_usize).fail {
                current_block = 8114179180390253173;
                break;
            }
            if CHECK!((*parser).buffer, if single { b'\'' } else { b'"' }) {
                current_block = 7468767852762055642;
                break;
            }
            if CACHE(parser, 1_usize).fail {
                current_block = 8114179180390253173;
                break;
            }
            while IS_BLANK!((*parser).buffer) || IS_BREAK!((*parser).buffer) {
                if IS_BLANK!((*parser).buffer) {
                    if leading_blanks == 0 {
                        if READ!(parser, whitespaces).fail {
                            current_block = 8114179180390253173;
                            break 's_58;
                        }
                    } else {
                        SKIP(parser);
                    }
                } else {
                    if CACHE(parser, 2_usize).fail {
                        current_block = 8114179180390253173;
                        break 's_58;
                    }
                    if leading_blanks == 0 {
                        CLEAR!(whitespaces);
                        if READ_LINE!(parser, leading_break).fail {
                            current_block = 8114179180390253173;
                            break 's_58;
                        }
                        leading_blanks = 1;
                    } else {
                        if READ_LINE!(parser, trailing_breaks).fail {
                            current_block = 8114179180390253173;
                            break 's_58;
                        }
                    }
                }
                if CACHE(parser, 1_usize).fail {
                    current_block = 8114179180390253173;
                    break 's_58;
                }
            }
            if leading_blanks != 0 {
                if *leading_break.start == b'\n' {
                    if *trailing_breaks.start == b'\0' {
                        if extend_string(parser, addr_of_mut!(string)).fail {
                            current_block = 8114179180390253173;
                            break 's_58;
                        }
                        let fresh711 = string.pointer;
                        string.pointer = string.pointer.wrapping_offset(1);
                        *fresh711 = b' ';
                    } else {
                        if join_strings(parser, addr_of_mut!(string), addr_of_mut!(trailing_breaks))
                            .fail
                        {
                            current_block = 8114179180390253173;
                            break 's_58;
                        }
                        CLEAR!(trailing_breaks);
                    }
                    CLEAR!(leading_break);
                } else {
                    if join_strings(parser, addr_of_mut!(string), addr_of_mut!(leading_break)).fail
                    {
                        current_block = 8114179180390253173;
                        break 's_58;
                    }
                    if join_strings(parser, addr_of_mut!(string), addr_of_mut!(trailing_breaks))
                        .fail
                    {
                        current_block = 8114179180390253173;
                        break 's_58;
                    }
                    CLEAR!(leading_break);
                    CLEAR!(trailing_breaks);
                }
            } else {
                if join_strings(parser, addr_of_mut!(string), addr_of_mut!(whitespaces)).fail {
                    current_block = 8114179180390253173;
                    break 's_58;
                }
                CLEAR!(whitespaces);
            }
        }
    }
    if current_block != 8114179180390253173 {
        SKIP(parser);
        end_mark = (*parser).mark;
        memset(
            token as *mut libc::c_void,
            0,
            size_of::<yaml_token_t>() as libc::c_ulong,
        );
        (*token).r#type = YAML_SCALAR_TOKEN;
        (*token).start_mark = start_mark;
        (*token).end_mark = end_mark;
        let fresh716 = addr_of_mut!((*token).data.scalar.value);
        *fresh716 = string.start;
        (*token).data.scalar.length =
            string.pointer.c_offset_from(string.start) as libc::c_long as size_t;
        (*token).data.scalar.style = if single {
            YAML_SINGLE_QUOTED_SCALAR_STYLE
        } else {
            YAML_DOUBLE_QUOTED_SCALAR_STYLE
        };
        STRING_DEL!(leading_break);
        STRING_DEL!(trailing_breaks);
        STRING_DEL!(whitespaces);
        return OK;
    }
    STRING_DEL!(string);
    STRING_DEL!(leading_break);
    STRING_DEL!(trailing_breaks);
    STRING_DEL!(whitespaces);
    FAIL
}

unsafe fn yaml_parser_scan_plain_scalar(
    mut parser: *mut yaml_parser_t,
    mut token: *mut yaml_token_t,
) -> Success {
    let mut current_block: u64;
    let mut end_mark: yaml_mark_t;
    let mut string = NULL_STRING;
    let mut leading_break = NULL_STRING;
    let mut trailing_breaks = NULL_STRING;
    let mut whitespaces = NULL_STRING;
    let mut leading_blanks: libc::c_int = 0;
    let indent: libc::c_int = (*parser).indent + 1;
    if init_string(parser, addr_of_mut!(string)).fail {
        return FAIL;
    }
    if init_string(parser, addr_of_mut!(leading_break)).fail {
        STRING_DEL!(string);
        return FAIL;
    }
    if init_string(parser, addr_of_mut!(trailing_breaks)).fail {
        STRING_DEL!(string);
        STRING_DEL!(leading_break);
        return FAIL;
    }
    if init_string(parser, addr_of_mut!(whitespaces)).fail {
        STRING_DEL!(string);
        STRING_DEL!(leading_break);
        STRING_DEL!(trailing_breaks);
        return FAIL;
    }
    end_mark = (*parser).mark;
    let start_mark: yaml_mark_t = end_mark;
    's_57: loop {
        if CACHE(parser, 4_usize).fail {
            current_block = 16642808987012640029;
            break;
        }
        if (*parser).mark.column == 0_usize
            && (CHECK_AT!((*parser).buffer, b'-', 0)
                && CHECK_AT!((*parser).buffer, b'-', 1)
                && CHECK_AT!((*parser).buffer, b'-', 2)
                || CHECK_AT!((*parser).buffer, b'.', 0)
                    && CHECK_AT!((*parser).buffer, b'.', 1)
                    && CHECK_AT!((*parser).buffer, b'.', 2))
            && IS_BLANKZ_AT!((*parser).buffer, 3)
        {
            current_block = 6281126495347172768;
            break;
        }
        if CHECK!((*parser).buffer, b'#') {
            current_block = 6281126495347172768;
            break;
        }
        while !IS_BLANKZ!((*parser).buffer) {
            if (*parser).flow_level != 0
                && CHECK!((*parser).buffer, b':')
                && (CHECK_AT!((*parser).buffer, b',', 1)
                    || CHECK_AT!((*parser).buffer, b'?', 1)
                    || CHECK_AT!((*parser).buffer, b'[', 1)
                    || CHECK_AT!((*parser).buffer, b']', 1)
                    || CHECK_AT!((*parser).buffer, b'{', 1)
                    || CHECK_AT!((*parser).buffer, b'}', 1))
            {
                yaml_parser_set_scanner_error(
                    parser,
                    b"while scanning a plain scalar\0" as *const u8 as *const libc::c_char,
                    start_mark,
                    b"found unexpected ':'\0" as *const u8 as *const libc::c_char,
                );
                current_block = 16642808987012640029;
                break 's_57;
            } else {
                if CHECK!((*parser).buffer, b':') && IS_BLANKZ_AT!((*parser).buffer, 1)
                    || (*parser).flow_level != 0
                        && (CHECK!((*parser).buffer, b',')
                            || CHECK!((*parser).buffer, b'[')
                            || CHECK!((*parser).buffer, b']')
                            || CHECK!((*parser).buffer, b'{')
                            || CHECK!((*parser).buffer, b'}'))
                {
                    break;
                }
                if leading_blanks != 0 || whitespaces.start != whitespaces.pointer {
                    if leading_blanks != 0 {
                        if *leading_break.start == b'\n' {
                            if *trailing_breaks.start == b'\0' {
                                if extend_string(parser, addr_of_mut!(string)).fail {
                                    current_block = 16642808987012640029;
                                    break 's_57;
                                }
                                let fresh717 = string.pointer;
                                string.pointer = string.pointer.wrapping_offset(1);
                                *fresh717 = b' ';
                            } else {
                                if join_strings(
                                    parser,
                                    addr_of_mut!(string),
                                    addr_of_mut!(trailing_breaks),
                                )
                                .fail
                                {
                                    current_block = 16642808987012640029;
                                    break 's_57;
                                }
                                CLEAR!(trailing_breaks);
                            }
                            CLEAR!(leading_break);
                        } else {
                            if join_strings(
                                parser,
                                addr_of_mut!(string),
                                addr_of_mut!(leading_break),
                            )
                            .fail
                            {
                                current_block = 16642808987012640029;
                                break 's_57;
                            }
                            if join_strings(
                                parser,
                                addr_of_mut!(string),
                                addr_of_mut!(trailing_breaks),
                            )
                            .fail
                            {
                                current_block = 16642808987012640029;
                                break 's_57;
                            }
                            CLEAR!(leading_break);
                            CLEAR!(trailing_breaks);
                        }
                        leading_blanks = 0;
                    } else {
                        if join_strings(parser, addr_of_mut!(string), addr_of_mut!(whitespaces))
                            .fail
                        {
                            current_block = 16642808987012640029;
                            break 's_57;
                        }
                        CLEAR!(whitespaces);
                    }
                }
                if READ!(parser, string).fail {
                    current_block = 16642808987012640029;
                    break 's_57;
                }
                end_mark = (*parser).mark;
                if CACHE(parser, 2_usize).fail {
                    current_block = 16642808987012640029;
                    break 's_57;
                }
            }
        }
        if !(IS_BLANK!((*parser).buffer) || IS_BREAK!((*parser).buffer)) {
            current_block = 6281126495347172768;
            break;
        }
        if CACHE(parser, 1_usize).fail {
            current_block = 16642808987012640029;
            break;
        }
        while IS_BLANK!((*parser).buffer) || IS_BREAK!((*parser).buffer) {
            if IS_BLANK!((*parser).buffer) {
                if leading_blanks != 0
                    && ((*parser).mark.column as libc::c_int) < indent
                    && IS_TAB!((*parser).buffer)
                {
                    yaml_parser_set_scanner_error(
                        parser,
                        b"while scanning a plain scalar\0" as *const u8 as *const libc::c_char,
                        start_mark,
                        b"found a tab character that violates indentation\0" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 16642808987012640029;
                    break 's_57;
                } else if leading_blanks == 0 {
                    if READ!(parser, whitespaces).fail {
                        current_block = 16642808987012640029;
                        break 's_57;
                    }
                } else {
                    SKIP(parser);
                }
            } else {
                if CACHE(parser, 2_usize).fail {
                    current_block = 16642808987012640029;
                    break 's_57;
                }
                if leading_blanks == 0 {
                    CLEAR!(whitespaces);
                    if READ_LINE!(parser, leading_break).fail {
                        current_block = 16642808987012640029;
                        break 's_57;
                    }
                    leading_blanks = 1;
                } else {
                    if READ_LINE!(parser, trailing_breaks).fail {
                        current_block = 16642808987012640029;
                        break 's_57;
                    }
                }
            }
            if CACHE(parser, 1_usize).fail {
                current_block = 16642808987012640029;
                break 's_57;
            }
        }
        if (*parser).flow_level == 0 && ((*parser).mark.column as libc::c_int) < indent {
            current_block = 6281126495347172768;
            break;
        }
    }
    if current_block != 16642808987012640029 {
        memset(
            token as *mut libc::c_void,
            0,
            size_of::<yaml_token_t>() as libc::c_ulong,
        );
        (*token).r#type = YAML_SCALAR_TOKEN;
        (*token).start_mark = start_mark;
        (*token).end_mark = end_mark;
        let fresh842 = addr_of_mut!((*token).data.scalar.value);
        *fresh842 = string.start;
        (*token).data.scalar.length =
            string.pointer.c_offset_from(string.start) as libc::c_long as size_t;
        (*token).data.scalar.style = YAML_PLAIN_SCALAR_STYLE;
        if leading_blanks != 0 {
            (*parser).simple_key_allowed = 1;
        }
        STRING_DEL!(leading_break);
        STRING_DEL!(trailing_breaks);
        STRING_DEL!(whitespaces);
        return OK;
    }
    STRING_DEL!(string);
    STRING_DEL!(leading_break);
    STRING_DEL!(trailing_breaks);
    STRING_DEL!(whitespaces);
    FAIL
}
