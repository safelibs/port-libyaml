use core::ffi::c_int;
use core::ptr;

use crate::alloc;
use crate::event::{duplicate_scalar_value, duplicate_utf8_c_string, zero_mark};
use crate::ffi;
use crate::types::{
    yaml_char_t, yaml_document_nodes_t, yaml_document_t, yaml_document_tag_directives_t,
    yaml_error_type_t, yaml_mapping_style_t, yaml_mark_t, yaml_node_data_mapping_pairs_t,
    yaml_node_data_mapping_t, yaml_node_data_scalar_t, yaml_node_data_sequence_items_t,
    yaml_node_data_sequence_t, yaml_node_data_t, yaml_node_pair_t, yaml_node_t, yaml_node_type_t,
    yaml_parser_tag_directives_t, yaml_scalar_style_t, yaml_sequence_style_t, yaml_tag_directive_t,
    yaml_version_directive_t,
};
use crate::{yaml_free, PointerExt, FAIL, OK};

pub(crate) const DEFAULT_SCALAR_TAG: &[u8] = b"tag:yaml.org,2002:str\0";
pub(crate) const DEFAULT_SEQUENCE_TAG: &[u8] = b"tag:yaml.org,2002:seq\0";
pub(crate) const DEFAULT_MAPPING_TAG: &[u8] = b"tag:yaml.org,2002:map\0";

#[repr(C)]
struct Context {
    error: yaml_error_type_t,
}

#[inline]
unsafe fn document_node_count(document: *const yaml_document_t) -> isize {
    if (*document).nodes.start.is_null() || (*document).nodes.top.is_null() {
        0
    } else {
        (*document).nodes.top.c_offset_from((*document).nodes.start)
    }
}

#[inline]
unsafe fn lookup_node(document: *mut yaml_document_t, index: c_int) -> *mut yaml_node_t {
    if document.is_null() || index <= 0 {
        return ptr::null_mut();
    }
    let count = document_node_count(document);
    if index as isize > count {
        return ptr::null_mut();
    }
    (*document).nodes.start.add(index as usize - 1)
}

#[inline]
unsafe fn initialize_document_struct(
    document: *mut yaml_document_t,
    nodes_start: *mut yaml_node_t,
    nodes_end: *mut yaml_node_t,
    version_directive: *mut yaml_version_directive_t,
    tag_directives_start: *mut yaml_tag_directive_t,
    tag_directives_end: *mut yaml_tag_directive_t,
    start_implicit: c_int,
    end_implicit: c_int,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
) {
    *document = yaml_document_t {
        nodes: yaml_document_nodes_t {
            start: nodes_start,
            end: nodes_end,
            top: nodes_start,
        },
        version_directive,
        tag_directives: yaml_document_tag_directives_t {
            start: tag_directives_start,
            end: tag_directives_end,
        },
        start_implicit,
        end_implicit,
        start_mark,
        end_mark,
    };
}

