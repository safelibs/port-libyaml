use std::env;
use std::ffi::CStr;
use std::fs;
use std::mem;
use std::process;

use yaml::{
    yaml_document_add_mapping, yaml_document_add_scalar, yaml_document_add_sequence,
    yaml_document_append_mapping_pair, yaml_document_append_sequence_item, yaml_document_delete,
    yaml_document_get_node, yaml_document_get_root_node, yaml_document_initialize, yaml_document_t,
    yaml_emitter_close, yaml_emitter_delete, yaml_emitter_dump, yaml_emitter_initialize,
    yaml_emitter_open, yaml_emitter_set_canonical, yaml_emitter_set_output_string,
    yaml_emitter_set_unicode, yaml_emitter_t, yaml_node_type_t, yaml_parser_delete,
    yaml_parser_initialize, yaml_parser_load, yaml_parser_set_input_string, yaml_parser_t,
};

const BUFFER_SIZE: usize = 65_536;
const MAX_DOCUMENTS: usize = 16;

fn main() {
    let (canonical, unicode, files) = parse_args();
    if files.is_empty() {
        println!("Usage: {} [-c] [-u] file1.yaml ...", program_name());
        return;
    }

    for (index, path) in files.iter().enumerate() {
        let input = fs::read(path).unwrap_or_else(|error| {
            panic!("failed to read {path}: {error}");
        });

        unsafe {
            let mut parser = mem::zeroed::<yaml_parser_t>();
            let mut emitter = mem::zeroed::<yaml_emitter_t>();
            let mut document = mem::zeroed::<yaml_document_t>();
            let mut output = [0u8; BUFFER_SIZE + 1];
            let mut written = 0usize;
            let mut documents = Vec::<yaml_document_t>::new();
            let mut done = false;
            let mut failed = false;

            print!(
                "[{}] Loading, dumping, and loading again '{}': ",
                index + 1,
                path
            );

            assert_eq!(yaml_parser_initialize(&mut parser), 1);
            yaml_parser_set_input_string(&mut parser, input.as_ptr(), input.len());
            assert_eq!(yaml_emitter_initialize(&mut emitter), 1);
            yaml_emitter_set_output_string(
                &mut emitter,
                output.as_mut_ptr(),
                BUFFER_SIZE,
                &mut written,
            );
            yaml_emitter_set_canonical(&mut emitter, canonical);
            yaml_emitter_set_unicode(&mut emitter, unicode);
            assert_eq!(yaml_emitter_open(&mut emitter), 1);

            while !done {
                if yaml_parser_load(&mut parser, &mut document) == 0 {
                    failed = true;
                    break;
                }

                done = yaml_document_get_root_node(&mut document).is_null();
                if !done {
                    assert!(documents.len() < MAX_DOCUMENTS);
                    let mut copy = mem::zeroed::<yaml_document_t>();
                    assert_eq!(copy_document(&mut copy, &mut document), 1);
                    documents.push(copy);
                    if yaml_emitter_dump(&mut emitter, &mut document) == 0 {
                        failed = true;
                        break;
                    }
                } else {
                    yaml_document_delete(&mut document);
                }
            }

            yaml_parser_delete(&mut parser);
            assert_eq!(yaml_emitter_close(&mut emitter), 1);
            yaml_emitter_delete(&mut emitter);

            if !failed {
                assert_eq!(yaml_parser_initialize(&mut parser), 1);
                yaml_parser_set_input_string(&mut parser, output.as_ptr(), written);

                let mut actual = mem::zeroed::<yaml_document_t>();
                let mut loaded = 0usize;
                loop {
                    if yaml_parser_load(&mut parser, &mut actual) == 0 {
                        failed = true;
                        break;
                    }
                    let stream_done = yaml_document_get_root_node(&mut actual).is_null();
                    if !stream_done {
                        if loaded >= documents.len()
                            || !compare_documents(&documents[loaded], &actual)
                        {
                            failed = true;
                        }
                        loaded += 1;
                    }
                    yaml_document_delete(&mut actual);
                    if failed || stream_done {
                        break;
                    }
                }
                yaml_parser_delete(&mut parser);
            }

            for document in &mut documents {
                yaml_document_delete(document);
            }

            if failed {
                eprintln!("FAILED");
                process::exit(1);
            }

            println!("PASSED (length: {written})");
            print_output(&input, &output[..written]);
        }
    }
}

