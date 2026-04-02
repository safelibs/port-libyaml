use core::ffi::c_int;
use core::mem::{size_of, zeroed};
use core::ptr;

use crate::alloc;
use crate::document::{DEFAULT_MAPPING_TAG, DEFAULT_SCALAR_TAG, DEFAULT_SEQUENCE_TAG};
use crate::emitter::yaml_emitter_emit_impl;
use crate::event::{
    initialize_alias_event, initialize_document_end_event, initialize_document_start_event,
    initialize_mapping_end_event, initialize_mapping_start_event, initialize_scalar_event,
    initialize_sequence_end_event, initialize_sequence_start_event, initialize_stream_end_event,
    initialize_stream_start_event, zero_mark,
};
use crate::externs::strcmp;
use crate::ffi;
use crate::success::{Success, FAIL, OK};
use crate::types::{
    yaml_anchors_t, yaml_document_t, yaml_emitter_t, yaml_encoding_t::YAML_ANY_ENCODING,
    yaml_event_t, yaml_node_t, yaml_node_type_t,
};
use crate::{yaml_document_delete, yaml_free, yaml_malloc};

unsafe fn yaml_emitter_open_impl(emitter: *mut yaml_emitter_t) -> Success {
    if emitter.is_null() || (*emitter).opened != 0 {
        return FAIL;
    }

    let mut event = zeroed::<yaml_event_t>();
    initialize_stream_start_event(&mut event, YAML_ANY_ENCODING, zero_mark(), zero_mark());
    if yaml_emitter_emit_impl(emitter, &mut event).fail {
        return FAIL;
    }

    (*emitter).opened = 1;
    OK
}

#[no_mangle]
pub unsafe extern "C" fn yaml_emitter_open(emitter: *mut yaml_emitter_t) -> c_int {
    ffi::int_boundary(|| unsafe {
        if yaml_emitter_open_impl(emitter).ok {
            crate::OK
        } else {
            crate::FAIL
        }
    })
}

unsafe fn yaml_emitter_close_impl(emitter: *mut yaml_emitter_t) -> Success {
    if emitter.is_null() || (*emitter).opened == 0 {
        return FAIL;
    }
    if (*emitter).closed != 0 {
        return OK;
    }

    let mut event = zeroed::<yaml_event_t>();
    initialize_stream_end_event(&mut event, zero_mark(), zero_mark());
    if yaml_emitter_emit_impl(emitter, &mut event).fail {
        return FAIL;
    }

    (*emitter).closed = 1;
    OK
}

#[no_mangle]
pub unsafe extern "C" fn yaml_emitter_close(emitter: *mut yaml_emitter_t) -> c_int {
    ffi::int_boundary(|| unsafe {
        if yaml_emitter_close_impl(emitter).ok {
            crate::OK
        } else {
            crate::FAIL
        }
    })
}

#[inline]
unsafe fn document_node_count(document: *const yaml_document_t) -> usize {
    if document.is_null() || (*document).nodes.start.is_null() || (*document).nodes.top.is_null() {
        0
    } else {
        (*document).nodes.top.offset_from((*document).nodes.start) as usize
    }
}

unsafe fn yaml_emitter_delete_document_and_anchors(emitter: *mut yaml_emitter_t) {
    if emitter.is_null() || (*emitter).document.is_null() {
        return;
    }

    if (*emitter).anchors.is_null() {
        yaml_document_delete((*emitter).document);
        (*emitter).document = ptr::null_mut();
        (*emitter).last_anchor_id = 0;
        return;
    }

    let document = (*emitter).document;
    let count = document_node_count(document);
    for index in 0..count {
        let node = &mut *(*document).nodes.start.add(index);
        let anchor = &*(*emitter).anchors.add(index);
        if anchor.serialized == 0 {
            yaml_free(node.tag.cast());
            if node.r#type == yaml_node_type_t::YAML_SCALAR_NODE {
                yaml_free(node.data.scalar.value.cast());
            }
        }

        match node.r#type {
            yaml_node_type_t::YAML_SEQUENCE_NODE => {
                STACK_DEL!(node.data.sequence.items);
            }
            yaml_node_type_t::YAML_MAPPING_NODE => {
                STACK_DEL!(node.data.mapping.pairs);
            }
            _ => {}
        }
    }

    STACK_DEL!((*document).nodes);
    yaml_free((*emitter).anchors.cast());
    alloc::zero_bytes(document.cast(), size_of::<yaml_document_t>());

    (*emitter).anchors = ptr::null_mut();
    (*emitter).last_anchor_id = 0;
    (*emitter).document = ptr::null_mut();
}

