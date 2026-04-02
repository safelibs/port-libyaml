#include <yaml.h>

#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>

#define BUFFER_SIZE 8192
#define EXAMPLE_TAG_PREFIX "tag:example.com,2026:"
#define EXAMPLE_TEXT_TAG EXAMPLE_TAG_PREFIX "text"

static const yaml_char_t utf8_greeting[] =
    "Hi is \xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82";

static const unsigned char utf16le_greeting_no_bom[] = {
    'H', 0x00, 'i', 0x00, ' ', 0x00,
    'i', 0x00, 's', 0x00, ' ', 0x00,
    0x1f, 0x04, 0x40, 0x04, 0x38, 0x04,
    0x32, 0x04, 0x35, 0x04, 0x42, 0x04
};

typedef struct {
    const unsigned char *input;
    size_t size;
    size_t offset;
    size_t chunk;
} memory_reader_t;

typedef struct {
    unsigned char *output;
    size_t capacity;
    size_t written;
} memory_writer_t;

static int
memory_read_handler(void *data, unsigned char *buffer, size_t size,
        size_t *size_read)
{
    memory_reader_t *reader = (memory_reader_t *)data;
    size_t remaining = reader->size - reader->offset;

    if (!remaining) {
        *size_read = 0;
        return 1;
    }

    if (reader->chunk && size > reader->chunk) {
        size = reader->chunk;
    }
    if (size > remaining) {
        size = remaining;
    }

    memcpy(buffer, reader->input + reader->offset, size);
    reader->offset += size;
    *size_read = size;

    return 1;
}

static int
memory_write_handler(void *data, unsigned char *buffer, size_t size)
{
    memory_writer_t *writer = (memory_writer_t *)data;

    if (writer->written + size >= writer->capacity) {
        return 0;
    }

    memcpy(writer->output + writer->written, buffer, size);
    writer->written += size;
    writer->output[writer->written] = '\0';

    return 1;
}

static int
buffer_contains(const unsigned char *buffer, size_t size, const char *needle)
{
    size_t needle_length = strlen(needle);
    size_t k;

    if (!needle_length || needle_length > size) {
        return 0;
    }

    for (k = 0; k + needle_length <= size; k++) {
        if (memcmp(buffer + k, needle, needle_length) == 0) {
            return 1;
        }
    }

    return 0;
}

static void
assert_crlf_line_breaks(const unsigned char *buffer, size_t size)
{
    size_t k;

    for (k = 0; k < size; k++) {
        if (buffer[k] == '\n') {
            assert(k > 0);
            assert(buffer[k-1] == '\r');
        }
    }
}

static void
assert_scalar_node(yaml_node_t *node, const yaml_char_t *tag,
        const yaml_char_t *value)
{
    size_t expected_length = strlen((const char *)value);

    assert(node);
    assert(node->type == YAML_SCALAR_NODE);
    assert(strcmp((char *)node->tag, (char *)tag) == 0);
    assert(node->data.scalar.length == expected_length);
    assert(memcmp(node->data.scalar.value, value, expected_length) == 0);
}

static yaml_node_t *
lookup_mapping_value(yaml_document_t *document, yaml_node_t *mapping,
        const char *key)
{
    yaml_node_pair_t *pair;

    assert(mapping);
    assert(mapping->type == YAML_MAPPING_NODE);

    for (pair = mapping->data.mapping.pairs.start;
            pair < mapping->data.mapping.pairs.top; pair++) {
        yaml_node_t *key_node = yaml_document_get_node(document, pair->key);

        assert(key_node);
        if (key_node->type == YAML_SCALAR_NODE
                && strlen(key) == key_node->data.scalar.length
                && memcmp(key_node->data.scalar.value, key,
                    key_node->data.scalar.length) == 0) {
            yaml_node_t *value_node = yaml_document_get_node(document, pair->value);
            assert(value_node);
            return value_node;
        }
    }

    return NULL;
}

