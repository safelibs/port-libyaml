#![allow(dead_code, non_camel_case_types, non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

pub type yaml_char_t = u8;
pub type yaml_file_t = c_void;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_version_directive_t {
    pub major: c_int,
    pub minor: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_tag_directive_t {
    pub handle: *mut yaml_char_t,
    pub prefix: *mut yaml_char_t,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_encoding_t {
    YAML_ANY_ENCODING = 0,
    YAML_UTF8_ENCODING = 1,
    YAML_UTF16LE_ENCODING = 2,
    YAML_UTF16BE_ENCODING = 3,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_break_t {
    YAML_ANY_BREAK = 0,
    YAML_CR_BREAK = 1,
    YAML_LN_BREAK = 2,
    YAML_CRLN_BREAK = 3,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_error_type_t {
    YAML_NO_ERROR = 0,
    YAML_MEMORY_ERROR = 1,
    YAML_READER_ERROR = 2,
    YAML_SCANNER_ERROR = 3,
    YAML_PARSER_ERROR = 4,
    YAML_COMPOSER_ERROR = 5,
    YAML_WRITER_ERROR = 6,
    YAML_EMITTER_ERROR = 7,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_mark_t {
    pub index: usize,
    pub line: usize,
    pub column: usize,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_scalar_style_t {
    YAML_ANY_SCALAR_STYLE = 0,
    YAML_PLAIN_SCALAR_STYLE = 1,
    YAML_SINGLE_QUOTED_SCALAR_STYLE = 2,
    YAML_DOUBLE_QUOTED_SCALAR_STYLE = 3,
    YAML_LITERAL_SCALAR_STYLE = 4,
    YAML_FOLDED_SCALAR_STYLE = 5,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_sequence_style_t {
    YAML_ANY_SEQUENCE_STYLE = 0,
    YAML_BLOCK_SEQUENCE_STYLE = 1,
    YAML_FLOW_SEQUENCE_STYLE = 2,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_mapping_style_t {
    YAML_ANY_MAPPING_STYLE = 0,
    YAML_BLOCK_MAPPING_STYLE = 1,
    YAML_FLOW_MAPPING_STYLE = 2,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_token_type_t {
    YAML_NO_TOKEN = 0,
    YAML_STREAM_START_TOKEN = 1,
    YAML_STREAM_END_TOKEN = 2,
    YAML_VERSION_DIRECTIVE_TOKEN = 3,
    YAML_TAG_DIRECTIVE_TOKEN = 4,
    YAML_DOCUMENT_START_TOKEN = 5,
    YAML_DOCUMENT_END_TOKEN = 6,
    YAML_BLOCK_SEQUENCE_START_TOKEN = 7,
    YAML_BLOCK_MAPPING_START_TOKEN = 8,
    YAML_BLOCK_END_TOKEN = 9,
    YAML_FLOW_SEQUENCE_START_TOKEN = 10,
    YAML_FLOW_SEQUENCE_END_TOKEN = 11,
    YAML_FLOW_MAPPING_START_TOKEN = 12,
    YAML_FLOW_MAPPING_END_TOKEN = 13,
    YAML_BLOCK_ENTRY_TOKEN = 14,
    YAML_FLOW_ENTRY_TOKEN = 15,
    YAML_KEY_TOKEN = 16,
    YAML_VALUE_TOKEN = 17,
    YAML_ALIAS_TOKEN = 18,
    YAML_ANCHOR_TOKEN = 19,
    YAML_TAG_TOKEN = 20,
    YAML_SCALAR_TOKEN = 21,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_stream_start_t {
    pub encoding: yaml_encoding_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_alias_t {
    pub value: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_anchor_t {
    pub value: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_tag_t {
    pub handle: *mut yaml_char_t,
    pub suffix: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_scalar_t {
    pub value: *mut yaml_char_t,
    pub length: usize,
    pub style: yaml_scalar_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_version_directive_t {
    pub major: c_int,
    pub minor: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_data_tag_directive_t {
    pub handle: *mut yaml_char_t,
    pub prefix: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union yaml_token_data_t {
    pub stream_start: yaml_token_data_stream_start_t,
    pub alias: yaml_token_data_alias_t,
    pub anchor: yaml_token_data_anchor_t,
    pub tag: yaml_token_data_tag_t,
    pub scalar: yaml_token_data_scalar_t,
    pub version_directive: yaml_token_data_version_directive_t,
    pub tag_directive: yaml_token_data_tag_directive_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_token_t {
    pub r#type: yaml_token_type_t,
    pub data: yaml_token_data_t,
    pub start_mark: yaml_mark_t,
    pub end_mark: yaml_mark_t,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_event_type_t {
    YAML_NO_EVENT = 0,
    YAML_STREAM_START_EVENT = 1,
    YAML_STREAM_END_EVENT = 2,
    YAML_DOCUMENT_START_EVENT = 3,
    YAML_DOCUMENT_END_EVENT = 4,
    YAML_ALIAS_EVENT = 5,
    YAML_SCALAR_EVENT = 6,
    YAML_SEQUENCE_START_EVENT = 7,
    YAML_SEQUENCE_END_EVENT = 8,
    YAML_MAPPING_START_EVENT = 9,
    YAML_MAPPING_END_EVENT = 10,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_stream_start_t {
    pub encoding: yaml_encoding_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_document_start_tag_directives_t {
    pub start: *mut yaml_tag_directive_t,
    pub end: *mut yaml_tag_directive_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_document_start_t {
    pub version_directive: *mut yaml_version_directive_t,
    pub tag_directives: yaml_event_data_document_start_tag_directives_t,
    pub implicit: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_document_end_t {
    pub implicit: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_alias_t {
    pub anchor: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_scalar_t {
    pub anchor: *mut yaml_char_t,
    pub tag: *mut yaml_char_t,
    pub value: *mut yaml_char_t,
    pub length: usize,
    pub plain_implicit: c_int,
    pub quoted_implicit: c_int,
    pub style: yaml_scalar_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_sequence_start_t {
    pub anchor: *mut yaml_char_t,
    pub tag: *mut yaml_char_t,
    pub implicit: c_int,
    pub style: yaml_sequence_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_data_mapping_start_t {
    pub anchor: *mut yaml_char_t,
    pub tag: *mut yaml_char_t,
    pub implicit: c_int,
    pub style: yaml_mapping_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union yaml_event_data_t {
    pub stream_start: yaml_event_data_stream_start_t,
    pub document_start: yaml_event_data_document_start_t,
    pub document_end: yaml_event_data_document_end_t,
    pub alias: yaml_event_data_alias_t,
    pub scalar: yaml_event_data_scalar_t,
    pub sequence_start: yaml_event_data_sequence_start_t,
    pub mapping_start: yaml_event_data_mapping_start_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_event_t {
    pub r#type: yaml_event_type_t,
    pub data: yaml_event_data_t,
    pub start_mark: yaml_mark_t,
    pub end_mark: yaml_mark_t,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_node_type_t {
    YAML_NO_NODE = 0,
    YAML_SCALAR_NODE = 1,
    YAML_SEQUENCE_NODE = 2,
    YAML_MAPPING_NODE = 3,
}

pub type yaml_node_item_t = c_int;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_pair_t {
    pub key: c_int,
    pub value: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_data_scalar_t {
    pub value: *mut yaml_char_t,
    pub length: usize,
    pub style: yaml_scalar_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_data_sequence_items_t {
    pub start: *mut yaml_node_item_t,
    pub end: *mut yaml_node_item_t,
    pub top: *mut yaml_node_item_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_data_sequence_t {
    pub items: yaml_node_data_sequence_items_t,
    pub style: yaml_sequence_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_data_mapping_pairs_t {
    pub start: *mut yaml_node_pair_t,
    pub end: *mut yaml_node_pair_t,
    pub top: *mut yaml_node_pair_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_data_mapping_t {
    pub pairs: yaml_node_data_mapping_pairs_t,
    pub style: yaml_mapping_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union yaml_node_data_t {
    pub scalar: yaml_node_data_scalar_t,
    pub sequence: yaml_node_data_sequence_t,
    pub mapping: yaml_node_data_mapping_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_node_t {
    pub r#type: yaml_node_type_t,
    pub tag: *mut yaml_char_t,
    pub data: yaml_node_data_t,
    pub start_mark: yaml_mark_t,
    pub end_mark: yaml_mark_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_document_nodes_t {
    pub start: *mut yaml_node_t,
    pub end: *mut yaml_node_t,
    pub top: *mut yaml_node_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_document_tag_directives_t {
    pub start: *mut yaml_tag_directive_t,
    pub end: *mut yaml_tag_directive_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_document_t {
    pub nodes: yaml_document_nodes_t,
    pub version_directive: *mut yaml_version_directive_t,
    pub tag_directives: yaml_document_tag_directives_t,
    pub start_implicit: c_int,
    pub end_implicit: c_int,
    pub start_mark: yaml_mark_t,
    pub end_mark: yaml_mark_t,
}

pub type yaml_read_handler_t = Option<
    unsafe extern "C" fn(
        data: *mut c_void,
        buffer: *mut u8,
        size: usize,
        size_read: *mut usize,
    ) -> c_int,
>;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_simple_key_t {
    pub possible: c_int,
    pub required: c_int,
    pub token_number: usize,
    pub mark: yaml_mark_t,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_parser_state_t {
    YAML_PARSE_STREAM_START_STATE = 0,
    YAML_PARSE_IMPLICIT_DOCUMENT_START_STATE = 1,
    YAML_PARSE_DOCUMENT_START_STATE = 2,
    YAML_PARSE_DOCUMENT_CONTENT_STATE = 3,
    YAML_PARSE_DOCUMENT_END_STATE = 4,
    YAML_PARSE_BLOCK_NODE_STATE = 5,
    YAML_PARSE_BLOCK_NODE_OR_INDENTLESS_SEQUENCE_STATE = 6,
    YAML_PARSE_FLOW_NODE_STATE = 7,
    YAML_PARSE_BLOCK_SEQUENCE_FIRST_ENTRY_STATE = 8,
    YAML_PARSE_BLOCK_SEQUENCE_ENTRY_STATE = 9,
    YAML_PARSE_INDENTLESS_SEQUENCE_ENTRY_STATE = 10,
    YAML_PARSE_BLOCK_MAPPING_FIRST_KEY_STATE = 11,
    YAML_PARSE_BLOCK_MAPPING_KEY_STATE = 12,
    YAML_PARSE_BLOCK_MAPPING_VALUE_STATE = 13,
    YAML_PARSE_FLOW_SEQUENCE_FIRST_ENTRY_STATE = 14,
    YAML_PARSE_FLOW_SEQUENCE_ENTRY_STATE = 15,
    YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_KEY_STATE = 16,
    YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_VALUE_STATE = 17,
    YAML_PARSE_FLOW_SEQUENCE_ENTRY_MAPPING_END_STATE = 18,
    YAML_PARSE_FLOW_MAPPING_FIRST_KEY_STATE = 19,
    YAML_PARSE_FLOW_MAPPING_KEY_STATE = 20,
    YAML_PARSE_FLOW_MAPPING_VALUE_STATE = 21,
    YAML_PARSE_FLOW_MAPPING_EMPTY_VALUE_STATE = 22,
    YAML_PARSE_END_STATE = 23,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_alias_data_t {
    pub anchor: *mut yaml_char_t,
    pub index: c_int,
    pub mark: yaml_mark_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_input_string_t {
    pub start: *const u8,
    pub end: *const u8,
    pub current: *const u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union yaml_parser_input_t {
    pub string: yaml_parser_input_string_t,
    pub file: *mut yaml_file_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_buffer_t {
    pub start: *mut yaml_char_t,
    pub end: *mut yaml_char_t,
    pub pointer: *mut yaml_char_t,
    pub last: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_raw_buffer_t {
    pub start: *mut u8,
    pub end: *mut u8,
    pub pointer: *mut u8,
    pub last: *mut u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_tokens_t {
    pub start: *mut yaml_token_t,
    pub end: *mut yaml_token_t,
    pub head: *mut yaml_token_t,
    pub tail: *mut yaml_token_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_indents_t {
    pub start: *mut c_int,
    pub end: *mut c_int,
    pub top: *mut c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_simple_keys_t {
    pub start: *mut yaml_simple_key_t,
    pub end: *mut yaml_simple_key_t,
    pub top: *mut yaml_simple_key_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_states_t {
    pub start: *mut yaml_parser_state_t,
    pub end: *mut yaml_parser_state_t,
    pub top: *mut yaml_parser_state_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_marks_t {
    pub start: *mut yaml_mark_t,
    pub end: *mut yaml_mark_t,
    pub top: *mut yaml_mark_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_tag_directives_t {
    pub start: *mut yaml_tag_directive_t,
    pub end: *mut yaml_tag_directive_t,
    pub top: *mut yaml_tag_directive_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_aliases_t {
    pub start: *mut yaml_alias_data_t,
    pub end: *mut yaml_alias_data_t,
    pub top: *mut yaml_alias_data_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_parser_t {
    pub error: yaml_error_type_t,
    pub problem: *const c_char,
    pub problem_offset: usize,
    pub problem_value: c_int,
    pub problem_mark: yaml_mark_t,
    pub context: *const c_char,
    pub context_mark: yaml_mark_t,
    pub read_handler: yaml_read_handler_t,
    pub read_handler_data: *mut c_void,
    pub input: yaml_parser_input_t,
    pub eof: c_int,
    pub buffer: yaml_parser_buffer_t,
    pub unread: usize,
    pub raw_buffer: yaml_parser_raw_buffer_t,
    pub encoding: yaml_encoding_t,
    pub offset: usize,
    pub mark: yaml_mark_t,
    pub stream_start_produced: c_int,
    pub stream_end_produced: c_int,
    pub flow_level: c_int,
    pub tokens: yaml_parser_tokens_t,
    pub tokens_parsed: usize,
    pub token_available: c_int,
    pub indents: yaml_parser_indents_t,
    pub indent: c_int,
    pub simple_key_allowed: c_int,
    pub simple_keys: yaml_parser_simple_keys_t,
    pub states: yaml_parser_states_t,
    pub state: yaml_parser_state_t,
    pub marks: yaml_parser_marks_t,
    pub tag_directives: yaml_parser_tag_directives_t,
    pub aliases: yaml_parser_aliases_t,
    pub document: *mut yaml_document_t,
}

pub type yaml_write_handler_t =
    Option<unsafe extern "C" fn(data: *mut c_void, buffer: *mut u8, size: usize) -> c_int>;

#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum yaml_emitter_state_t {
    YAML_EMIT_STREAM_START_STATE = 0,
    YAML_EMIT_FIRST_DOCUMENT_START_STATE = 1,
    YAML_EMIT_DOCUMENT_START_STATE = 2,
    YAML_EMIT_DOCUMENT_CONTENT_STATE = 3,
    YAML_EMIT_DOCUMENT_END_STATE = 4,
    YAML_EMIT_FLOW_SEQUENCE_FIRST_ITEM_STATE = 5,
    YAML_EMIT_FLOW_SEQUENCE_ITEM_STATE = 6,
    YAML_EMIT_FLOW_MAPPING_FIRST_KEY_STATE = 7,
    YAML_EMIT_FLOW_MAPPING_KEY_STATE = 8,
    YAML_EMIT_FLOW_MAPPING_SIMPLE_VALUE_STATE = 9,
    YAML_EMIT_FLOW_MAPPING_VALUE_STATE = 10,
    YAML_EMIT_BLOCK_SEQUENCE_FIRST_ITEM_STATE = 11,
    YAML_EMIT_BLOCK_SEQUENCE_ITEM_STATE = 12,
    YAML_EMIT_BLOCK_MAPPING_FIRST_KEY_STATE = 13,
    YAML_EMIT_BLOCK_MAPPING_KEY_STATE = 14,
    YAML_EMIT_BLOCK_MAPPING_SIMPLE_VALUE_STATE = 15,
    YAML_EMIT_BLOCK_MAPPING_VALUE_STATE = 16,
    YAML_EMIT_END_STATE = 17,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_anchors_t {
    pub references: c_int,
    pub anchor: c_int,
    pub serialized: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_output_string_t {
    pub buffer: *mut u8,
    pub size: usize,
    pub size_written: *mut usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union yaml_emitter_output_t {
    pub string: yaml_emitter_output_string_t,
    pub file: *mut yaml_file_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_buffer_t {
    pub start: *mut yaml_char_t,
    pub end: *mut yaml_char_t,
    pub pointer: *mut yaml_char_t,
    pub last: *mut yaml_char_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_raw_buffer_t {
    pub start: *mut u8,
    pub end: *mut u8,
    pub pointer: *mut u8,
    pub last: *mut u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_states_t {
    pub start: *mut yaml_emitter_state_t,
    pub end: *mut yaml_emitter_state_t,
    pub top: *mut yaml_emitter_state_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_events_t {
    pub start: *mut yaml_event_t,
    pub end: *mut yaml_event_t,
    pub head: *mut yaml_event_t,
    pub tail: *mut yaml_event_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_indents_t {
    pub start: *mut c_int,
    pub end: *mut c_int,
    pub top: *mut c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_tag_directives_t {
    pub start: *mut yaml_tag_directive_t,
    pub end: *mut yaml_tag_directive_t,
    pub top: *mut yaml_tag_directive_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_anchor_data_t {
    pub anchor: *mut yaml_char_t,
    pub anchor_length: usize,
    pub alias: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_tag_data_t {
    pub handle: *mut yaml_char_t,
    pub handle_length: usize,
    pub suffix: *mut yaml_char_t,
    pub suffix_length: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_scalar_data_t {
    pub value: *mut yaml_char_t,
    pub length: usize,
    pub multiline: c_int,
    pub flow_plain_allowed: c_int,
    pub block_plain_allowed: c_int,
    pub single_quoted_allowed: c_int,
    pub block_allowed: c_int,
    pub style: yaml_scalar_style_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yaml_emitter_t {
    pub error: yaml_error_type_t,
    pub problem: *const c_char,
    pub write_handler: yaml_write_handler_t,
    pub write_handler_data: *mut c_void,
    pub output: yaml_emitter_output_t,
    pub buffer: yaml_emitter_buffer_t,
    pub raw_buffer: yaml_emitter_raw_buffer_t,
    pub encoding: yaml_encoding_t,
    pub canonical: c_int,
    pub best_indent: c_int,
    pub best_width: c_int,
    pub unicode: c_int,
    pub line_break: yaml_break_t,
    pub states: yaml_emitter_states_t,
    pub state: yaml_emitter_state_t,
    pub events: yaml_emitter_events_t,
    pub indents: yaml_emitter_indents_t,
    pub tag_directives: yaml_emitter_tag_directives_t,
    pub indent: c_int,
    pub flow_level: c_int,
    pub root_context: c_int,
    pub sequence_context: c_int,
    pub mapping_context: c_int,
    pub simple_key_context: c_int,
    pub line: c_int,
    pub column: c_int,
    pub whitespace: c_int,
    pub indention: c_int,
    pub open_ended: c_int,
    pub anchor_data: yaml_emitter_anchor_data_t,
    pub tag_data: yaml_emitter_tag_data_t,
    pub scalar_data: yaml_emitter_scalar_data_t,
    pub opened: c_int,
    pub closed: c_int,
    pub anchors: *mut yaml_anchors_t,
    pub last_anchor_id: c_int,
    pub document: *mut yaml_document_t,
}
