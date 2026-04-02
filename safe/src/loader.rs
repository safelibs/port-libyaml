use core::ffi::c_int;
use core::ptr;

use crate::alloc;
use crate::document::{DEFAULT_MAPPING_TAG, DEFAULT_SCALAR_TAG, DEFAULT_SEQUENCE_TAG};
use crate::event::yaml_event_delete;
use crate::externs::strcmp;
use crate::ffi;
use crate::types::{
    yaml_alias_data_t, yaml_char_t, yaml_document_t, yaml_error_type_t, yaml_event_t,
    yaml_event_type_t, yaml_node_data_mapping_pairs_t, yaml_node_data_mapping_t,
    yaml_node_data_scalar_t, yaml_node_data_sequence_items_t, yaml_node_data_sequence_t,
    yaml_node_data_t, yaml_node_pair_t, yaml_node_t, yaml_node_type_t, yaml_parser_t,
};
use crate::{yaml_free, yaml_strdup, PointerExt, FAIL, OK};

const FOUND_DUPLICATE_ANCHOR_FIRST_OCCURRENCE: &[u8] =
    b"found duplicate anchor; first occurrence\0";
const SECOND_OCCURRENCE: &[u8] = b"second occurrence\0";
const FOUND_UNDEFINED_ALIAS: &[u8] = b"found undefined alias\0";
const BANG_TAG: &[u8] = b"!\0";
const STACK_LIMIT_SIZE: isize = c_int::MAX as isize - 1;

#[repr(C)]
struct LoaderCtx {
    start: *mut c_int,
    end: *mut c_int,
    top: *mut c_int,
}

#[inline]
unsafe fn initialize_scalar_node(
    tag: *mut yaml_char_t,
    value: *mut yaml_char_t,
    length: usize,
    style: crate::types::yaml_scalar_style_t,
    start_mark: crate::types::yaml_mark_t,
    end_mark: crate::types::yaml_mark_t,
) -> yaml_node_t {
    yaml_node_t {
        r#type: yaml_node_type_t::YAML_SCALAR_NODE,
        tag,
        data: yaml_node_data_t {
            scalar: yaml_node_data_scalar_t {
                value,
                length,
                style,
            },
        },
        start_mark,
        end_mark,
    }
}

#[inline]
unsafe fn initialize_sequence_node(
    tag: *mut yaml_char_t,
    items: yaml_node_data_sequence_items_t,
    style: crate::types::yaml_sequence_style_t,
    start_mark: crate::types::yaml_mark_t,
    end_mark: crate::types::yaml_mark_t,
) -> yaml_node_t {
    yaml_node_t {
        r#type: yaml_node_type_t::YAML_SEQUENCE_NODE,
        tag,
        data: yaml_node_data_t {
            sequence: yaml_node_data_sequence_t { items, style },
        },
        start_mark,
        end_mark,
    }
}

#[inline]
unsafe fn initialize_mapping_node(
    tag: *mut yaml_char_t,
    pairs: yaml_node_data_mapping_pairs_t,
    style: crate::types::yaml_mapping_style_t,
    start_mark: crate::types::yaml_mark_t,
    end_mark: crate::types::yaml_mark_t,
) -> yaml_node_t {
    yaml_node_t {
        r#type: yaml_node_type_t::YAML_MAPPING_NODE,
        tag,
        data: yaml_node_data_t {
            mapping: yaml_node_data_mapping_t { pairs, style },
        },
        start_mark,
        end_mark,
    }
}

#[inline]
unsafe fn check_stack_limit<T>(parser: *mut yaml_parser_t, start: *mut T, top: *mut T) -> bool {
    let len = if start.is_null() || top.is_null() {
        if start.is_null() && top.is_null() {
            0
        } else {
            (*parser).error = yaml_error_type_t::YAML_MEMORY_ERROR;
            return false;
        }
    } else {
        top.c_offset_from(start)
    };

    if len < STACK_LIMIT_SIZE {
        true
    } else {
        (*parser).error = yaml_error_type_t::YAML_MEMORY_ERROR;
        false
    }
}

