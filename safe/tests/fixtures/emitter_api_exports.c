#include <yaml.h>

#include <assert.h>
#include <stddef.h>
#include <string.h>

static const yaml_char_t utf8_greeting[] =
    "Hi is \xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82";

typedef struct {
    unsigned char *buffer;
    size_t capacity;
    size_t written;
} memory_writer_t;

static int
memory_write_handler(void *data, unsigned char *buffer, size_t size)
{
    memory_writer_t *writer = (memory_writer_t *)data;

    if (writer->written + size > writer->capacity) {
        return 0;
    }

    memcpy(writer->buffer + writer->written, buffer, size);
    writer->written += size;
    if (writer->written < writer->capacity) {
        writer->buffer[writer->written] = '\0';
    }
    return 1;
}

static int
emit_scalar_mapping_document(yaml_emitter_t *emitter, const yaml_char_t *value,
        size_t length)
{
    yaml_event_t event;

    if (!yaml_stream_start_event_initialize(&event, YAML_UTF8_ENCODING)
            || !yaml_emitter_emit(emitter, &event)) {
        return 0;
    }
    if (!yaml_document_start_event_initialize(&event, NULL, NULL, NULL, 0)
            || !yaml_emitter_emit(emitter, &event)) {
        return 0;
    }
    if (!yaml_mapping_start_event_initialize(&event, NULL, NULL, 1,
                YAML_BLOCK_MAPPING_STYLE)
            || !yaml_emitter_emit(emitter, &event)) {
        return 0;
    }
    if (!yaml_scalar_event_initialize(&event, NULL, NULL,
                (yaml_char_t *)"message", 7, 1, 1, YAML_PLAIN_SCALAR_STYLE)
            || !yaml_emitter_emit(emitter, &event)) {
        return 0;
    }
    if (!yaml_scalar_event_initialize(&event, NULL, NULL,
                (yaml_char_t *)value, (int)length, 1, 1, YAML_PLAIN_SCALAR_STYLE)
            || !yaml_emitter_emit(emitter, &event)) {
        return 0;
    }
    if (!yaml_mapping_end_event_initialize(&event)
            || !yaml_emitter_emit(emitter, &event)) {
        return 0;
    }
    if (!yaml_document_end_event_initialize(&event, 0)
            || !yaml_emitter_emit(emitter, &event)) {
        return 0;
    }
    if (!yaml_stream_end_event_initialize(&event)
            || !yaml_emitter_emit(emitter, &event)) {
        return 0;
    }

    return 1;
}

static void
assert_output_contains_scalar(const unsigned char *buffer, size_t size,
        const char *expected)
{
    yaml_parser_t parser;
    yaml_event_t event;
    int found = 0;

    assert(yaml_parser_initialize(&parser));
    yaml_parser_set_input_string(&parser, buffer, size);

    while (1) {
        assert(yaml_parser_parse(&parser, &event));
        if (event.type == YAML_SCALAR_EVENT
                && event.data.scalar.length == strlen(expected)
                && memcmp(event.data.scalar.value, expected,
                    event.data.scalar.length) == 0) {
            found = 1;
        }

        if (event.type == YAML_STREAM_END_EVENT) {
            yaml_event_delete(&event);
            break;
        }
        yaml_event_delete(&event);
    }

    yaml_parser_delete(&parser);
    assert(found);
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
test_output_string_and_setters(void)
{
    yaml_emitter_t emitter;
    unsigned char output[512];
    size_t written = 17;

    memset(&emitter, 0, sizeof(emitter));
    memset(output, 0, sizeof(output));

    assert(yaml_emitter_initialize(&emitter));
    yaml_emitter_set_output_string(&emitter, output, sizeof(output), &written);
    yaml_emitter_set_encoding(&emitter, YAML_UTF8_ENCODING);
    yaml_emitter_set_indent(&emitter, 1);
    assert(emitter.best_indent == 2);
    yaml_emitter_set_indent(&emitter, 4);
    assert(emitter.best_indent == 4);
    yaml_emitter_set_width(&emitter, -7);
    assert(emitter.best_width == -1);
    yaml_emitter_set_width(&emitter, 24);
    assert(emitter.best_width == 24);
    yaml_emitter_set_unicode(&emitter, 9);
    assert(emitter.unicode == 1);
    yaml_emitter_set_break(&emitter, YAML_CRLN_BREAK);
    assert(emitter.line_break == YAML_CRLN_BREAK);

    assert(emit_scalar_mapping_document(&emitter, (yaml_char_t *)"hello", 5));
    assert(yaml_emitter_flush(&emitter));
    yaml_emitter_delete(&emitter);

    assert(written > 0);
    assert(strstr((char *)output, "message: hello") != NULL);
    assert_crlf_line_breaks(output, written);
    assert_output_contains_scalar(output, written, "hello");
}

static void
test_callback_output_and_utf16_flush(void)
{
    yaml_emitter_t emitter;
    unsigned char output[512];
    memory_writer_t writer = { output, sizeof(output), 0 };

    memset(&emitter, 0, sizeof(emitter));
    memset(output, 0, sizeof(output));

    assert(yaml_emitter_initialize(&emitter));
    yaml_emitter_set_output(&emitter, memory_write_handler, &writer);
    yaml_emitter_set_encoding(&emitter, YAML_UTF16LE_ENCODING);
    yaml_emitter_set_width(&emitter, -11);
    assert(emitter.best_width == -1);
    yaml_emitter_set_break(&emitter, YAML_LN_BREAK);
    assert(emitter.line_break == YAML_LN_BREAK);
    yaml_emitter_set_unicode(&emitter, 1);
    assert(emitter.unicode == 1);

    assert(emit_scalar_mapping_document(&emitter, utf8_greeting,
                sizeof(utf8_greeting)-1));
    assert(yaml_emitter_flush(&emitter));
    yaml_emitter_delete(&emitter);

    assert(writer.written > 2);
    assert(output[0] == 0xFF);
    assert(output[1] == 0xFE);
    assert_output_contains_scalar(output, writer.written,
            "Hi is \xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82");
}

static void
test_writer_error_propagation(void)
{
    yaml_emitter_t emitter;
    unsigned char output[8];
    memory_writer_t writer = { output, sizeof(output), 0 };

    memset(&emitter, 0, sizeof(emitter));
    memset(output, 0, sizeof(output));

    assert(yaml_emitter_initialize(&emitter));
    yaml_emitter_set_output(&emitter, memory_write_handler, &writer);
    assert(!emit_scalar_mapping_document(&emitter, (yaml_char_t *)"hello", 5));
    assert(emitter.error == YAML_WRITER_ERROR);
    assert(strcmp(emitter.problem, "write error") == 0);
    yaml_emitter_delete(&emitter);
}

int
main(void)
{
    test_output_string_and_setters();
    test_callback_output_and_utf16_flush();
    test_writer_error_propagation();
    return 0;
}