unsafe fn yaml_emitter_anchor_node(emitter: *mut yaml_emitter_t, index: c_int) {
    if emitter.is_null() || (*emitter).document.is_null() || index <= 0 {
        return;
    }

    let node = (*(*emitter).document).nodes.start.add(index as usize - 1);
    let anchor = &mut *(*emitter).anchors.add(index as usize - 1);
    anchor.references += 1;

    if anchor.references == 1 {
        match (*node).r#type {
            yaml_node_type_t::YAML_SEQUENCE_NODE => {
                let mut item = (*node).data.sequence.items.start;
                while item < (*node).data.sequence.items.top {
                    yaml_emitter_anchor_node(emitter, *item);
                    item = item.add(1);
                }
            }
            yaml_node_type_t::YAML_MAPPING_NODE => {
                let mut pair = (*node).data.mapping.pairs.start;
                while pair < (*node).data.mapping.pairs.top {
                    yaml_emitter_anchor_node(emitter, (*pair).key);
                    yaml_emitter_anchor_node(emitter, (*pair).value);
                    pair = pair.add(1);
                }
            }
            _ => {}
        }
    } else if anchor.references == 2 {
        (*anchor).anchor = {
            (*emitter).last_anchor_id += 1;
            (*emitter).last_anchor_id
        };
    }
}

unsafe fn yaml_emitter_generate_anchor(anchor_id: c_int) -> *mut crate::yaml::yaml_char_t {
    let text = format!("id{anchor_id:03}");
    let size = match text.len().checked_add(1) {
        Some(size) => size,
        None => return ptr::null_mut(),
    };
    let anchor = yaml_malloc(size).cast::<crate::yaml::yaml_char_t>();
    if anchor.is_null() {
        return ptr::null_mut();
    }

    ptr::copy_nonoverlapping(text.as_ptr(), anchor.cast::<u8>(), text.len());
    *anchor.add(text.len()) = b'\0';
    anchor
}

unsafe fn yaml_emitter_dump_alias(
    emitter: *mut yaml_emitter_t,
    anchor: *mut crate::yaml::yaml_char_t,
) -> Success {
    let mut event = zeroed::<yaml_event_t>();
    initialize_alias_event(&mut event, anchor, zero_mark(), zero_mark());
    yaml_emitter_emit_impl(emitter, &mut event)
}

unsafe fn yaml_emitter_dump_scalar(
    emitter: *mut yaml_emitter_t,
    node: *mut yaml_node_t,
    anchor: *mut crate::yaml::yaml_char_t,
) -> Success {
    let mut event = zeroed::<yaml_event_t>();
    let implicit =
        bool_to_c_int(strcmp((*node).tag.cast(), DEFAULT_SCALAR_TAG.as_ptr().cast()) == 0);
    initialize_scalar_event(
        &mut event,
        anchor,
        (*node).tag,
        (*node).data.scalar.value,
        (*node).data.scalar.length,
        implicit,
        implicit,
        (*node).data.scalar.style,
        zero_mark(),
        zero_mark(),
    );
    yaml_emitter_emit_impl(emitter, &mut event)
}

unsafe fn yaml_emitter_dump_sequence(
    emitter: *mut yaml_emitter_t,
    node: *mut yaml_node_t,
    anchor: *mut crate::yaml::yaml_char_t,
) -> Success {
    let mut event = zeroed::<yaml_event_t>();
    let implicit =
        bool_to_c_int(strcmp((*node).tag.cast(), DEFAULT_SEQUENCE_TAG.as_ptr().cast()) == 0);
    initialize_sequence_start_event(
        &mut event,
        anchor,
        (*node).tag,
        implicit,
        (*node).data.sequence.style,
        zero_mark(),
        zero_mark(),
    );
    if yaml_emitter_emit_impl(emitter, &mut event).fail {
        return FAIL;
    }

    let mut item = (*node).data.sequence.items.start;
    while item < (*node).data.sequence.items.top {
        if yaml_emitter_dump_node(emitter, *item).fail {
            return FAIL;
        }
        item = item.add(1);
    }

    initialize_sequence_end_event(&mut event, zero_mark(), zero_mark());
    yaml_emitter_emit_impl(emitter, &mut event)
}

