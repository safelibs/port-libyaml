#include <yaml.h>

#include <assert.h>
#include <stddef.h>
#include <string.h>

#define EXAMPLE_TAG_PREFIX "tag:example.com,2026:"
#define EXAMPLE_TEXT_TAG EXAMPLE_TAG_PREFIX "text"

typedef struct {
    const unsigned char *input;
    size_t size;
    size_t offset;
    size_t chunk;
} memory_reader_t;

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

static void
assert_zeroed_document(const yaml_document_t *document)
{
    const unsigned char *bytes = (const unsigned char *)document;
    size_t k;

    for (k = 0; k < sizeof(*document); k++) {
        assert(bytes[k] == 0);
    }
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

static void
test_document_mutators(void)
{
    yaml_document_t document;
    yaml_version_directive_t version = { 1, 1 };
    yaml_char_t handle[] = "!e!";
    yaml_char_t prefix[] = EXAMPLE_TAG_PREFIX;
    yaml_tag_directive_t tags[] = {
        { handle, prefix }
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
    yaml_node_t *node;

    assert(yaml_document_initialize(&document, &version, tags, tags + 1, 0, 0));
    assert(document.version_directive != &version);
    assert(document.version_directive->major == 1);
    assert(document.version_directive->minor == 1);
    assert((document.tag_directives.end - document.tag_directives.start) == 1);
    assert(document.tag_directives.start != tags);
    assert(document.tag_directives.start[0].handle != handle);
    assert(document.tag_directives.start[0].prefix != prefix);

    version.major = 9;
    handle[0] = 'X';
    prefix[0] = 'X';
    assert(document.version_directive->major == 1);
    assert(strcmp((char *)document.tag_directives.start[0].handle, "!e!") == 0);
    assert(strcmp((char *)document.tag_directives.start[0].prefix,
                EXAMPLE_TAG_PREFIX) == 0);

    root = yaml_document_add_mapping(&document, (yaml_char_t *)YAML_MAP_TAG,
            YAML_BLOCK_MAPPING_STYLE);
    message_key = yaml_document_add_scalar(&document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"message", -1, YAML_PLAIN_SCALAR_STYLE);
    message_value = yaml_document_add_scalar(&document,
            (yaml_char_t *)EXAMPLE_TEXT_TAG,
            (yaml_char_t *)"value", -1, YAML_DOUBLE_QUOTED_SCALAR_STYLE);
    items_key = yaml_document_add_scalar(&document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"items", -1, YAML_PLAIN_SCALAR_STYLE);
    items_value = yaml_document_add_sequence(&document, (yaml_char_t *)YAML_SEQ_TAG,
            YAML_BLOCK_SEQUENCE_STYLE);
    first_item = yaml_document_add_scalar(&document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"one", -1, YAML_PLAIN_SCALAR_STYLE);
    second_item = yaml_document_add_scalar(&document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"two", -1, YAML_PLAIN_SCALAR_STYLE);
    meta_key = yaml_document_add_scalar(&document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"meta", -1, YAML_PLAIN_SCALAR_STYLE);
    meta_value = yaml_document_add_mapping(&document, (yaml_char_t *)YAML_MAP_TAG,
            YAML_FLOW_MAPPING_STYLE);
    count_key = yaml_document_add_scalar(&document, (yaml_char_t *)YAML_STR_TAG,
            (yaml_char_t *)"count", -1, YAML_PLAIN_SCALAR_STYLE);
    count_value = yaml_document_add_scalar(&document, (yaml_char_t *)YAML_INT_TAG,
            (yaml_char_t *)"2", -1, YAML_PLAIN_SCALAR_STYLE);

    assert(root == 1);
    assert(message_key == 2);
    assert(message_value == 3);
    assert(items_key == 4);
    assert(items_value == 5);
    assert(first_item == 6);
    assert(second_item == 7);
    assert(meta_key == 8);
    assert(meta_value == 9);
    assert(count_key == 10);
    assert(count_value == 11);

    assert(yaml_document_get_node(&document, 0) == NULL);
    assert(yaml_document_get_root_node(&document)
            == yaml_document_get_node(&document, root));

    assert(yaml_document_append_mapping_pair(&document, root,
                message_key, message_value));
    assert(yaml_document_append_sequence_item(&document, items_value, first_item));
    assert(yaml_document_append_sequence_item(&document, items_value, second_item));
    assert(yaml_document_append_mapping_pair(&document, root,
                items_key, items_value));
    assert(yaml_document_append_mapping_pair(&document, meta_value,
                count_key, count_value));
    assert(yaml_document_append_mapping_pair(&document, root,
                meta_key, meta_value));

    node = yaml_document_get_node(&document, message_value);
    assert(node);
    assert(node->type == YAML_SCALAR_NODE);
    assert(strcmp((char *)node->tag, EXAMPLE_TEXT_TAG) == 0);
    assert(memcmp(node->data.scalar.value, "value", 5) == 0);

    node = lookup_mapping_value(&document, yaml_document_get_root_node(&document), "items");
    assert(node);
    assert(node->type == YAML_SEQUENCE_NODE);
    assert((node->data.sequence.items.top - node->data.sequence.items.start) == 2);

    node = lookup_mapping_value(&document, yaml_document_get_root_node(&document), "meta");
    assert(node);
    assert(node->type == YAML_MAPPING_NODE);
    assert((node->data.mapping.pairs.top - node->data.mapping.pairs.start) == 1);

    yaml_document_delete(&document);
    assert_zeroed_document(&document);
}

static void
test_parser_load_with_chunked_reader(void)
{
    static const unsigned char input[] =
        "%YAML 1.1\n"
        "%TAG !e! " EXAMPLE_TAG_PREFIX "\n"
        "---\n"
        "message: &item !e!text \"value\"\n"
        "alias: *item\n"
        "seq: [one, two]\n"
        "...\n";
    memory_reader_t reader = { input, sizeof(input)-1, 0, 3 };
    yaml_parser_t parser;
    yaml_document_t document;
    yaml_document_t end;
    yaml_node_t *root;
    yaml_node_t *message;
    yaml_node_t *alias;
    yaml_node_t *seq;

    assert(yaml_parser_initialize(&parser));
    yaml_parser_set_input(&parser, memory_read_handler, &reader);

    assert(yaml_parser_load(&parser, &document));
    assert(reader.offset != 0);
    assert(document.version_directive);
    assert(document.version_directive->major == 1);
    assert(document.version_directive->minor == 1);
    assert((document.tag_directives.end - document.tag_directives.start) == 1);

    root = yaml_document_get_root_node(&document);
    assert(root);
    assert(root->type == YAML_MAPPING_NODE);

    message = lookup_mapping_value(&document, root, "message");
    alias = lookup_mapping_value(&document, root, "alias");
    seq = lookup_mapping_value(&document, root, "seq");
    assert(message);
    assert(alias == message);
    assert(seq);
    assert(seq->type == YAML_SEQUENCE_NODE);
    assert((seq->data.sequence.items.top - seq->data.sequence.items.start) == 2);

    yaml_document_delete(&document);

    assert(yaml_parser_load(&parser, &end));
    assert(yaml_document_get_root_node(&end) == NULL);
    yaml_document_delete(&end);
    yaml_parser_delete(&parser);
}

static void
test_empty_document_root_behavior(void)
{
    static const unsigned char input[] =
        "---\n"
        "...\n"
        "---\n"
        "answer: 42\n";
    yaml_parser_t parser;
    yaml_document_t empty;
    yaml_document_t document;
    yaml_document_t end;
    yaml_node_t *root;
    yaml_node_t *answer;

    assert(yaml_parser_initialize(&parser));
    yaml_parser_set_input_string(&parser, input, sizeof(input)-1);

    assert(yaml_parser_load(&parser, &empty));
    root = yaml_document_get_root_node(&empty);
    assert(root);
    assert(root->type == YAML_SCALAR_NODE);
    assert(root->data.scalar.length == 0);
    yaml_document_delete(&empty);

    assert(yaml_parser_load(&parser, &document));
    root = yaml_document_get_root_node(&document);
    assert(root);
    answer = lookup_mapping_value(&document, root, "answer");
    assert(answer);
    assert(answer->type == YAML_SCALAR_NODE);
    assert(answer->data.scalar.length == 2);
    assert(memcmp(answer->data.scalar.value, "42", 2) == 0);
    yaml_document_delete(&document);

    assert(yaml_parser_load(&parser, &end));
    assert(yaml_document_get_root_node(&end) == NULL);
    yaml_document_delete(&end);
    yaml_parser_delete(&parser);
}

int
main(void)
{
    test_document_mutators();
    test_parser_load_with_chunked_reader();
    test_empty_document_root_behavior();
    return 0;
}