#[inline]
unsafe fn set_composer_error(
    parser: *mut yaml_parser_t,
    problem: &'static [u8],
    problem_mark: crate::types::yaml_mark_t,
) -> c_int {
    (*parser).error = yaml_error_type_t::YAML_COMPOSER_ERROR;
    (*parser).problem = problem.as_ptr().cast();
    (*parser).problem_mark = problem_mark;
    FAIL
}

#[inline]
unsafe fn set_composer_error_context(
    parser: *mut yaml_parser_t,
    context: &'static [u8],
    context_mark: crate::types::yaml_mark_t,
    problem: &'static [u8],
    problem_mark: crate::types::yaml_mark_t,
) -> c_int {
    (*parser).error = yaml_error_type_t::YAML_COMPOSER_ERROR;
    (*parser).context = context.as_ptr().cast();
    (*parser).context_mark = context_mark;
    (*parser).problem = problem.as_ptr().cast();
    (*parser).problem_mark = problem_mark;
    FAIL
}

#[inline]
unsafe fn yaml_parser_delete_aliases(parser: *mut yaml_parser_t) {
    while !STACK_EMPTY!((*parser).aliases) {
        yaml_free(POP!((*parser).aliases).anchor.cast());
    }
    STACK_DEL!((*parser).aliases);
}

unsafe fn yaml_parser_load_document(parser: *mut yaml_parser_t, event: *mut yaml_event_t) -> c_int {
    let mut ctx = LoaderCtx {
        start: ptr::null_mut(),
        end: ptr::null_mut(),
        top: ptr::null_mut(),
    };

    (*(*parser).document).version_directive = (*event).data.document_start.version_directive;
    (*(*parser).document).tag_directives.start = (*event).data.document_start.tag_directives.start;
    (*(*parser).document).tag_directives.end = (*event).data.document_start.tag_directives.end;
    (*(*parser).document).start_implicit = (*event).data.document_start.implicit;
    (*(*parser).document).start_mark = (*event).start_mark;

    if STACK_INIT!(ctx, c_int) == FAIL {
        return FAIL;
    }
    if yaml_parser_load_nodes(parser, &mut ctx) == FAIL {
        STACK_DEL!(ctx);
        return FAIL;
    }
    STACK_DEL!(ctx);
    OK
}

unsafe fn yaml_parser_load_nodes(parser: *mut yaml_parser_t, ctx: *mut LoaderCtx) -> c_int {
    let mut event = core::mem::zeroed::<yaml_event_t>();

    loop {
        if crate::yaml_parser_parse(parser, &mut event) == FAIL {
            return FAIL;
        }

        match event.r#type {
            yaml_event_type_t::YAML_ALIAS_EVENT => {
                if yaml_parser_load_alias(parser, &mut event, ctx) == FAIL {
                    return FAIL;
                }
            }
            yaml_event_type_t::YAML_SCALAR_EVENT => {
                if yaml_parser_load_scalar(parser, &mut event, ctx) == FAIL {
                    return FAIL;
                }
            }
            yaml_event_type_t::YAML_SEQUENCE_START_EVENT => {
                if yaml_parser_load_sequence(parser, &mut event, ctx) == FAIL {
                    return FAIL;
                }
            }
            yaml_event_type_t::YAML_SEQUENCE_END_EVENT => {
                if yaml_parser_load_sequence_end(parser, &mut event, ctx) == FAIL {
                    return FAIL;
                }
            }
            yaml_event_type_t::YAML_MAPPING_START_EVENT => {
                if yaml_parser_load_mapping(parser, &mut event, ctx) == FAIL {
                    return FAIL;
                }
            }
            yaml_event_type_t::YAML_MAPPING_END_EVENT => {
                if yaml_parser_load_mapping_end(parser, &mut event, ctx) == FAIL {
                    return FAIL;
                }
            }
            yaml_event_type_t::YAML_DOCUMENT_END_EVENT => {
                (*(*parser).document).end_implicit = event.data.document_end.implicit;
                (*(*parser).document).end_mark = event.end_mark;
                return OK;
            }
            _ => return FAIL,
        }
    }
}