#[inline]
unsafe fn initialize_scalar_node(
    tag: *mut yaml_char_t,
    value: *mut yaml_char_t,
    length: usize,
    style: yaml_scalar_style_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
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
    style: yaml_sequence_style_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
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
    style: yaml_mapping_style_t,
    start_mark: yaml_mark_t,
    end_mark: yaml_mark_t,
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

#[no_mangle]
pub unsafe extern "C" fn yaml_document_initialize(
    document: *mut yaml_document_t,
    version_directive: *mut yaml_version_directive_t,
    tag_directives_start: *mut yaml_tag_directive_t,
    tag_directives_end: *mut yaml_tag_directive_t,
    start_implicit: c_int,
    end_implicit: c_int,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if document.is_null() {
            return FAIL;
        }
        if (tag_directives_start.is_null() && !tag_directives_end.is_null())
            || (!tag_directives_start.is_null() && tag_directives_end.is_null())
        {
            return FAIL;
        }

        let mut context = Context {
            error: yaml_error_type_t::YAML_NO_ERROR,
        };
        let mut nodes = yaml_document_nodes_t {
            start: ptr::null_mut(),
            end: ptr::null_mut(),
            top: ptr::null_mut(),
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

        if STACK_INIT!(nodes, yaml_node_t) == FAIL {
            return FAIL;
        }

        if !version_directive.is_null() {
            version_directive_copy =
                crate::yaml_malloc(core::mem::size_of::<yaml_version_directive_t>())
                    .cast::<yaml_version_directive_t>();
            if version_directive_copy.is_null() {
                goto_document_initialize_error(
                    &mut nodes,
                    version_directive_copy,
                    &mut tag_directives_copy,
                    &mut value,
                );
                return FAIL;
            }
            (*version_directive_copy).major = (*version_directive).major;
            (*version_directive_copy).minor = (*version_directive).minor;
        }

        if tag_directives_start != tag_directives_end {
            if STACK_INIT!(tag_directives_copy, yaml_tag_directive_t) == FAIL {
                goto_document_initialize_error(
                    &mut nodes,
                    version_directive_copy,
                    &mut tag_directives_copy,
                    &mut value,
                );
                return FAIL;
            }

            let mut tag_directive = tag_directives_start;
            while tag_directive != tag_directives_end {
                if (*tag_directive).handle.is_null() || (*tag_directive).prefix.is_null() {
                    goto_document_initialize_error(
                        &mut nodes,
                        version_directive_copy,
                        &mut tag_directives_copy,
                        &mut value,
                    );
                    return FAIL;
                }

                value.handle = duplicate_utf8_c_string((*tag_directive).handle);
                value.prefix = duplicate_utf8_c_string((*tag_directive).prefix);
                if value.handle.is_null() || value.prefix.is_null() {
                    goto_document_initialize_error(
                        &mut nodes,
                        version_directive_copy,
                        &mut tag_directives_copy,
                        &mut value,
                    );
                    return FAIL;
                }
                if PUSH!(context, tag_directives_copy, value) == FAIL {
                    goto_document_initialize_error(
                        &mut nodes,
                        version_directive_copy,
                        &mut tag_directives_copy,
                        &mut value,
                    );
                    return FAIL;
                }
                value.handle = ptr::null_mut();
                value.prefix = ptr::null_mut();
                tag_directive = tag_directive.add(1);
            }
        }

        initialize_document_struct(
            document,
            nodes.start,
            nodes.end,
            version_directive_copy,
            tag_directives_copy.start,
            tag_directives_copy.top,
            start_implicit,
            end_implicit,
            zero_mark(),
            zero_mark(),
        );
        OK
    })
}