fn program_name() -> String {
    env::args()
        .next()
        .unwrap_or_else(|| String::from("run_dumper"))
}

fn parse_args() -> (i32, i32, Vec<String>) {
    let mut canonical = 0;
    let mut unicode = 0;
    let mut files = Vec::new();

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "-c" => canonical = 1,
            "-u" => unicode = 1,
            _ if arg.starts_with('-') => {
                println!("Unknown option: '{arg}'");
                process::exit(0);
            }
            _ => files.push(arg),
        }
    }

    (canonical, unicode, files)
}

unsafe fn copy_document(to: *mut yaml_document_t, from: *mut yaml_document_t) -> i32 {
    assert_eq!(
        yaml_document_initialize(
            to,
            (*from).version_directive,
            (*from).tag_directives.start,
            (*from).tag_directives.end,
            (*from).start_implicit,
            (*from).end_implicit,
        ),
        1
    );

    let mut node = (*from).nodes.start;
    while node < (*from).nodes.top {
        let ok = match (*node).r#type {
            yaml_node_type_t::YAML_SCALAR_NODE => yaml_document_add_scalar(
                to,
                (*node).tag,
                (*node).data.scalar.value,
                (*node).data.scalar.length as i32,
                (*node).data.scalar.style,
            ),
            yaml_node_type_t::YAML_SEQUENCE_NODE => {
                yaml_document_add_sequence(to, (*node).tag, (*node).data.sequence.style)
            }
            yaml_node_type_t::YAML_MAPPING_NODE => {
                yaml_document_add_mapping(to, (*node).tag, (*node).data.mapping.style)
            }
            _ => 0,
        };
        if ok == 0 {
            yaml_document_delete(to);
            return 0;
        }
        node = node.add(1);
    }

    node = (*from).nodes.start;
    while node < (*from).nodes.top {
        match (*node).r#type {
            yaml_node_type_t::YAML_SEQUENCE_NODE => {
                let mut item = (*node).data.sequence.items.start;
                while item < (*node).data.sequence.items.top {
                    if yaml_document_append_sequence_item(
                        to,
                        node.offset_from((*from).nodes.start) as i32 + 1,
                        *item,
                    ) == 0
                    {
                        yaml_document_delete(to);
                        return 0;
                    }
                    item = item.add(1);
                }
            }
            yaml_node_type_t::YAML_MAPPING_NODE => {
                let mut pair = (*node).data.mapping.pairs.start;
                while pair < (*node).data.mapping.pairs.top {
                    if yaml_document_append_mapping_pair(
                        to,
                        node.offset_from((*from).nodes.start) as i32 + 1,
                        (*pair).key,
                        (*pair).value,
                    ) == 0
                    {
                        yaml_document_delete(to);
                        return 0;
                    }
                    pair = pair.add(1);
                }
            }
            _ => {}
        }
        node = node.add(1);
    }

    1
}

unsafe fn compare_documents(lhs: &yaml_document_t, rhs: &yaml_document_t) -> bool {
    if lhs.start_implicit != rhs.start_implicit || lhs.end_implicit != rhs.end_implicit {
        return false;
    }

    if !same_optional_version(lhs.version_directive, rhs.version_directive) {
        return false;
    }

    let lhs_tags = lhs.tag_directives.end.offset_from(lhs.tag_directives.start);
    let rhs_tags = rhs.tag_directives.end.offset_from(rhs.tag_directives.start);
    if lhs_tags != rhs_tags {
        return false;
    }
    for index in 0..lhs_tags {
        let left = *lhs.tag_directives.start.offset(index);
        let right = *rhs.tag_directives.start.offset(index);
        if CStr::from_ptr(left.handle.cast()) != CStr::from_ptr(right.handle.cast())
            || CStr::from_ptr(left.prefix.cast()) != CStr::from_ptr(right.prefix.cast())
        {
            return false;
        }
    }

    let lhs_nodes = lhs.nodes.top.offset_from(lhs.nodes.start);
    let rhs_nodes = rhs.nodes.top.offset_from(rhs.nodes.start);
    lhs_nodes == rhs_nodes
        && (lhs_nodes == 0
            || compare_nodes(
                lhs as *const _ as *mut _,
                1,
                rhs as *const _ as *mut _,
                1,
                0,
            ))
}