unsafe fn yaml_emitter_dump_mapping(
    emitter: *mut yaml_emitter_t,
    node: *mut yaml_node_t,
    anchor: *mut crate::yaml::yaml_char_t,
) -> Success {
    let mut event = zeroed::<yaml_event_t>();
    let implicit =
        bool_to_c_int(strcmp((*node).tag.cast(), DEFAULT_MAPPING_TAG.as_ptr().cast()) == 0);
    initialize_mapping_start_event(
        &mut event,
        anchor,
        (*node).tag,
        implicit,
        (*node).data.mapping.style,
        zero_mark(),
        zero_mark(),
    );
    if yaml_emitter_emit_impl(emitter, &mut event).fail {
        return FAIL;
    }

    let mut pair = (*node).data.mapping.pairs.start;
    while pair < (*node).data.mapping.pairs.top {
        if yaml_emitter_dump_node(emitter, (*pair).key).fail
            || yaml_emitter_dump_node(emitter, (*pair).value).fail
        {
            return FAIL;
        }
        pair = pair.add(1);
    }

    initialize_mapping_end_event(&mut event, zero_mark(), zero_mark());
    yaml_emitter_emit_impl(emitter, &mut event)
}

unsafe fn yaml_emitter_dump_node(emitter: *mut yaml_emitter_t, index: c_int) -> Success {
    let node = (*(*emitter).document).nodes.start.add(index as usize - 1);
    let anchor_id = (*(*emitter).anchors.add(index as usize - 1)).anchor;
    let anchor = if anchor_id != 0 {
        yaml_emitter_generate_anchor(anchor_id)
    } else {
        ptr::null_mut()
    };
    if anchor_id != 0 && anchor.is_null() {
        return FAIL;
    }

    if (*(*emitter).anchors.add(index as usize - 1)).serialized != 0 {
        return yaml_emitter_dump_alias(emitter, anchor);
    }

    (*(*emitter).anchors.add(index as usize - 1)).serialized = 1;
    match (*node).r#type {
        yaml_node_type_t::YAML_SCALAR_NODE => yaml_emitter_dump_scalar(emitter, node, anchor),
        yaml_node_type_t::YAML_SEQUENCE_NODE => yaml_emitter_dump_sequence(emitter, node, anchor),
        yaml_node_type_t::YAML_MAPPING_NODE => yaml_emitter_dump_mapping(emitter, node, anchor),
        _ => FAIL,
    }
}

unsafe fn yaml_emitter_dump_impl(
    emitter: *mut yaml_emitter_t,
    document: *mut yaml_document_t,
) -> Success {
    (*emitter).document = document;

    if (*emitter).opened == 0 && yaml_emitter_open_impl(emitter).fail {
        yaml_emitter_delete_document_and_anchors(emitter);
        return FAIL;
    }

    if STACK_EMPTY!((*document).nodes) {
        let result = yaml_emitter_close_impl(emitter);
        yaml_emitter_delete_document_and_anchors(emitter);
        return result;
    }

    let count = document_node_count(document);
    let anchors_size = match size_of::<yaml_anchors_t>().checked_mul(count) {
        Some(size) => size,
        None => {
            yaml_emitter_delete_document_and_anchors(emitter);
            return FAIL;
        }
    };
    (*emitter).anchors = yaml_malloc(anchors_size).cast::<yaml_anchors_t>();
    if (*emitter).anchors.is_null() {
        yaml_emitter_delete_document_and_anchors(emitter);
        return FAIL;
    }
    alloc::zero_bytes((*emitter).anchors.cast(), anchors_size);

    let mut event = zeroed::<yaml_event_t>();
    initialize_document_start_event(
        &mut event,
        (*document).version_directive,
        (*document).tag_directives.start,
        (*document).tag_directives.end,
        (*document).start_implicit,
        zero_mark(),
        zero_mark(),
    );
    if yaml_emitter_emit_impl(emitter, &mut event).fail {
        yaml_emitter_delete_document_and_anchors(emitter);
        return FAIL;
    }

    yaml_emitter_anchor_node(emitter, 1);
    if yaml_emitter_dump_node(emitter, 1).fail {
        yaml_emitter_delete_document_and_anchors(emitter);
        return FAIL;
    }

    initialize_document_end_event(
        &mut event,
        (*document).end_implicit,
        zero_mark(),
        zero_mark(),
    );
    let result = yaml_emitter_emit_impl(emitter, &mut event);
    yaml_emitter_delete_document_and_anchors(emitter);
    result
}

#[inline]
fn bool_to_c_int(value: bool) -> c_int {
    if value {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn yaml_emitter_dump(
    emitter: *mut yaml_emitter_t,
    document: *mut yaml_document_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if emitter.is_null() || document.is_null() {
            return crate::FAIL;
        }

        if yaml_emitter_dump_impl(emitter, document).ok {
            crate::OK
        } else {
            crate::FAIL
        }
    })
}