#[inline]
unsafe fn goto_document_initialize_error(
    nodes: &mut yaml_document_nodes_t,
    version_directive_copy: *mut yaml_version_directive_t,
    tag_directives_copy: &mut yaml_parser_tag_directives_t,
    value: &mut yaml_tag_directive_t,
) {
    STACK_DEL!(*nodes);
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
pub unsafe extern "C" fn yaml_document_delete(document: *mut yaml_document_t) {
    ffi::void_boundary(|| unsafe {
        if document.is_null() {
            return;
        }

        while !STACK_EMPTY!((*document).nodes) {
            let mut node = POP!((*document).nodes);
            yaml_free(node.tag.cast());
            match node.r#type {
                yaml_node_type_t::YAML_SCALAR_NODE => {
                    yaml_free(node.data.scalar.value.cast());
                }
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

        yaml_free((*document).version_directive.cast());
        let mut tag_directive = (*document).tag_directives.start;
        while tag_directive != (*document).tag_directives.end {
            yaml_free((*tag_directive).handle.cast());
            yaml_free((*tag_directive).prefix.cast());
            tag_directive = tag_directive.add(1);
        }
        yaml_free((*document).tag_directives.start.cast());

        alloc::zero_bytes(document.cast(), core::mem::size_of::<yaml_document_t>());
    });
}

#[no_mangle]
pub unsafe extern "C" fn yaml_document_get_node(
    document: *mut yaml_document_t,
    index: c_int,
) -> *mut yaml_node_t {
    ffi::ptr_boundary(|| unsafe { lookup_node(document, index) })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_document_get_root_node(
    document: *mut yaml_document_t,
) -> *mut yaml_node_t {
    ffi::ptr_boundary(|| unsafe {
        if document.is_null() {
            return ptr::null_mut();
        }
        if (*document).nodes.top != (*document).nodes.start {
            (*document).nodes.start
        } else {
            ptr::null_mut()
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_document_add_scalar(
    document: *mut yaml_document_t,
    tag: *const yaml_char_t,
    value: *const yaml_char_t,
    mut length: c_int,
    style: yaml_scalar_style_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if document.is_null() || value.is_null() {
            return FAIL;
        }

        let mut context = Context {
            error: yaml_error_type_t::YAML_NO_ERROR,
        };
        let mut tag_copy = duplicate_utf8_c_string(if tag.is_null() {
            DEFAULT_SCALAR_TAG.as_ptr()
        } else {
            tag
        });
        if tag_copy.is_null() {
            return FAIL;
        }

        if length < 0 {
            length = alloc::c_string_len(value.cast()) as c_int;
        }
        let value_copy = duplicate_scalar_value(value, length as usize);
        if value_copy.is_null() {
            yaml_free(tag_copy.cast());
            return FAIL;
        }

        let node = initialize_scalar_node(
            tag_copy,
            value_copy,
            length as usize,
            style,
            zero_mark(),
            zero_mark(),
        );
        if PUSH!(context, (*document).nodes, node) == FAIL {
            yaml_free(tag_copy.cast());
            yaml_free(value_copy.cast());
            return FAIL;
        }

        document_node_count(document) as c_int
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_document_add_sequence(
    document: *mut yaml_document_t,
    tag: *const yaml_char_t,
    style: yaml_sequence_style_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if document.is_null() {
            return FAIL;
        }

        let mut context = Context {
            error: yaml_error_type_t::YAML_NO_ERROR,
        };
        let tag_copy = duplicate_utf8_c_string(if tag.is_null() {
            DEFAULT_SEQUENCE_TAG.as_ptr()
        } else {
            tag
        });
        if tag_copy.is_null() {
            return FAIL;
        }

        let mut items = yaml_node_data_sequence_items_t {
            start: ptr::null_mut(),
            end: ptr::null_mut(),
            top: ptr::null_mut(),
        };
        if STACK_INIT!(items, c_int) == FAIL {
            yaml_free(tag_copy.cast());
            return FAIL;
        }

        let node = initialize_sequence_node(tag_copy, items, style, zero_mark(), zero_mark());
        if PUSH!(context, (*document).nodes, node) == FAIL {
            STACK_DEL!(items);
            yaml_free(tag_copy.cast());
            return FAIL;
        }

        document_node_count(document) as c_int
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_document_add_mapping(
    document: *mut yaml_document_t,
    tag: *const yaml_char_t,
    style: yaml_mapping_style_t,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        if document.is_null() {
            return FAIL;
        }

        let mut context = Context {
            error: yaml_error_type_t::YAML_NO_ERROR,
        };
        let tag_copy = duplicate_utf8_c_string(if tag.is_null() {
            DEFAULT_MAPPING_TAG.as_ptr()
        } else {
            tag
        });
        if tag_copy.is_null() {
            return FAIL;
        }

        let mut pairs = yaml_node_data_mapping_pairs_t {
            start: ptr::null_mut(),
            end: ptr::null_mut(),
            top: ptr::null_mut(),
        };
        if STACK_INIT!(pairs, yaml_node_pair_t) == FAIL {
            yaml_free(tag_copy.cast());
            return FAIL;
        }

        let node = initialize_mapping_node(tag_copy, pairs, style, zero_mark(), zero_mark());
        if PUSH!(context, (*document).nodes, node) == FAIL {
            STACK_DEL!(pairs);
            yaml_free(tag_copy.cast());
            return FAIL;
        }

        document_node_count(document) as c_int
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_document_append_sequence_item(
    document: *mut yaml_document_t,
    sequence: c_int,
    item: c_int,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        let mut context = Context {
            error: yaml_error_type_t::YAML_NO_ERROR,
        };
        let sequence_node = lookup_node(document, sequence);
        if sequence_node.is_null()
            || (*sequence_node).r#type != yaml_node_type_t::YAML_SEQUENCE_NODE
        {
            return FAIL;
        }
        if lookup_node(document, item).is_null() {
            return FAIL;
        }
        PUSH!(context, (*sequence_node).data.sequence.items, item)
    })
}

#[no_mangle]
pub unsafe extern "C" fn yaml_document_append_mapping_pair(
    document: *mut yaml_document_t,
    mapping: c_int,
    key: c_int,
    value: c_int,
) -> c_int {
    ffi::int_boundary(|| unsafe {
        let mut context = Context {
            error: yaml_error_type_t::YAML_NO_ERROR,
        };
        let mapping_node = lookup_node(document, mapping);
        if mapping_node.is_null() || (*mapping_node).r#type != yaml_node_type_t::YAML_MAPPING_NODE {
            return FAIL;
        }
        if lookup_node(document, key).is_null() || lookup_node(document, value).is_null() {
            return FAIL;
        }

        let pair = yaml_node_pair_t { key, value };
        PUSH!(context, (*mapping_node).data.mapping.pairs, pair)
    })
}
