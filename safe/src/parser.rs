use core::ffi::c_int;
use core::mem::size_of;
use core::ptr;

use crate::alloc;
use crate::event::{
    initialize_alias_event, initialize_document_end_event, initialize_document_start_event,
    initialize_mapping_end_event, initialize_mapping_start_event, initialize_scalar_event,
    initialize_sequence_end_event, initialize_sequence_start_event, initialize_stream_end_event,
    initialize_stream_start_event, zero_event,
};
use crate::externs::{strcmp, strlen};
use crate::ffi;
use crate::scanner::yaml_parser_fetch_more_tokens_impl;
use crate::types::{
    yaml_char_t, yaml_error_type_t, yaml_event_t, yaml_mapping_style_t, yaml_mark_t,
    yaml_parser_state_t, yaml_parser_t, yaml_sequence_style_t, yaml_tag_directive_t, yaml_token_t,
    yaml_token_type_t, yaml_version_directive_t,
};
use crate::{yaml_free, yaml_malloc, yaml_strdup, FAIL, OK};

const DID_NOT_FIND_EXPECTED_STREAM_START: &[u8] = b"did not find expected <stream-start>\0";
const DID_NOT_FIND_EXPECTED_DOCUMENT_START: &[u8] = b"did not find expected <document start>\0";
const WHILE_PARSING_A_NODE: &[u8] = b"while parsing a node\0";
const FOUND_UNDEFINED_TAG_HANDLE: &[u8] = b"found undefined tag handle\0";
const WHILE_PARSING_A_BLOCK_NODE: &[u8] = b"while parsing a block node\0";
const WHILE_PARSING_A_FLOW_NODE: &[u8] = b"while parsing a flow node\0";
const DID_NOT_FIND_EXPECTED_NODE_CONTENT: &[u8] = b"did not find expected node content\0";
const WHILE_PARSING_A_BLOCK_COLLECTION: &[u8] = b"while parsing a block collection\0";
const DID_NOT_FIND_EXPECTED_DASH: &[u8] = b"did not find expected '-' indicator\0";
const WHILE_PARSING_A_BLOCK_MAPPING: &[u8] = b"while parsing a block mapping\0";
const DID_NOT_FIND_EXPECTED_KEY: &[u8] = b"did not find expected key\0";
const WHILE_PARSING_A_FLOW_SEQUENCE: &[u8] = b"while parsing a flow sequence\0";
const DID_NOT_FIND_EXPECTED_COMMA_OR_RBRACKET: &[u8] = b"did not find expected ',' or ']'\0";
const WHILE_PARSING_A_FLOW_MAPPING: &[u8] = b"while parsing a flow mapping\0";
const DID_NOT_FIND_EXPECTED_COMMA_OR_RBRACE: &[u8] = b"did not find expected ',' or '}'\0";
const FOUND_DUPLICATE_YAML_DIRECTIVE: &[u8] = b"found duplicate %YAML directive\0";
const FOUND_INCOMPATIBLE_YAML_DOCUMENT: &[u8] = b"found incompatible YAML document\0";
const FOUND_DUPLICATE_TAG_DIRECTIVE: &[u8] = b"found duplicate %TAG directive\0";
const BANG_TAG: &[u8] = b"!\0";
const DEFAULT_TAG_HANDLE_BANG: &[u8] = b"!\0";
const DEFAULT_TAG_PREFIX_BANG: &[u8] = b"!\0";
const DEFAULT_TAG_HANDLE_YAML: &[u8] = b"!!\0";
const DEFAULT_TAG_PREFIX_YAML: &[u8] = b"tag:yaml.org,2002:\0";

#[inline]
unsafe fn peek_token(parser: *mut yaml_parser_t) -> *mut yaml_token_t {
    if (*parser).token_available != 0 || yaml_parser_fetch_more_tokens_impl(parser).ok {
        if (*parser).tokens.head == (*parser).tokens.tail {
            ptr::null_mut()
        } else {
            (*parser).tokens.head
        }
    } else {
        ptr::null_mut()
    }
}