static int
compare_nodes(yaml_document_t *document1, int index1,
        yaml_document_t *document2, int index2, int level)
{
    int k;
    yaml_node_t *node1;
    yaml_node_t *node2;

    if (level++ > 1000) {
        return 0;
    }

    node1 = yaml_document_get_node(document1, index1);
    node2 = yaml_document_get_node(document2, index2);

    assert(node1);
    assert(node2);

    if (node1->type != node2->type) {
        return 0;
    }

    if (strcmp((char *)node1->tag, (char *)node2->tag) != 0) {
        return 0;
    }

    switch (node1->type) {
        case YAML_SCALAR_NODE:
            if (node1->data.scalar.length != node2->data.scalar.length) {
                return 0;
            }
            if (memcmp(node1->data.scalar.value, node2->data.scalar.value,
                        node1->data.scalar.length) != 0) {
                return 0;
            }
            break;

        case YAML_SEQUENCE_NODE:
            if ((node1->data.sequence.items.top - node1->data.sequence.items.start) !=
                    (node2->data.sequence.items.top - node2->data.sequence.items.start)) {
                return 0;
            }
            for (k = 0;
                    k < (node1->data.sequence.items.top - node1->data.sequence.items.start);
                    k++) {
                if (!compare_nodes(document1, node1->data.sequence.items.start[k],
                            document2, node2->data.sequence.items.start[k], level)) {
                    return 0;
                }
            }
            break;

        case YAML_MAPPING_NODE:
            if ((node1->data.mapping.pairs.top - node1->data.mapping.pairs.start) !=
                    (node2->data.mapping.pairs.top - node2->data.mapping.pairs.start)) {
                return 0;
            }
            for (k = 0;
                    k < (node1->data.mapping.pairs.top - node1->data.mapping.pairs.start);
                    k++) {
                if (!compare_nodes(document1,
                            node1->data.mapping.pairs.start[k].key,
                            document2, node2->data.mapping.pairs.start[k].key, level)) {
                    return 0;
                }
                if (!compare_nodes(document1,
                            node1->data.mapping.pairs.start[k].value,
                            document2, node2->data.mapping.pairs.start[k].value, level)) {
                    return 0;
                }
            }
            break;

        default:
            assert(0);
            break;
    }

    return 1;
}

static int
compare_documents(yaml_document_t *document1, yaml_document_t *document2)
{
    int k;

    if (document1->start_implicit != document2->start_implicit
            || document1->end_implicit != document2->end_implicit) {
        return 0;
    }

    if ((document1->version_directive && !document2->version_directive)
            || (!document1->version_directive && document2->version_directive)
            || (document1->version_directive && document2->version_directive
                && (document1->version_directive->major != document2->version_directive->major
                    || document1->version_directive->minor != document2->version_directive->minor))) {
        return 0;
    }

    if ((document1->tag_directives.end - document1->tag_directives.start) !=
            (document2->tag_directives.end - document2->tag_directives.start)) {
        return 0;
    }

    for (k = 0; k < (document1->tag_directives.end - document1->tag_directives.start); k++) {
        if (strcmp((char *)document1->tag_directives.start[k].handle,
                    (char *)document2->tag_directives.start[k].handle) != 0
                || strcmp((char *)document1->tag_directives.start[k].prefix,
                    (char *)document2->tag_directives.start[k].prefix) != 0) {
            return 0;
        }
    }

    if ((document1->nodes.top - document1->nodes.start) !=
            (document2->nodes.top - document2->nodes.start)) {
        return 0;
    }

    if (document1->nodes.top != document1->nodes.start) {
        return compare_nodes(document1, 1, document2, 1, 0);
    }

    return 1;
}

static void
build_scalar_document(yaml_document_t *document, const yaml_char_t *value)
{
    int root;

    assert(yaml_document_initialize(document, NULL, NULL, NULL, 1, 1));
    root = yaml_document_add_scalar(document, (yaml_char_t *)YAML_STR_TAG,
            value, -1, YAML_PLAIN_SCALAR_STYLE);
    assert(root == 1);
    assert(yaml_document_get_root_node(document)
            == yaml_document_get_node(document, root));
}