unsafe fn yaml_parser_register_anchor(
    parser: *mut yaml_parser_t,
    index: c_int,
    anchor: *mut yaml_char_t,
) -> c_int {
    if anchor.is_null() {
        return OK;
    }

    let data = yaml_alias_data_t {
        anchor,
        index,
        mark: (*(*parser).document)
            .nodes
            .start
            .add(index as usize - 1)
            .read()
            .start_mark,
    };

    let mut alias_data = (*parser).aliases.start;
    while alias_data != (*parser).aliases.top {
        if strcmp((*alias_data).anchor.cast(), anchor.cast()) == 0 {
            yaml_free(anchor.cast());
            return set_composer_error_context(
                parser,
                FOUND_DUPLICATE_ANCHOR_FIRST_OCCURRENCE,
                (*alias_data).mark,
                SECOND_OCCURRENCE,
                data.mark,
            );
        }
        alias_data = alias_data.add(1);
    }

    if PUSH!((*parser), (*parser).aliases, data) == FAIL {
        yaml_free(anchor.cast());
        return FAIL;
    }

    OK
}

unsafe fn yaml_parser_load_node_add(
    parser: *mut yaml_parser_t,
    ctx: *mut LoaderCtx,
    index: c_int,
) -> c_int {
    if STACK_EMPTY!((*ctx)) {
        return OK;
    }

    let parent_index = *(*ctx).top.sub(1);
    let parent = (*(*parser).document)
        .nodes
        .start
        .add(parent_index as usize - 1);

    match (*parent).r#type {
        yaml_node_type_t::YAML_SEQUENCE_NODE => {
            if !check_stack_limit(
                parser,
                (*parent).data.sequence.items.start,
                (*parent).data.sequence.items.top,
            ) {
                return FAIL;
            }
            PUSH!((*parser), (*parent).data.sequence.items, index)
        }
        yaml_node_type_t::YAML_MAPPING_NODE => {
            if !STACK_EMPTY!((*parent).data.mapping.pairs) {
                let pair = (*parent).data.mapping.pairs.top.sub(1);
                if (*pair).key != 0 && (*pair).value == 0 {
                    (*pair).value = index;
                    return OK;
                }
            }

            let pair = yaml_node_pair_t {
                key: index,
                value: 0,
            };
            if !check_stack_limit(
                parser,
                (*parent).data.mapping.pairs.start,
                (*parent).data.mapping.pairs.top,
            ) {
                return FAIL;
            }
            PUSH!((*parser), (*parent).data.mapping.pairs, pair)
        }
        _ => FAIL,
    }
}

unsafe fn yaml_parser_load_alias(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut LoaderCtx,
) -> c_int {
    let anchor = (*event).data.alias.anchor;
    let mut alias_data = (*parser).aliases.start;

    while alias_data != (*parser).aliases.top {
        if strcmp((*alias_data).anchor.cast(), anchor.cast()) == 0 {
            yaml_free(anchor.cast());
            return yaml_parser_load_node_add(parser, ctx, (*alias_data).index);
        }
        alias_data = alias_data.add(1);
    }

    yaml_free(anchor.cast());
    set_composer_error(parser, FOUND_UNDEFINED_ALIAS, (*event).start_mark)
}

unsafe fn yaml_parser_load_scalar(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut LoaderCtx,
) -> c_int {
    let mut tag = (*event).data.scalar.tag;

    if !check_stack_limit(
        parser,
        (*(*parser).document).nodes.start,
        (*(*parser).document).nodes.top,
    ) {
        goto_scalar_error(event, tag);
        return FAIL;
    }

    if tag.is_null() || strcmp(tag.cast(), BANG_TAG.as_ptr().cast()) == 0 {
        yaml_free(tag.cast());
        tag = yaml_strdup(DEFAULT_SCALAR_TAG.as_ptr());
        if tag.is_null() {
            goto_scalar_error(event, tag);
            return FAIL;
        }
    }

    let node = initialize_scalar_node(
        tag,
        (*event).data.scalar.value,
        (*event).data.scalar.length,
        (*event).data.scalar.style,
        (*event).start_mark,
        (*event).end_mark,
    );
    if PUSH!((*parser), (*(*parser).document).nodes, node) == FAIL {
        goto_scalar_error(event, tag);
        return FAIL;
    }

    let index = (*(*parser).document)
        .nodes
        .top
        .c_offset_from((*(*parser).document).nodes.start) as c_int;

    if yaml_parser_register_anchor(parser, index, (*event).data.scalar.anchor) == FAIL {
        return FAIL;
    }

    yaml_parser_load_node_add(parser, ctx, index)
}