#[inline]
unsafe fn skip_token(parser: *mut yaml_parser_t) {
    (*parser).token_available = 0;
    (*parser).tokens_parsed = (*parser).tokens_parsed.wrapping_add(1);
    (*parser).stream_end_produced =
        ((*(*parser).tokens.head).r#type == yaml_token_type_t::YAML_STREAM_END_TOKEN) as c_int;
    (*parser).tokens.head = (*parser).tokens.head.add(1);
}

#[inline]
unsafe fn set_parser_error(
    parser: *mut yaml_parser_t,
    problem: &'static [u8],
    problem_mark: yaml_mark_t,
) -> c_int {
    (*parser).error = yaml_error_type_t::YAML_PARSER_ERROR;
    (*parser).problem = problem.as_ptr().cast();
    (*parser).problem_mark = problem_mark;
    FAIL
}

#[inline]
unsafe fn set_parser_error_context(
    parser: *mut yaml_parser_t,
    context: &'static [u8],
    context_mark: yaml_mark_t,
    problem: &'static [u8],
    problem_mark: yaml_mark_t,
) -> c_int {
    (*parser).error = yaml_error_type_t::YAML_PARSER_ERROR;
    (*parser).context = context.as_ptr().cast();
    (*parser).context_mark = context_mark;
    (*parser).problem = problem.as_ptr().cast();
    (*parser).problem_mark = problem_mark;
    FAIL
}

unsafe fn yaml_parser_state_machine(parser: *mut yaml_parser_t, event: *mut yaml_event_t) -> c_int {
    match (*parser).state {
        yaml_parser_state_t::YAML_PARSE_STREAM_START_STATE => {
            yaml_parser_parse_stream_start(parser, event)
        }
        yaml_parser_state_t::YAML_PARSE_IMPLICIT_DOCUMENT_START_STATE => {
            yaml_parser_parse_document_start(parser, event, 1)
        }
        yaml_parser_state_t::YAML_PARSE_DOCUMENT_START_STATE => {
            yaml_parser_parse_document_start(parser, event, 0)
        }
        yaml_parser_state_t::YAML_PARSE_DOCUMENT_CONTENT_STATE => {
            yaml_parser_parse_document_content(parser, event)
        }
        yaml_parser_state_t::YAML_PARSE_DOCUMENT_END_STATE => {
            yaml_parser_parse_document_end(parser, event)
        }
        yaml_parser_state_t::YAML_PARSE_BLOCK_NODE_STATE => {
            yaml_parser_parse_node(parser, event, 1, 0)
        }
        yaml_parser_state_t::YAML_PARSE_BLOCK_NODE_OR_INDENTLESS_SEQUENCE_STATE => {
            yaml_parser_parse_node(parser, event, 1, 1)
        }
        yaml_parser_state_t::YAML_PARSE_FLOW_NODE_STATE => {
            yaml_parser_parse_node(parser, event, 0, 0)
        }
        yaml_parser_state_t::YAML_PARSE_BLOCK_SEQUENCE_FIRST_ENTRY_STATE => {
            yaml_parser_parse_block_sequence_entry(parser, event, 1)
        }
        yaml_parser_state_t::YAML_PARSE_BLOCK_SEQUENCE_ENTRY_STATE => {
            yaml_parser_parse_block_sequence_entry(parser, event, 0)
        }
        yaml_parser_state_t::YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE => {
            yaml_parser_parse_indentless_sequence_entry(parser, event)
        }
        yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_FIRST_KEY_STATE => {
            yaml_parser_parse_block_mapping_key(parser, event, 1)
        }
        yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_KEY_STATE => {
            yaml_parser_parse_block_mapping_key(parser, event, 0)
        }
        yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_VALUE_STATE => {
            yaml_parser_parse_block_mapping_value(parser, event)
        }
        yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_FIRST_ENTRY_STATE => {
            yaml_parser_parse_flow_sequence_entry(parser, event, 1)
        }
        yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_STATE => {
            yaml_parser_parse_flow_sequence_entry(parser, event, 0)
        }
        yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_KEY_STATE => {
            yaml_parser_parse_flow_sequence_entry_mapping_key(parser, event)
        }
        yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE => {
            yaml_parser_parse_flow_sequence_entry_mapping_value(parser, event)
        }
        yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_END_STATE => {
            yaml_parser_parse_flow_sequence_entry_mapping_end(parser, event)
        }
        yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_FIRST_KEY_STATE => {
            yaml_parser_parse_flow_mapping_key(parser, event, 1)
        }
        yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_KEY_STATE => {
            yaml_parser_parse_flow_mapping_key(parser, event, 0)
        }
        yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_VALUE_STATE => {
            yaml_parser_parse_flow_mapping_value(parser, event, 0)
        }
        yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_EMPTY_VALUE_STATE => {
            yaml_parser_parse_flow_mapping_value(parser, event, 1)
        }
        yaml_parser_state_t::YAML_PARSE_END_STATE => FAIL,
    }
}

unsafe fn yaml_parser_parse_stream_start(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type != yaml_token_type_t::YAML_STREAM_START_TOKEN {
        return set_parser_error(
            parser,
            DID_NOT_FIND_EXPECTED_STREAM_START,
            (*token).start_mark,
        );
    }

    (*parser).state = yaml_parser_state_t::YAML_PARSE_IMPLICIT_DOCUMENT_START_STATE;
    initialize_stream_start_event(
        event,
        (*token).data.stream_start.encoding,
        (*token).start_mark,
        (*token).start_mark,
    );
    skip_token(parser);
    OK
}

unsafe fn yaml_parser_parse_document_start(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    implicit: c_int,
) -> c_int {
    let mut token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    let mut version_directive = ptr::null_mut::<yaml_version_directive_t>();
    let mut tag_directives_start = ptr::null_mut::<yaml_tag_directive_t>();
    let mut tag_directives_end = ptr::null_mut::<yaml_tag_directive_t>();

    if implicit == 0 {
        while !token.is_null() && (*token).r#type == yaml_token_type_t::YAML_DOCUMENT_END_TOKEN {
            skip_token(parser);
            token = peek_token(parser);
            if token.is_null() {
                return FAIL;
            }
        }
    }

    if implicit != 0
        && (*token).r#type != yaml_token_type_t::YAML_VERSION_DIRECTIVE_TOKEN
        && (*token).r#type != yaml_token_type_t::YAML_TAG_DIRECTIVE_TOKEN
        && (*token).r#type != yaml_token_type_t::YAML_DOCUMENT_START_TOKEN
        && (*token).r#type != yaml_token_type_t::YAML_STREAM_END_TOKEN
    {
        if yaml_parser_process_directives(parser, ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
            == FAIL
        {
            return FAIL;
        }
        if PUSH!(
            *parser,
            (*parser).states,
            yaml_parser_state_t::YAML_PARSE_DOCUMENT_END_STATE
        ) == FAIL
        {
            return FAIL;
        }
        (*parser).state = yaml_parser_state_t::YAML_PARSE_BLOCK_NODE_STATE;
        initialize_document_start_event(
            event,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            1,
            (*token).start_mark,
            (*token).start_mark,
        );
        return OK;
    }

    if (*token).r#type != yaml_token_type_t::YAML_STREAM_END_TOKEN {
        let start_mark = (*token).start_mark;
        if yaml_parser_process_directives(
            parser,
            &mut version_directive,
            &mut tag_directives_start,
            &mut tag_directives_end,
        ) == FAIL
        {
            return FAIL;
        }
        token = peek_token(parser);
        if token.is_null() {
            yaml_parser_document_start_cleanup(
                version_directive,
                tag_directives_start,
                tag_directives_end,
            );
            return FAIL;
        }
        if (*token).r#type != yaml_token_type_t::YAML_DOCUMENT_START_TOKEN {
            let result = set_parser_error(
                parser,
                DID_NOT_FIND_EXPECTED_DOCUMENT_START,
                (*token).start_mark,
            );
            yaml_parser_document_start_cleanup(
                version_directive,
                tag_directives_start,
                tag_directives_end,
            );
            return result;
        }
        if PUSH!(
            *parser,
            (*parser).states,
            yaml_parser_state_t::YAML_PARSE_DOCUMENT_END_STATE
        ) == FAIL
        {
            yaml_parser_document_start_cleanup(
                version_directive,
                tag_directives_start,
                tag_directives_end,
            );
            return FAIL;
        }
        (*parser).state = yaml_parser_state_t::YAML_PARSE_DOCUMENT_CONTENT_STATE;
        initialize_document_start_event(
            event,
            version_directive,
            tag_directives_start,
            tag_directives_end,
            0,
            start_mark,
            (*token).end_mark,
        );
        skip_token(parser);
        return OK;
    }

    (*parser).state = yaml_parser_state_t::YAML_PARSE_END_STATE;
    initialize_stream_end_event(event, (*token).start_mark, (*token).end_mark);
    skip_token(parser);
    OK
}