static void
build_roundtrip_document(yaml_document_t *document)
{
    yaml_version_directive_t version = { 1, 1 };
    yaml_tag_directive_t tags[] = {
        { (yaml_char_t *)"!e!", (yaml_char_t *)EXAMPLE_TAG_PREFIX }
    };
    int root;
    int message_key;
    int message_value;
    int items_key;
    int items_value;
    int first_item;
    int second_item;
    int meta_key;
    int meta_value;
    int count_key;
    int count_value;

    assert(yaml_document_initialize(document, &version, tags, tags + 1, 0, 0));

    root = yaml_document_add_mapping(document, (yaml_char_t *)YAML_MAP_TAG,
            YAML_BLOCK_MAPPING_STYLE);
    assert(root == 1);
    assert(yaml_document_get_root_node(document)
            == yaml_document_get_node(document, root));
    assert(yaml_document_get_node(document, 0) == NULL);

    message_key = yaml_document_add_scalar(document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"message", -1, YAML_PLAIN_SCALAR_STYLE);
    message_value = yaml_document_add_scalar(document,
            (yaml_char_t *)EXAMPLE_TEXT_TAG,
            (yaml_char_t *)utf8_greeting, -1, YAML_PLAIN_SCALAR_STYLE);
    assert(message_key);
    assert(message_value);
    assert(yaml_document_append_mapping_pair(document, root,
                message_key, message_value));

    items_key = yaml_document_add_scalar(document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"items", -1, YAML_PLAIN_SCALAR_STYLE);
    items_value = yaml_document_add_sequence(document, (yaml_char_t *)YAML_SEQ_TAG,
            YAML_BLOCK_SEQUENCE_STYLE);
    first_item = yaml_document_add_scalar(document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"one", -1, YAML_PLAIN_SCALAR_STYLE);
    second_item = yaml_document_add_scalar(document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"two", -1, YAML_PLAIN_SCALAR_STYLE);
    assert(items_key);
    assert(items_value);
    assert(first_item);
    assert(second_item);
    assert(yaml_document_append_sequence_item(document, items_value, first_item));
    assert(yaml_document_append_sequence_item(document, items_value, second_item));
    assert(yaml_document_append_mapping_pair(document, root,
                items_key, items_value));

    meta_key = yaml_document_add_scalar(document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"meta", -1, YAML_PLAIN_SCALAR_STYLE);
    meta_value = yaml_document_add_mapping(document, (yaml_char_t *)YAML_MAP_TAG,
            YAML_FLOW_MAPPING_STYLE);
    count_key = yaml_document_add_scalar(document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"count", -1, YAML_PLAIN_SCALAR_STYLE);
    count_value = yaml_document_add_scalar(document, (yaml_char_t *)YAML_INT_TAG,
            (yaml_char_t *)"2", -1, YAML_PLAIN_SCALAR_STYLE);
    assert(meta_key);
    assert(meta_value);
    assert(count_key);
    assert(count_value);
    assert(yaml_document_append_mapping_pair(document, meta_value,
                count_key, count_value));
    assert(yaml_document_append_mapping_pair(document, root,
                meta_key, meta_value));
}