#[inline]
unsafe fn goto_scalar_error(event: *mut yaml_event_t, tag: *mut yaml_char_t) {
    yaml_free(tag.cast());
    yaml_free((*event).data.scalar.anchor.cast());
    yaml_free((*event).data.scalar.value.cast());
}

unsafe fn yaml_parser_load_sequence(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut LoaderCtx,
) -> c_int {
    let mut items = yaml_node_data_sequence_items_t {
        start: ptr::null_mut(),
        end: ptr::null_mut(),
        top: ptr::null_mut(),
    };
    let mut tag = (*event).data.sequence_start.tag;

    if !check_stack_limit(
        parser,
        (*(*parser).document).nodes.start,
        (*(*parser).document).nodes.top,
    ) {
        goto_sequence_error(event, &mut items, tag);
        return FAIL;
    }

    if tag.is_null() || strcmp(tag.cast(), BANG_TAG.as_ptr().cast()) == 0 {
        yaml_free(tag.cast());
        tag = yaml_strdup(DEFAULT_SEQUENCE_TAG.as_ptr());
        if tag.is_null() {
            goto_sequence_error(event, &mut items, tag);
            return FAIL;
        }
    }

    if STACK_INIT!(items, c_int) == FAIL {
        goto_sequence_error(event, &mut items, tag);
        return FAIL;
    }

    let node = initialize_sequence_node(
        tag,
        items,
        (*event).data.sequence_start.style,
        (*event).start_mark,
        (*event).end_mark,
    );
    if PUSH!((*parser), (*(*parser).document).nodes, node) == FAIL {
        goto_sequence_error(event, &mut items, tag);
        return FAIL;
    }

    let index = (*(*parser).document)
        .nodes
        .top
        .c_offset_from((*(*parser).document).nodes.start) as c_int;

    if yaml_parser_register_anchor(parser, index, (*event).data.sequence_start.anchor) == FAIL {
        return FAIL;
    }
    if yaml_parser_load_node_add(parser, ctx, index) == FAIL {
        return FAIL;
    }
    if !check_stack_limit(parser, (*ctx).start, (*ctx).top) {
        return FAIL;
    }
    PUSH!((*parser), (*ctx), index)
}

#[inline]
unsafe fn goto_sequence_error(
    event: *mut yaml_event_t,
    items: &mut yaml_node_data_sequence_items_t,
    tag: *mut yaml_char_t,
) {
    STACK_DEL!(*items);
    yaml_free(tag.cast());
    yaml_free((*event).data.sequence_start.anchor.cast());
}

unsafe fn yaml_parser_load_sequence_end(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut LoaderCtx,
) -> c_int {
    if STACK_EMPTY!((*ctx)) {
        return FAIL;
    }

    let index = *(*ctx).top.sub(1);
    let node = (*(*parser).document).nodes.start.add(index as usize - 1);
    if (*node).r#type != yaml_node_type_t::YAML_SEQUENCE_NODE {
        return FAIL;
    }
    (*node).end_mark = (*event).end_mark;
    let _ = POP!((*ctx));
    OK
}