#[inline]
unsafe fn yaml_parser_document_start_cleanup(
    version_directive: *mut yaml_version_directive_t,
    tag_directives_start: *mut yaml_tag_directive_t,
    mut tag_directives_end: *mut yaml_tag_directive_t,
) {
    yaml_free(version_directive.cast());
    while tag_directives_start != tag_directives_end {
        tag_directives_end = tag_directives_end.sub(1);
        yaml_free((*tag_directives_end).handle.cast());
        yaml_free((*tag_directives_end).prefix.cast());
    }
    yaml_free(tag_directives_start.cast());
}

unsafe fn yaml_parser_parse_document_content(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type == yaml_token_type_t::YAML_VERSION_DIRECTIVE_TOKEN
        || (*token).r#type == yaml_token_type_t::YAML_TAG_DIRECTIVE_TOKEN
        || (*token).r#type == yaml_token_type_t::YAML_DOCUMENT_START_TOKEN
        || (*token).r#type == yaml_token_type_t::YAML_DOCUMENT_END_TOKEN
        || (*token).r#type == yaml_token_type_t::YAML_STREAM_END_TOKEN
    {
        (*parser).state = POP!((*parser).states);
        yaml_parser_process_empty_scalar(parser, event, (*token).start_mark)
    } else {
        yaml_parser_parse_node(parser, event, 1, 0)
    }
}

unsafe fn yaml_parser_parse_document_end(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    let start_mark = (*token).start_mark;
    let mut end_mark = (*token).start_mark;
    let mut implicit = 1;

    if (*token).r#type == yaml_token_type_t::YAML_DOCUMENT_END_TOKEN {
        end_mark = (*token).end_mark;
        skip_token(parser);
        implicit = 0;
    }

    while !STACK_EMPTY!((*parser).tag_directives) {
        let tag_directive = POP!((*parser).tag_directives);
        yaml_free(tag_directive.handle.cast());
        yaml_free(tag_directive.prefix.cast());
    }

    (*parser).state = yaml_parser_state_t::YAML_PARSE_DOCUMENT_START_STATE;
    initialize_document_end_event(event, implicit, start_mark, end_mark);
    OK
}