static int
check_encoding_controls(void)
{
    yaml_parser_t parser;
    yaml_document_t document;
    yaml_node_t *root;
    yaml_emitter_t emitter;
    yaml_document_t emitted;
    unsigned char output[BUFFER_SIZE];
    size_t written = 0;

    printf("checking encoding controls...\n");

    assert(yaml_parser_initialize(&parser));
    yaml_parser_set_input_string(&parser, utf16le_greeting_no_bom,
            sizeof(utf16le_greeting_no_bom));
    yaml_parser_set_encoding(&parser, YAML_UTF16LE_ENCODING);
    assert(parser.encoding == YAML_UTF16LE_ENCODING);
    assert(yaml_parser_load(&parser, &document));

    root = yaml_document_get_root_node(&document);
    assert_scalar_node(root, (yaml_char_t *)YAML_STR_TAG, utf8_greeting);

    yaml_document_delete(&document);
    yaml_parser_delete(&parser);

    memset(output, 0, sizeof(output));
    build_scalar_document(&emitted, utf8_greeting);

    assert(yaml_emitter_initialize(&emitter));
    yaml_emitter_set_output_string(&emitter, output, sizeof(output), &written);
    yaml_emitter_set_encoding(&emitter, YAML_UTF16LE_ENCODING);
    assert(emitter.encoding == YAML_UTF16LE_ENCODING);
    assert(yaml_emitter_open(&emitter));
    assert(yaml_emitter_dump(&emitter, &emitted));
    assert(yaml_emitter_close(&emitter));
    assert(yaml_emitter_flush(&emitter));
    yaml_emitter_delete(&emitter);

    assert(written > 2);
    assert(output[0] == 0xff);
    assert(output[1] == 0xfe);

    assert(yaml_parser_initialize(&parser));
    yaml_parser_set_input_string(&parser, output, written);
    assert(yaml_parser_load(&parser, &document));

    root = yaml_document_get_root_node(&document);
    assert_scalar_node(root, (yaml_char_t *)YAML_STR_TAG, utf8_greeting);

    yaml_document_delete(&document);
    yaml_parser_delete(&parser);

    printf("checking encoding controls: ok\n");
    return 0;
}

static int
check_document_api_roundtrip(void)
{
    yaml_document_t emitted;
    yaml_document_t actual;
    yaml_document_t expected;
    yaml_document_t end;
    yaml_emitter_t emitter;
    yaml_parser_t parser;
    yaml_node_t *root;
    yaml_node_t *items;
    yaml_node_t *meta;
    unsigned char output[BUFFER_SIZE];
    size_t written = 0;
    memory_reader_t reader;

    printf("checking document api round-trip...\n");

    memset(output, 0, sizeof(output));
    build_roundtrip_document(&emitted);

    assert(yaml_emitter_initialize(&emitter));
    yaml_emitter_set_output_string(&emitter, output, sizeof(output), &written);
    yaml_emitter_set_canonical(&emitter, 0);
    yaml_emitter_set_indent(&emitter, 3);
    yaml_emitter_set_width(&emitter, 48);
    yaml_emitter_set_unicode(&emitter, 1);
    yaml_emitter_set_break(&emitter, YAML_CRLN_BREAK);
    assert(emitter.best_indent == 3);
    assert(emitter.best_width == 48);
    assert(emitter.unicode);
    assert(emitter.line_break == YAML_CRLN_BREAK);
    assert(yaml_emitter_open(&emitter));
    assert(yaml_emitter_dump(&emitter, &emitted));
    assert(yaml_emitter_close(&emitter));
    assert(yaml_emitter_flush(&emitter));
    yaml_emitter_delete(&emitter);

    assert(written < sizeof(output));
    output[written] = '\0';
    assert(buffer_contains(output, written, "%YAML 1.1\r\n"));
    assert(buffer_contains(output, written,
                "%TAG !e! tag:example.com,2026:\r\n"));
    assert(buffer_contains(output, written, "message: "));
    assert(buffer_contains(output, written, "meta:"));
    assert(buffer_contains(output, written, "count"));
    assert_crlf_line_breaks(output, written);

    assert(yaml_parser_initialize(&parser));
    reader.input = output;
    reader.size = written;
    reader.offset = 0;
    reader.chunk = 5;
    yaml_parser_set_input(&parser, memory_read_handler, &reader);
    assert(yaml_parser_load(&parser, &actual));
    assert(reader.offset != 0);

    build_roundtrip_document(&expected);
    assert(compare_documents(&expected, &actual));
    assert(yaml_document_get_node(&actual, 999) == NULL);

    root = yaml_document_get_root_node(&actual);
    items = lookup_mapping_value(&actual, root, "items");
    meta = lookup_mapping_value(&actual, root, "meta");
    assert(items);
    assert(items->type == YAML_SEQUENCE_NODE);
    assert((items->data.sequence.items.top - items->data.sequence.items.start) == 2);
    assert(meta);
    assert(meta->type == YAML_MAPPING_NODE);
    assert(lookup_mapping_value(&actual, root, "missing") == NULL);

    yaml_document_delete(&expected);
    yaml_document_delete(&actual);

    assert(yaml_parser_load(&parser, &end));
    assert(yaml_document_get_root_node(&end) == NULL);
    yaml_document_delete(&end);
    yaml_parser_delete(&parser);

    printf("checking document api round-trip: ok\n");
    return 0;
}