unsafe fn yaml_parser_load_mapping(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut LoaderCtx,
) -> c_int {
    let mut pairs = yaml_node_data_mapping_pairs_t {
        start: ptr::null_mut(),
        end: ptr::null_mut(),
        top: ptr::null_mut(),
    };
    let mut tag = (*event).data.mapping_start.tag;

    if !check_stack_limit(
        parser,
        (*(*parser).document).nodes.start,
        (*(*parser).document).nodes.top,
    ) {
        goto_mapping_error(event, &mut pairs, tag);
        return FAIL;
    }

    if tag.is_null() || strcmp(tag.cast(), BANG_TAG.as_ptr().cast()) == 0 {
        yaml_free(tag.cast());
        tag = yaml_strdup(DEFAULT_MAPPING_TAG.as_ptr());
        if tag.is_null() {
            goto_mapping_error(event, &mut pairs, tag);
            return FAIL;
        }
    }

    if STACK_INIT!(pairs, yaml_node_pair_t) == FAIL {
        goto_mapping_error(event, &mut pairs, tag);
        return FAIL;
    }

    let node = initialize_mapping_node(
        tag,
        pairs,
        (*event).data.mapping_start.style,
        (*event).start_mark,
        (*event).end_mark,
    );
    if PUSH!((*parser), (*(*parser).document).nodes, node) == FAIL {
        goto_mapping_error(event, &mut pairs, tag);
        return FAIL;
    }

    let index = (*(*parser).document)
        .nodes
        .top
        .c_offset_from((*(*parser).document).nodes.start) as c_int;

    if yaml_parser_register_anchor(parser, index, (*event).data.mapping_start.anchor) == FAIL {
        return FAIL;
    }
    if yaml_parser_load_node_add(parser, ctx, index) == FAIL {
        return FAIL;
    }
    if !check_stack_limit(parser, (*ctx).start, (*ctx).top) {
        return FAIL;
    }
    PUSH!((*parser), (*ctx), index)
}

#[inline]
unsafe fn goto_mapping_error(
    event: *mut yaml_event_t,
    pairs: &mut yaml_node_data_mapping_pairs_t,
    tag: *mut yaml_char_t,
) {
    STACK_DEL!(*pairs);
    yaml_free(tag.cast());
    yaml_free((*event).data.mapping_start.anchor.cast());
}

unsafe fn yaml_parser_load_mapping_end(
    parser: *mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut LoaderCtx,
) -> c_int {
    if STACK_EMPTY!((*ctx)) {
        return FAIL;
    }

    let index = *(*ctx).top.sub(1);
    let node = (*(*parser).document).nodes.start.add(index as usize - 1);
    if (*node).r#type != yaml_node_type_t::YAML_MAPPING_NODE {
        return FAIL;
    }
    (*node).end_mark = (*event).end_mark;
    let _ = POP!((*ctx));
    OK
}

#[no_mangle]
pub unsafe extern "C" fn yaml_parser_load(
    parser: *mut yaml_parser_t,
    document: *mut yaml_document_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if parser.is_null() || document.is_null() {
            return FAIL;
        }

        let mut event = core::mem::zeroed::<yaml_event_t>();

        alloc::zero_bytes(document.cast(), core::mem::size_of::<yaml_document_t>());
        if STACK_INIT!((*document).nodes, yaml_node_t) == FAIL {
            return FAIL;
        }

        if (*parser).stream_start_produced == 0 {
            if crate::yaml_parser_parse(parser, &mut event) == FAIL {
                goto_load_error(parser, document);
                return FAIL;
            }
        }

        if (*parser).stream_end_produced != 0 {
            return OK;
        }

        if crate::yaml_parser_parse(parser, &mut event) == FAIL {
            goto_load_error(parser, document);
            return FAIL;
        }
        if event.r#type == yaml_event_type_t::YAML_STREAM_END_EVENT {
            return OK;
        }

        if STACK_INIT!((*parser).aliases, yaml_alias_data_t) == FAIL {
            yaml_event_delete(&mut event);
            goto_load_error(parser, document);
            return FAIL;
        }

        (*parser).document = document;
        if yaml_parser_load_document(parser, &mut event) == FAIL {
            goto_load_error(parser, document);
            return FAIL;
        }

        yaml_parser_delete_aliases(parser);
        (*parser).document = ptr::null_mut();
        OK
    })
}

#[inline]
unsafe fn goto_load_error(parser: *mut yaml_parser_t, document: *mut yaml_document_t) {
    yaml_parser_delete_aliases(parser);
    crate::yaml_document_delete(document);
    (*parser).document = ptr::null_mut();
}