unsafe fn yaml_parser_parse_node(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    block: c_int,
    indentless_sequence: c_int,
) -> c_int {
    let mut token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type == yaml_token_type_t::YAML_ALIAS_TOKEN {
        (*parser).state = POP!((*parser).states);
        initialize_alias_event(
            event,
            (*token).data.alias.value,
            (*token).start_mark,
            (*token).end_mark,
        );
        skip_token(parser);
        return OK;
    }

    let mut anchor = ptr::null_mut::<yaml_char_t>();
    let mut tag_handle = ptr::null_mut::<yaml_char_t>();
    let mut tag_suffix = ptr::null_mut::<yaml_char_t>();
    let mut tag = ptr::null_mut::<yaml_char_t>();
    let mut start_mark = (*token).start_mark;
    let mut end_mark = (*token).start_mark;
    let mut tag_mark = (*token).start_mark;

    if (*token).r#type == yaml_token_type_t::YAML_ANCHOR_TOKEN {
        anchor = (*token).data.anchor.value;
        start_mark = (*token).start_mark;
        end_mark = (*token).end_mark;
        skip_token(parser);
        token = peek_token(parser);
        if token.is_null() {
            return yaml_parser_parse_node_error_cleanup(anchor, tag_handle, tag_suffix, tag);
        }
        if (*token).r#type == yaml_token_type_t::YAML_TAG_TOKEN {
            tag_handle = (*token).data.tag.handle;
            tag_suffix = (*token).data.tag.suffix;
            tag_mark = (*token).start_mark;
            end_mark = (*token).end_mark;
            skip_token(parser);
            token = peek_token(parser);
            if token.is_null() {
                return yaml_parser_parse_node_error_cleanup(anchor, tag_handle, tag_suffix, tag);
            }
        }
    } else if (*token).r#type == yaml_token_type_t::YAML_TAG_TOKEN {
        tag_handle = (*token).data.tag.handle;
        tag_suffix = (*token).data.tag.suffix;
        start_mark = (*token).start_mark;
        tag_mark = (*token).start_mark;
        end_mark = (*token).end_mark;
        skip_token(parser);
        token = peek_token(parser);
        if token.is_null() {
            return yaml_parser_parse_node_error_cleanup(anchor, tag_handle, tag_suffix, tag);
        }
        if (*token).r#type == yaml_token_type_t::YAML_ANCHOR_TOKEN {
            anchor = (*token).data.anchor.value;
            end_mark = (*token).end_mark;
            skip_token(parser);
            token = peek_token(parser);
            if token.is_null() {
                return yaml_parser_parse_node_error_cleanup(anchor, tag_handle, tag_suffix, tag);
            }
        }
    }

    if !tag_handle.is_null() {
        if *tag_handle == b'\0' {
            tag = tag_suffix;
            yaml_free(tag_handle.cast());
            tag_handle = ptr::null_mut();
            tag_suffix = ptr::null_mut();
        } else {
            let mut tag_directive = (*parser).tag_directives.start;
            while tag_directive != (*parser).tag_directives.top {
                if strcmp((*tag_directive).handle.cast(), tag_handle.cast()) == 0 {
                    let prefix_len = strlen((*tag_directive).prefix.cast());
                    let suffix_len = strlen(tag_suffix.cast());
                    tag = yaml_malloc(prefix_len + suffix_len + 1).cast::<yaml_char_t>();
                    if tag.is_null() {
                        (*parser).error = yaml_error_type_t::YAML_MEMORY_ERROR;
                        return yaml_parser_parse_node_error_cleanup(
                            anchor, tag_handle, tag_suffix, tag,
                        );
                    }
                    alloc::copy_bytes(tag.cast(), (*tag_directive).prefix.cast(), prefix_len);
                    alloc::copy_bytes(tag.add(prefix_len).cast(), tag_suffix.cast(), suffix_len);
                    *tag.add(prefix_len + suffix_len) = b'\0';
                    yaml_free(tag_handle.cast());
                    yaml_free(tag_suffix.cast());
                    tag_handle = ptr::null_mut();
                    tag_suffix = ptr::null_mut();
                    break;
                }
                tag_directive = tag_directive.add(1);
            }
            if tag.is_null() {
                let result = set_parser_error_context(
                    parser,
                    WHILE_PARSING_A_NODE,
                    start_mark,
                    FOUND_UNDEFINED_TAG_HANDLE,
                    tag_mark,
                );
                yaml_parser_parse_node_error_cleanup(anchor, tag_handle, tag_suffix, tag);
                return result;
            }
        }
    }

    let implicit = (tag.is_null() || *tag == b'\0') as c_int;
    if indentless_sequence != 0 && (*token).r#type == yaml_token_type_t::YAML_BLOCK_ENTRY_TOKEN {
        end_mark = (*token).end_mark;
        (*parser).state = yaml_parser_state_t::YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE;
        initialize_sequence_start_event(
            event,
            anchor,
            tag,
            implicit,
            yaml_sequence_style_t::YAML_BLOCK_SEQUENCE_STYLE,
            start_mark,
            end_mark,
        );
        return OK;
    }

    if (*token).r#type == yaml_token_type_t::YAML_SCALAR_TOKEN {
        let mut plain_implicit = 0;
        let mut quoted_implicit = 0;
        end_mark = (*token).end_mark;
        if ((*token).data.scalar.style
            == crate::types::yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE
            && tag.is_null())
            || (!tag.is_null() && strcmp(tag.cast(), BANG_TAG.as_ptr().cast()) == 0)
        {
            plain_implicit = 1;
        } else if tag.is_null() {
            quoted_implicit = 1;
        }
        (*parser).state = POP!((*parser).states);
        initialize_scalar_event(
            event,
            anchor,
            tag,
            (*token).data.scalar.value,
            (*token).data.scalar.length,
            plain_implicit,
            quoted_implicit,
            (*token).data.scalar.style,
            start_mark,
            end_mark,
        );
        skip_token(parser);
        return OK;
    }

    if (*token).r#type == yaml_token_type_t::YAML_FLOW_SEQUENCE_START_TOKEN {
        end_mark = (*token).end_mark;
        (*parser).state = yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_FIRST_ENTRY_STATE;
        initialize_sequence_start_event(
            event,
            anchor,
            tag,
            implicit,
            crate::types::yaml_sequence_style_t::YAML_FLOW_SEQUENCE_STYLE,
            start_mark,
            end_mark,
        );
        return OK;
    }

    if (*token).r#type == yaml_token_type_t::YAML_FLOW_MAPPING_START_TOKEN {
        end_mark = (*token).end_mark;
        (*parser).state = yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_FIRST_KEY_STATE;
        initialize_mapping_start_event(
            event,
            anchor,
            tag,
            implicit,
            yaml_mapping_style_t::YAML_FLOW_MAPPING_STYLE,
            start_mark,
            end_mark,
        );
        return OK;
    }

    if block != 0 && (*token).r#type == yaml_token_type_t::YAML_BLOCK_SEQUENCE_START_TOKEN {
        end_mark = (*token).end_mark;
        (*parser).state = yaml_parser_state_t::YAML_PARSE_BLOCK_SEQUENCE_FIRST_ENTRY_STATE;
        initialize_sequence_start_event(
            event,
            anchor,
            tag,
            implicit,
            crate::types::yaml_sequence_style_t::YAML_BLOCK_SEQUENCE_STYLE,
            start_mark,
            end_mark,
        );
        return OK;
    }

    if block != 0 && (*token).r#type == yaml_token_type_t::YAML_BLOCK_MAPPING_START_TOKEN {
        end_mark = (*token).end_mark;
        (*parser).state = yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_FIRST_KEY_STATE;
        initialize_mapping_start_event(
            event,
            anchor,
            tag,
            implicit,
            yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE,
            start_mark,
            end_mark,
        );
        return OK;
    }

    if !anchor.is_null() || !tag.is_null() {
        let value = yaml_malloc(1).cast::<yaml_char_t>();
        if value.is_null() {
            (*parser).error = yaml_error_type_t::YAML_MEMORY_ERROR;
            return yaml_parser_parse_node_error_cleanup(anchor, tag_handle, tag_suffix, tag);
        }
        *value = b'\0';
        (*parser).state = POP!((*parser).states);
        initialize_scalar_event(
            event,
            anchor,
            tag,
            value,
            0,
            implicit,
            0,
            crate::types::yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
            start_mark,
            end_mark,
        );
        return OK;
    }

    let context = if block != 0 {
        WHILE_PARSING_A_BLOCK_NODE
    } else {
        WHILE_PARSING_A_FLOW_NODE
    };
    let result = set_parser_error_context(
        parser,
        context,
        start_mark,
        DID_NOT_FIND_EXPECTED_NODE_CONTENT,
        (*token).start_mark,
    );
    yaml_parser_parse_node_error_cleanup(anchor, tag_handle, tag_suffix, tag);
    result
}