static void
emit_event(yaml_emitter_t *emitter, yaml_event_t *event)
{
    assert(yaml_emitter_emit(emitter, event));
}

static void
parse_event(yaml_parser_t *parser, yaml_event_t *event)
{
    assert(yaml_parser_parse(parser, event));
}

static int
check_event_api_roundtrip(void)
{
    yaml_emitter_t emitter;
    yaml_parser_t parser;
    yaml_event_t event;
    yaml_version_directive_t version = { 1, 2 };
    yaml_tag_directive_t tags[] = {
        { (yaml_char_t *)"!e!", (yaml_char_t *)EXAMPLE_TAG_PREFIX }
    };
    unsigned char output[BUFFER_SIZE];
    memory_writer_t writer;

    printf("checking event api round-trip...\n");

    memset(output, 0, sizeof(output));
    writer.output = output;
    writer.capacity = sizeof(output);
    writer.written = 0;

    assert(yaml_emitter_initialize(&emitter));
    yaml_emitter_set_output(&emitter, memory_write_handler, &writer);
    yaml_emitter_set_width(&emitter, -1);
    yaml_emitter_set_break(&emitter, YAML_LN_BREAK);
    yaml_emitter_set_unicode(&emitter, 1);
    assert(emitter.best_width == -1);
    assert(emitter.line_break == YAML_LN_BREAK);
    assert(emitter.unicode);

    assert(yaml_stream_start_event_initialize(&event, YAML_UTF8_ENCODING));
    emit_event(&emitter, &event);

    assert(yaml_document_start_event_initialize(&event, &version, tags, tags + 1, 0));
    emit_event(&emitter, &event);

    assert(yaml_mapping_start_event_initialize(&event,
                (yaml_char_t *)"root", (yaml_char_t *)YAML_MAP_TAG, 1,
                YAML_BLOCK_MAPPING_STYLE));
    emit_event(&emitter, &event);

    assert(yaml_scalar_event_initialize(&event, NULL, NULL,
                (yaml_char_t *)"shared", -1, 1, 1, YAML_PLAIN_SCALAR_STYLE));
    emit_event(&emitter, &event);

    assert(yaml_scalar_event_initialize(&event, (yaml_char_t *)"item",
                (yaml_char_t *)EXAMPLE_TEXT_TAG, (yaml_char_t *)"value", -1,
                0, 0, YAML_DOUBLE_QUOTED_SCALAR_STYLE));
    emit_event(&emitter, &event);

    assert(yaml_scalar_event_initialize(&event, NULL, NULL,
                (yaml_char_t *)"alias", -1, 1, 1, YAML_PLAIN_SCALAR_STYLE));
    emit_event(&emitter, &event);

    assert(yaml_alias_event_initialize(&event, (yaml_char_t *)"item"));
    emit_event(&emitter, &event);

    assert(yaml_scalar_event_initialize(&event, NULL, NULL,
                (yaml_char_t *)"seq", -1, 1, 1, YAML_PLAIN_SCALAR_STYLE));
    emit_event(&emitter, &event);

    assert(yaml_sequence_start_event_initialize(&event, NULL,
                (yaml_char_t *)YAML_SEQ_TAG, 1, YAML_FLOW_SEQUENCE_STYLE));
    emit_event(&emitter, &event);

    assert(yaml_scalar_event_initialize(&event, NULL, NULL,
                (yaml_char_t *)"a", -1, 1, 1, YAML_PLAIN_SCALAR_STYLE));
    emit_event(&emitter, &event);

    assert(yaml_scalar_event_initialize(&event, NULL, NULL,
                (yaml_char_t *)"b", -1, 1, 1, YAML_PLAIN_SCALAR_STYLE));
    emit_event(&emitter, &event);

    assert(yaml_sequence_end_event_initialize(&event));
    emit_event(&emitter, &event);

    assert(yaml_mapping_end_event_initialize(&event));
    emit_event(&emitter, &event);

    assert(yaml_document_end_event_initialize(&event, 0));
    emit_event(&emitter, &event);

    assert(yaml_stream_end_event_initialize(&event));
    emit_event(&emitter, &event);

    assert(yaml_emitter_flush(&emitter));
    yaml_emitter_delete(&emitter);

    assert(writer.written > 0);
    assert(buffer_contains(output, writer.written, "%YAML 1.2\n"));
    assert(buffer_contains(output, writer.written,
                "%TAG !e! tag:example.com,2026:\n"));
    assert(buffer_contains(output, writer.written, "&item"));
    assert(buffer_contains(output, writer.written, "*item"));
    assert(buffer_contains(output, writer.written, "[a, b]"));

    assert(yaml_parser_initialize(&parser));
    yaml_parser_set_input_string(&parser, output, writer.written);

    parse_event(&parser, &event);
    assert(event.type == YAML_STREAM_START_EVENT);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_DOCUMENT_START_EVENT);
    assert(event.data.document_start.version_directive);
    assert(event.data.document_start.version_directive->major == 1);
    assert(event.data.document_start.version_directive->minor == 2);
    assert((event.data.document_start.tag_directives.end
                - event.data.document_start.tag_directives.start) == 1);
    assert(strcmp((char *)event.data.document_start.tag_directives.start[0].handle,
                "!e!") == 0);
    assert(strcmp((char *)event.data.document_start.tag_directives.start[0].prefix,
                EXAMPLE_TAG_PREFIX) == 0);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_MAPPING_START_EVENT);
    assert(strcmp((char *)event.data.mapping_start.anchor, "root") == 0);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_SCALAR_EVENT);
    assert(memcmp(event.data.scalar.value, "shared",
                event.data.scalar.length) == 0);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_SCALAR_EVENT);
    assert(strcmp((char *)event.data.scalar.anchor, "item") == 0);
    assert(strcmp((char *)event.data.scalar.tag, EXAMPLE_TEXT_TAG) == 0);
    assert(memcmp(event.data.scalar.value, "value",
                event.data.scalar.length) == 0);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_SCALAR_EVENT);
    assert(memcmp(event.data.scalar.value, "alias",
                event.data.scalar.length) == 0);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_ALIAS_EVENT);
    assert(strcmp((char *)event.data.alias.anchor, "item") == 0);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_SCALAR_EVENT);
    assert(memcmp(event.data.scalar.value, "seq",
                event.data.scalar.length) == 0);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_SEQUENCE_START_EVENT);
    assert(event.data.sequence_start.style == YAML_FLOW_SEQUENCE_STYLE);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_SCALAR_EVENT);
    assert(memcmp(event.data.scalar.value, "a",
                event.data.scalar.length) == 0);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_SCALAR_EVENT);
    assert(memcmp(event.data.scalar.value, "b",
                event.data.scalar.length) == 0);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_SEQUENCE_END_EVENT);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_MAPPING_END_EVENT);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_DOCUMENT_END_EVENT);
    yaml_event_delete(&event);

    parse_event(&parser, &event);
    assert(event.type == YAML_STREAM_END_EVENT);
    yaml_event_delete(&event);

    yaml_parser_delete(&parser);

    printf("checking event api round-trip: ok\n");
    return 0;
}

int
main(void)
{
    return check_encoding_controls()
         + check_document_api_roundtrip()
         + check_event_api_roundtrip();
}