unsafe fn compare_nodes(
    lhs_document: *mut yaml_document_t,
    lhs_index: i32,
    rhs_document: *mut yaml_document_t,
    rhs_index: i32,
    level: i32,
) -> bool {
    if level > 1000 {
        return false;
    }

    let lhs = yaml_document_get_node(lhs_document, lhs_index);
    let rhs = yaml_document_get_node(rhs_document, rhs_index);
    if lhs.is_null() || rhs.is_null() || (*lhs).r#type != (*rhs).r#type {
        return false;
    }
    if CStr::from_ptr((*lhs).tag.cast()) != CStr::from_ptr((*rhs).tag.cast()) {
        return false;
    }

    match (*lhs).r#type {
        yaml_node_type_t::YAML_SCALAR_NODE => {
            (*lhs).data.scalar.length == (*rhs).data.scalar.length
                && std::slice::from_raw_parts((*lhs).data.scalar.value, (*lhs).data.scalar.length)
                    == std::slice::from_raw_parts(
                        (*rhs).data.scalar.value,
                        (*rhs).data.scalar.length,
                    )
        }
        yaml_node_type_t::YAML_SEQUENCE_NODE => {
            let lhs_len = (*lhs)
                .data
                .sequence
                .items
                .top
                .offset_from((*lhs).data.sequence.items.start);
            let rhs_len = (*rhs)
                .data
                .sequence
                .items
                .top
                .offset_from((*rhs).data.sequence.items.start);
            if lhs_len != rhs_len {
                return false;
            }
            for index in 0..lhs_len {
                if !compare_nodes(
                    lhs_document,
                    *(*lhs).data.sequence.items.start.offset(index),
                    rhs_document,
                    *(*rhs).data.sequence.items.start.offset(index),
                    level + 1,
                ) {
                    return false;
                }
            }
            true
        }
        yaml_node_type_t::YAML_MAPPING_NODE => {
            let lhs_len = (*lhs)
                .data
                .mapping
                .pairs
                .top
                .offset_from((*lhs).data.mapping.pairs.start);
            let rhs_len = (*rhs)
                .data
                .mapping
                .pairs
                .top
                .offset_from((*rhs).data.mapping.pairs.start);
            if lhs_len != rhs_len {
                return false;
            }
            for index in 0..lhs_len {
                let lhs_pair = *(*lhs).data.mapping.pairs.start.offset(index);
                let rhs_pair = *(*rhs).data.mapping.pairs.start.offset(index);
                if !compare_nodes(
                    lhs_document,
                    lhs_pair.key,
                    rhs_document,
                    rhs_pair.key,
                    level + 1,
                ) || !compare_nodes(
                    lhs_document,
                    lhs_pair.value,
                    rhs_document,
                    rhs_pair.value,
                    level + 1,
                ) {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

unsafe fn same_optional_version(
    lhs: *mut yaml::yaml_version_directive_t,
    rhs: *mut yaml::yaml_version_directive_t,
) -> bool {
    match (lhs.is_null(), rhs.is_null()) {
        (true, true) => true,
        (false, false) => (*lhs).major == (*rhs).major && (*lhs).minor == (*rhs).minor,
        _ => false,
    }
}

fn print_output(input: &[u8], output: &[u8]) {
    print!("SOURCE:\n{}", String::from_utf8_lossy(input));
    print!("#### (length: {})\n", input.len());
    print!("OUTPUT:\n{}", String::from_utf8_lossy(output));
    print!("#### (length: {})\n", output.len());
}