#[inline]
unsafe fn yaml_parser_parse_node_error_cleanup(
    anchor: *mut yaml_char_t,
    tag_handle: *mut yaml_char_t,
    tag_suffix: *mut yaml_char_t,
    tag: *mut yaml_char_t,
) -> c_int {
    yaml_free(anchor.cast());
    yaml_free(tag_handle.cast());
    yaml_free(tag_suffix.cast());
    yaml_free(tag.cast());
    FAIL
}

unsafe fn yaml_parser_parse_block_sequence_entry(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    if first != 0 {
        let token = peek_token(parser);
        if token.is_null() {
            return FAIL;
        }
        if PUSH!(*parser, (*parser).marks, (*token).start_mark) == FAIL {
            return FAIL;
        }
        skip_token(parser);
    }

    let token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type == yaml_token_type_t::YAML_BLOCK_ENTRY_TOKEN {
        let mark = (*token).end_mark;
        skip_token(parser);
        let next = peek_token(parser);
        if next.is_null() {
            return FAIL;
        }
        if (*next).r#type != yaml_token_type_t::YAML_BLOCK_ENTRY_TOKEN
            && (*next).r#type != yaml_token_type_t::YAML_BLOCK_END_TOKEN
        {
            if PUSH!(
                *parser,
                (*parser).states,
                yaml_parser_state_t::YAML_PARSE_BLOCK_SEQUENCE_ENTRY_STATE
            ) == FAIL
            {
                return FAIL;
            }
            return yaml_parser_parse_node(parser, event, 1, 0);
        }
        (*parser).state = yaml_parser_state_t::YAML_PARSE_BLOCK_SEQUENCE_ENTRY_STATE;
        return yaml_parser_process_empty_scalar(parser, event, mark);
    }

    if (*token).r#type == yaml_token_type_t::YAML_BLOCK_END_TOKEN {
        (*parser).state = POP!((*parser).states);
        let _ = POP!((*parser).marks);
        initialize_sequence_end_event(event, (*token).start_mark, (*token).end_mark);
        skip_token(parser);
        return OK;
    }

    set_parser_error_context(
        parser,
        WHILE_PARSING_A_BLOCK_COLLECTION,
        POP!((*parser).marks),
        DID_NOT_FIND_EXPECTED_DASH,
        (*token).start_mark,
    )
}

unsafe fn yaml_parser_parse_indentless_sequence_entry(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type == yaml_token_type_t::YAML_BLOCK_ENTRY_TOKEN {
        let mark = (*token).end_mark;
        skip_token(parser);
        let next = peek_token(parser);
        if next.is_null() {
            return FAIL;
        }
        if (*next).r#type != yaml_token_type_t::YAML_BLOCK_ENTRY_TOKEN
            && (*next).r#type != yaml_token_type_t::YAML_KEY_TOKEN
            && (*next).r#type != yaml_token_type_t::YAML_VALUE_TOKEN
            && (*next).r#type != yaml_token_type_t::YAML_BLOCK_END_TOKEN
        {
            if PUSH!(
                *parser,
                (*parser).states,
                yaml_parser_state_t::YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE
            ) == FAIL
            {
                return FAIL;
            }
            return yaml_parser_parse_node(parser, event, 1, 0);
        }
        (*parser).state = yaml_parser_state_t::YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE;
        return yaml_parser_process_empty_scalar(parser, event, mark);
    }

    (*parser).state = POP!((*parser).states);
    initialize_sequence_end_event(event, (*token).start_mark, (*token).start_mark);
    OK
}

unsafe fn yaml_parser_parse_block_mapping_key(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    if first != 0 {
        let token = peek_token(parser);
        if token.is_null() {
            return FAIL;
        }
        if PUSH!(*parser, (*parser).marks, (*token).start_mark) == FAIL {
            return FAIL;
        }
        skip_token(parser);
    }

    let token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type == yaml_token_type_t::YAML_KEY_TOKEN {
        let mark = (*token).end_mark;
        skip_token(parser);
        let next = peek_token(parser);
        if next.is_null() {
            return FAIL;
        }
        if (*next).r#type != yaml_token_type_t::YAML_KEY_TOKEN
            && (*next).r#type != yaml_token_type_t::YAML_VALUE_TOKEN
            && (*next).r#type != yaml_token_type_t::YAML_BLOCK_END_TOKEN
        {
            if PUSH!(
                *parser,
                (*parser).states,
                yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_VALUE_STATE
            ) == FAIL
            {
                return FAIL;
            }
            return yaml_parser_parse_node(parser, event, 1, 1);
        }
        (*parser).state = yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_VALUE_STATE;
        return yaml_parser_process_empty_scalar(parser, event, mark);
    }

    if (*token).r#type == yaml_token_type_t::YAML_BLOCK_END_TOKEN {
        (*parser).state = POP!((*parser).states);
        let _ = POP!((*parser).marks);
        initialize_mapping_end_event(event, (*token).start_mark, (*token).end_mark);
        skip_token(parser);
        return OK;
    }

    set_parser_error_context(
        parser,
        WHILE_PARSING_A_BLOCK_MAPPING,
        POP!((*parser).marks),
        DID_NOT_FIND_EXPECTED_KEY,
        (*token).start_mark,
    )
}

