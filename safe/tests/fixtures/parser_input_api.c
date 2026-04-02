#include <yaml.h>

#include <assert.h>
#include <stddef.h>
#include <string.h>

extern int yaml_parser_update_buffer(yaml_parser_t *parser, size_t length);

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
test_string_input_decodes_utf8_bom(void)
{
    static const unsigned char input[] = "\xef\xbb\xbfplain";
    yaml_parser_t parser;

    assert(yaml_parser_initialize(&parser));
    yaml_parser_set_input_string(&parser, input, sizeof(input)-1);
    assert(yaml_parser_update_buffer(&parser, 6));
    assert(parser.encoding == YAML_UTF8_ENCODING);
    assert(parser.unread >= 6);
    assert(memcmp(parser.buffer.pointer, "plain", 5) == 0);
    assert(parser.buffer.pointer[5] == '\0');
    yaml_parser_delete(&parser);
}

static void
test_generic_input_with_explicit_utf16le_encoding(void)
{
    static const unsigned char input[] = {
        'H', 0x00,
        'i', 0x00,
        '!', 0x00
    };
    memory_reader_t reader = { input, sizeof(input), 0, 1 };
    yaml_parser_t parser;

    assert(yaml_parser_initialize(&parser));
    yaml_parser_set_input(&parser, memory_read_handler, &reader);
    yaml_parser_set_encoding(&parser, YAML_UTF16LE_ENCODING);
    assert(yaml_parser_update_buffer(&parser, 4));
    assert(parser.encoding == YAML_UTF16LE_ENCODING);
    assert(parser.unread >= 4);
    assert(memcmp(parser.buffer.pointer, "Hi!", 3) == 0);
    assert(parser.buffer.pointer[3] == '\0');
    yaml_parser_delete(&parser);
}

int
main(void)
{
    test_string_input_decodes_utf8_bom();
    test_generic_input_with_explicit_utf16le_encoding();
    return 0;
}