unsafe fn yaml_parser_parse_block_mapping_value(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type == yaml_token_type_t::YAML_VALUE_TOKEN {
        let mark = (*token).end_mark;
        skip_token(parser);
        let next = peek_token(parser);
        if next.is_null() {
            return FAIL;
        }
        if (*next).r#type != yaml_token_type_t::YAML_KEY_TOKEN
            && (*next).r#type != yaml_token_type_t::YAML_VALUE_TOKEN
            && (*next).r#type != yaml_token_type_t::YAML_BLOCK_END_TOKEN
        {
            if PUSH!(
                *parser,
                (*parser).states,
                yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_KEY_STATE
            ) == FAIL
            {
                return FAIL;
            }
            return yaml_parser_parse_node(parser, event, 1, 1);
        }
        (*parser).state = yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_KEY_STATE;
        return yaml_parser_process_empty_scalar(parser, event, mark);
    }

    (*parser).state = yaml_parser_state_t::YAML_PARSE_BLOCK_MAPPING_KEY_STATE;
    yaml_parser_process_empty_scalar(parser, event, (*token).start_mark)
}

unsafe fn yaml_parser_parse_flow_sequence_entry(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    if first != 0 {
        let token = peek_token(parser);
        if token.is_null() {
            return FAIL;
        }
        if PUSH!(*parser, (*parser).marks, (*token).start_mark) == FAIL {
            return FAIL;
        }
        skip_token(parser);
    }

    let mut token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type != yaml_token_type_t::YAML_FLOW_SEQUENCE_END_TOKEN {
        if first == 0 {
            if (*token).r#type == yaml_token_type_t::YAML_FLOW_ENTRY_TOKEN {
                skip_token(parser);
                token = peek_token(parser);
                if token.is_null() {
                    return FAIL;
                }
            } else {
                return set_parser_error_context(
                    parser,
                    WHILE_PARSING_A_FLOW_SEQUENCE,
                    POP!((*parser).marks),
                    DID_NOT_FIND_EXPECTED_COMMA_OR_RBRACKET,
                    (*token).start_mark,
                );
            }
        }

        if (*token).r#type == yaml_token_type_t::YAML_KEY_TOKEN {
            (*parser).state = yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_KEY_STATE;
            initialize_mapping_start_event(
                event,
                ptr::null_mut(),
                ptr::null_mut(),
                1,
                yaml_mapping_style_t::YAML_FLOW_MAPPING_STYLE,
                (*token).start_mark,
                (*token).end_mark,
            );
            skip_token(parser);
            return OK;
        }

        if (*token).r#type != yaml_token_type_t::YAML_FLOW_SEQUENCE_END_TOKEN {
            if PUSH!(
                *parser,
                (*parser).states,
                yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_STATE
            ) == FAIL
            {
                return FAIL;
            }
            return yaml_parser_parse_node(parser, event, 0, 0);
        }
    }

    (*parser).state = POP!((*parser).states);
    let _ = POP!((*parser).marks);
    initialize_sequence_end_event(event, (*token).start_mark, (*token).end_mark);
    skip_token(parser);
    OK
}

unsafe fn yaml_parser_parse_flow_sequence_entry_mapping_key(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type != yaml_token_type_t::YAML_VALUE_TOKEN
        && (*token).r#type != yaml_token_type_t::YAML_FLOW_ENTRY_TOKEN
        && (*token).r#type != yaml_token_type_t::YAML_FLOW_SEQUENCE_END_TOKEN
    {
        if PUSH!(
            *parser,
            (*parser).states,
            yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE
        ) == FAIL
        {
            return FAIL;
        }
        return yaml_parser_parse_node(parser, event, 0, 0);
    }

    let mark = (*token).end_mark;
    skip_token(parser);
    (*parser).state = yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE;
    yaml_parser_process_empty_scalar(parser, event, mark)
}

unsafe fn yaml_parser_parse_flow_sequence_entry_mapping_value(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let mut token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type == yaml_token_type_t::YAML_VALUE_TOKEN {
        skip_token(parser);
        token = peek_token(parser);
        if token.is_null() {
            return FAIL;
        }
        if (*token).r#type != yaml_token_type_t::YAML_FLOW_ENTRY_TOKEN
            && (*token).r#type != yaml_token_type_t::YAML_FLOW_SEQUENCE_END_TOKEN
        {
            if PUSH!(
                *parser,
                (*parser).states,
                yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_END_STATE
            ) == FAIL
            {
                return FAIL;
            }
            return yaml_parser_parse_node(parser, event, 0, 0);
        }
    }

    (*parser).state = yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_END_STATE;
    yaml_parser_process_empty_scalar(parser, event, (*token).start_mark)
}

unsafe fn yaml_parser_parse_flow_sequence_entry_mapping_end(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    let token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    (*parser).state = yaml_parser_state_t::YAML_PARSE_FLOW_SEQUENCE_ENTRY_STATE;
    initialize_mapping_end_event(event, (*token).start_mark, (*token).start_mark);
    OK
}

unsafe fn yaml_parser_parse_flow_mapping_key(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    first: c_int,
) -> c_int {
    if first != 0 {
        let token = peek_token(parser);
        if token.is_null() {
            return FAIL;
        }
        if PUSH!(*parser, (*parser).marks, (*token).start_mark) == FAIL {
            return FAIL;
        }
        skip_token(parser);
    }

    let mut token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if (*token).r#type != yaml_token_type_t::YAML_FLOW_MAPPING_END_TOKEN {
        if first == 0 {
            if (*token).r#type == yaml_token_type_t::YAML_FLOW_ENTRY_TOKEN {
                skip_token(parser);
                token = peek_token(parser);
                if token.is_null() {
                    return FAIL;
                }
            } else {
                return set_parser_error_context(
                    parser,
                    WHILE_PARSING_A_FLOW_MAPPING,
                    POP!((*parser).marks),
                    DID_NOT_FIND_EXPECTED_COMMA_OR_RBRACE,
                    (*token).start_mark,
                );
            }
        }

        if (*token).r#type == yaml_token_type_t::YAML_KEY_TOKEN {
            skip_token(parser);
            token = peek_token(parser);
            if token.is_null() {
                return FAIL;
            }
            if (*token).r#type != yaml_token_type_t::YAML_VALUE_TOKEN
                && (*token).r#type != yaml_token_type_t::YAML_FLOW_ENTRY_TOKEN
                && (*token).r#type != yaml_token_type_t::YAML_FLOW_MAPPING_END_TOKEN
            {
                if PUSH!(
                    *parser,
                    (*parser).states,
                    yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_VALUE_STATE
                ) == FAIL
                {
                    return FAIL;
                }
                return yaml_parser_parse_node(parser, event, 0, 0);
            }
            (*parser).state = yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_VALUE_STATE;
            return yaml_parser_process_empty_scalar(parser, event, (*token).start_mark);
        }

        if (*token).r#type != yaml_token_type_t::YAML_FLOW_MAPPING_END_TOKEN {
            if PUSH!(
                *parser,
                (*parser).states,
                yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_EMPTY_VALUE_STATE
            ) == FAIL
            {
                return FAIL;
            }
            return yaml_parser_parse_node(parser, event, 0, 0);
        }
    }

    (*parser).state = POP!((*parser).states);
    let _ = POP!((*parser).marks);
    initialize_mapping_end_event(event, (*token).start_mark, (*token).end_mark);
    skip_token(parser);
    OK
}

unsafe fn yaml_parser_parse_flow_mapping_value(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    empty: c_int,
) -> c_int {
    let mut token = peek_token(parser);
    if token.is_null() {
        return FAIL;
    }

    if empty != 0 {
        (*parser).state = yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_KEY_STATE;
        return yaml_parser_process_empty_scalar(parser, event, (*token).start_mark);
    }

    if (*token).r#type == yaml_token_type_t::YAML_VALUE_TOKEN {
        skip_token(parser);
        token = peek_token(parser);
        if token.is_null() {
            return FAIL;
        }
        if (*token).r#type != yaml_token_type_t::YAML_FLOW_ENTRY_TOKEN
            && (*token).r#type != yaml_token_type_t::YAML_FLOW_MAPPING_END_TOKEN
        {
            if PUSH!(
                *parser,
                (*parser).states,
                yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_KEY_STATE
            ) == FAIL
            {
                return FAIL;
            }
            return yaml_parser_parse_node(parser, event, 0, 0);
        }
    }

    (*parser).state = yaml_parser_state_t::YAML_PARSE_FLOW_MAPPING_KEY_STATE;
    yaml_parser_process_empty_scalar(parser, event, (*token).start_mark)
}

unsafe fn yaml_parser_process_empty_scalar(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    mark: yaml_mark_t,
) -> c_int {
    let value = yaml_malloc(1).cast::<yaml_char_t>();
    if value.is_null() {
        (*parser).error = yaml_error_type_t::YAML_MEMORY_ERROR;
        return FAIL;
    }
    *value = b'\0';
    initialize_scalar_event(
        event,
        ptr::null_mut(),
        ptr::null_mut(),
        value,
        0,
        1,
        0,
        crate::types::yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
        mark,
        mark,
    );
    OK
}

unsafe fn yaml_parser_process_directives(
    parser: *mut yaml_parser_t,
    version_directive_ref: *mut *mut yaml_version_directive_t,
    tag_directives_start_ref: *mut *mut yaml_tag_directive_t,
    tag_directives_end_ref: *mut *mut yaml_tag_directive_t,
) -> c_int {
    let default_tag_directives = [
        yaml_tag_directive_t {
            handle: DEFAULT_TAG_HANDLE_BANG.as_ptr().cast_mut(),
            prefix: DEFAULT_TAG_PREFIX_BANG.as_ptr().cast_mut(),
        },
        yaml_tag_directive_t {
            handle: DEFAULT_TAG_HANDLE_YAML.as_ptr().cast_mut(),
            prefix: DEFAULT_TAG_PREFIX_YAML.as_ptr().cast_mut(),
        },
    ];

    let mut version_directive = ptr::null_mut::<yaml_version_directive_t>();
    let mut tag_directives = crate::types::yaml_parser_tag_directives_t {
        start: ptr::null_mut(),
        end: ptr::null_mut(),
        top: ptr::null_mut(),
    };

    if STACK_INIT!(tag_directives, yaml_tag_directive_t) == FAIL {
        return FAIL;
    }

    let mut token = peek_token(parser);
    if token.is_null() {
        STACK_DEL!(tag_directives);
        return FAIL;
    }

    while (*token).r#type == yaml_token_type_t::YAML_VERSION_DIRECTIVE_TOKEN
        || (*token).r#type == yaml_token_type_t::YAML_TAG_DIRECTIVE_TOKEN
    {
        if (*token).r#type == yaml_token_type_t::YAML_VERSION_DIRECTIVE_TOKEN {
            if !version_directive.is_null() {
                let result =
                    set_parser_error(parser, FOUND_DUPLICATE_YAML_DIRECTIVE, (*token).start_mark);
                yaml_parser_process_directives_error_cleanup(
                    version_directive,
                    &mut tag_directives,
                );
                return result;
            }
            if (*token).data.version_directive.major != 1
                || ((*token).data.version_directive.minor != 1
                    && (*token).data.version_directive.minor != 2)
            {
                let result = set_parser_error(
                    parser,
                    FOUND_INCOMPATIBLE_YAML_DOCUMENT,
                    (*token).start_mark,
                );
                yaml_parser_process_directives_error_cleanup(
                    version_directive,
                    &mut tag_directives,
                );
                return result;
            }
            version_directive = yaml_malloc(size_of::<yaml_version_directive_t>())
                .cast::<yaml_version_directive_t>();
            if version_directive.is_null() {
                (*parser).error = yaml_error_type_t::YAML_MEMORY_ERROR;
                yaml_parser_process_directives_error_cleanup(
                    version_directive,
                    &mut tag_directives,
                );
                return FAIL;
            }
            (*version_directive).major = (*token).data.version_directive.major;
            (*version_directive).minor = (*token).data.version_directive.minor;
        } else {
            let value = yaml_tag_directive_t {
                handle: (*token).data.tag_directive.handle,
                prefix: (*token).data.tag_directive.prefix,
            };

            if yaml_parser_append_tag_directive(parser, value, 0, (*token).start_mark) == FAIL {
                yaml_parser_process_directives_error_cleanup(
                    version_directive,
                    &mut tag_directives,
                );
                return FAIL;
            }
            if PUSH!(*parser, tag_directives, value) == FAIL {
                yaml_parser_process_directives_error_cleanup(
                    version_directive,
                    &mut tag_directives,
                );
                return FAIL;
            }
        }

        skip_token(parser);
        token = peek_token(parser);
        if token.is_null() {
            yaml_parser_process_directives_error_cleanup(version_directive, &mut tag_directives);
            return FAIL;
        }
    }

    for default_tag_directive in default_tag_directives {
        if yaml_parser_append_tag_directive(parser, default_tag_directive, 1, (*token).start_mark)
            == FAIL
        {
            yaml_parser_process_directives_error_cleanup(version_directive, &mut tag_directives);
            return FAIL;
        }
    }

    if !version_directive_ref.is_null() {
        *version_directive_ref = version_directive;
    }
    if !tag_directives_start_ref.is_null() {
        if STACK_EMPTY!(tag_directives) {
            *tag_directives_start_ref = ptr::null_mut();
            if !tag_directives_end_ref.is_null() {
                *tag_directives_end_ref = ptr::null_mut();
            }
            STACK_DEL!(tag_directives);
        } else {
            *tag_directives_start_ref = tag_directives.start;
            if !tag_directives_end_ref.is_null() {
                *tag_directives_end_ref = tag_directives.top;
            }
        }
    } else {
        STACK_DEL!(tag_directives);
    }

    if version_directive_ref.is_null() {
        yaml_free(version_directive.cast());
    }
    OK
}

#[inline]
unsafe fn yaml_parser_process_directives_error_cleanup(
    version_directive: *mut yaml_version_directive_t,
    tag_directives: &mut crate::types::yaml_parser_tag_directives_t,
) {
    yaml_free(version_directive.cast());
    while !STACK_EMPTY!(*tag_directives) {
        let tag_directive = POP!(*tag_directives);
        yaml_free(tag_directive.handle.cast());
        yaml_free(tag_directive.prefix.cast());
    }
    STACK_DEL!(*tag_directives);
}

unsafe fn yaml_parser_append_tag_directive(
    parser: *mut yaml_parser_t,
    value: yaml_tag_directive_t,
    allow_duplicates: c_int,
    mark: yaml_mark_t,
) -> c_int {
    let mut tag_directive = (*parser).tag_directives.start;
    while tag_directive != (*parser).tag_directives.top {
        if strcmp(value.handle.cast(), (*tag_directive).handle.cast()) == 0 {
            if allow_duplicates != 0 {
                return OK;
            }
            return set_parser_error(parser, FOUND_DUPLICATE_TAG_DIRECTIVE, mark);
        }
        tag_directive = tag_directive.add(1);
    }

    let mut copy = yaml_tag_directive_t {
        handle: ptr::null_mut(),
        prefix: ptr::null_mut(),
    };
    copy.handle = yaml_strdup(value.handle);
    copy.prefix = yaml_strdup(value.prefix);
    if copy.handle.is_null() || copy.prefix.is_null() {
        (*parser).error = yaml_error_type_t::YAML_MEMORY_ERROR;
        yaml_free(copy.handle.cast());
        yaml_free(copy.prefix.cast());
        return FAIL;
    }
    if PUSH!(*parser, (*parser).tag_directives, copy) == FAIL {
        yaml_free(copy.handle.cast());
        yaml_free(copy.prefix.cast());
        return FAIL;
    }
    OK
}

unsafe fn yaml_parser_parse_impl(parser: *mut yaml_parser_t, event: *mut yaml_event_t) -> c_int {
    zero_event(event);

    if (*parser).stream_end_produced != 0
        || (*parser).error != yaml_error_type_t::YAML_NO_ERROR
        || (*parser).state == yaml_parser_state_t::YAML_PARSE_END_STATE
    {
        return OK;
    }

    yaml_parser_state_machine(parser, event)
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_parse(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if parser.is_null() || event.is_null() {
            return FAIL;
        }
        yaml_parser_parse_impl(parser, event)
    })
}
