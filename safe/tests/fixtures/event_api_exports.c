#include <yaml.h>

#include <assert.h>
#include <stddef.h>
#include <string.h>

static void
assert_zeroed_event(const yaml_event_t *event)
{
    const unsigned char *bytes = (const unsigned char *)event;
    size_t k;

    for (k = 0; k < sizeof(*event); k++) {
        assert(bytes[k] == 0);
    }
}

static void
test_stream_events(void)
{
    yaml_event_t event;

    assert(yaml_stream_start_event_initialize(&event, YAML_UTF8_ENCODING));
    assert(event.type == YAML_STREAM_START_EVENT);
    assert(event.data.stream_start.encoding == YAML_UTF8_ENCODING);
    yaml_event_delete(&event);
    assert_zeroed_event(&event);

    assert(yaml_stream_end_event_initialize(&event));
    assert(event.type == YAML_STREAM_END_EVENT);
    yaml_event_delete(&event);
    assert_zeroed_event(&event);
}

static void
test_document_events(void)
{
    yaml_event_t event;
    yaml_version_directive_t version = { 1, 2 };
    yaml_char_t handle[] = "!e!";
    yaml_char_t prefix[] = "tag:example.com,2026:";
    yaml_tag_directive_t tags[] = {
        { handle, prefix }
    };

    assert(yaml_document_start_event_initialize(&event, &version, tags, tags + 1, 0));
    assert(event.type == YAML_DOCUMENT_START_EVENT);
    assert(event.data.document_start.implicit == 0);
    assert(event.data.document_start.version_directive != &version);
    assert(event.data.document_start.version_directive->major == 1);
    assert(event.data.document_start.version_directive->minor == 2);
    assert((event.data.document_start.tag_directives.end
                - event.data.document_start.tag_directives.start) == 1);
    assert(event.data.document_start.tag_directives.start != tags);
    assert(event.data.document_start.tag_directives.start[0].handle != handle);
    assert(event.data.document_start.tag_directives.start[0].prefix != prefix);
    version.major = 9;
    handle[0] = '?';
    prefix[0] = 'X';
    assert(event.data.document_start.version_directive->major == 1);
    assert(strcmp((char *)event.data.document_start.tag_directives.start[0].handle,
                "!e!") == 0);
    assert(strcmp((char *)event.data.document_start.tag_directives.start[0].prefix,
                "tag:example.com,2026:") == 0);
    yaml_event_delete(&event);
    assert_zeroed_event(&event);

    assert(yaml_document_end_event_initialize(&event, 0));
    assert(event.type == YAML_DOCUMENT_END_EVENT);
    assert(event.data.document_end.implicit == 0);
    yaml_event_delete(&event);
    assert_zeroed_event(&event);
}

static void
test_alias_and_scalar_events(void)
{
    yaml_event_t event;
    yaml_char_t alias_anchor[] = "item";
    yaml_char_t scalar_anchor[] = "item";
    yaml_char_t scalar_tag[] = "tag:example.com,2026:text";
    yaml_char_t scalar_value[] = "value";

    assert(yaml_alias_event_initialize(&event, alias_anchor));
    assert(event.type == YAML_ALIAS_EVENT);
    assert(event.data.alias.anchor != alias_anchor);
    alias_anchor[0] = 'X';
    assert(strcmp((char *)event.data.alias.anchor, "item") == 0);
    yaml_event_delete(&event);
    assert_zeroed_event(&event);

    assert(yaml_scalar_event_initialize(&event,
                scalar_anchor, scalar_tag, scalar_value, 5,
                0, 0, YAML_DOUBLE_QUOTED_SCALAR_STYLE));
    assert(event.type == YAML_SCALAR_EVENT);
    assert(event.data.scalar.anchor != scalar_anchor);
    assert(event.data.scalar.tag != scalar_tag);
    assert(event.data.scalar.value != scalar_value);
    assert(event.data.scalar.length == 5);
    assert(event.data.scalar.style == YAML_DOUBLE_QUOTED_SCALAR_STYLE);
    assert(event.data.scalar.plain_implicit == 0);
    assert(event.data.scalar.quoted_implicit == 0);
    scalar_anchor[0] = 'X';
    scalar_tag[0] = 'X';
    scalar_value[0] = 'X';
    assert(strcmp((char *)event.data.scalar.anchor, "item") == 0);
    assert(strcmp((char *)event.data.scalar.tag,
                "tag:example.com,2026:text") == 0);
    assert(memcmp(event.data.scalar.value, "value", 5) == 0);
    yaml_event_delete(&event);
    assert_zeroed_event(&event);
}

static void
test_collection_events(void)
{
    yaml_event_t event;
    yaml_char_t seq_anchor[] = "seq";
    yaml_char_t seq_tag[] = "tag:yaml.org,2002:seq";
    yaml_char_t map_anchor[] = "map";
    yaml_char_t map_tag[] = "tag:yaml.org,2002:map";

    assert(yaml_sequence_start_event_initialize(&event,
                seq_anchor, seq_tag, 1, YAML_FLOW_SEQUENCE_STYLE));
    assert(event.type == YAML_SEQUENCE_START_EVENT);
    assert(event.data.sequence_start.anchor != seq_anchor);
    assert(event.data.sequence_start.tag != seq_tag);
    assert(event.data.sequence_start.implicit == 1);
    assert(event.data.sequence_start.style == YAML_FLOW_SEQUENCE_STYLE);
    seq_anchor[0] = 'X';
    seq_tag[0] = 'X';
    assert(strcmp((char *)event.data.sequence_start.anchor, "seq") == 0);
    assert(strcmp((char *)event.data.sequence_start.tag,
                "tag:yaml.org,2002:seq") == 0);
    yaml_event_delete(&event);
    assert_zeroed_event(&event);

    assert(yaml_sequence_end_event_initialize(&event));
    assert(event.type == YAML_SEQUENCE_END_EVENT);
    yaml_event_delete(&event);
    assert_zeroed_event(&event);

    assert(yaml_mapping_start_event_initialize(&event,
                map_anchor, map_tag, 1, YAML_BLOCK_MAPPING_STYLE));
    assert(event.type == YAML_MAPPING_START_EVENT);
    assert(event.data.mapping_start.anchor != map_anchor);
    assert(event.data.mapping_start.tag != map_tag);
    assert(event.data.mapping_start.implicit == 1);
    assert(event.data.mapping_start.style == YAML_BLOCK_MAPPING_STYLE);
    map_anchor[0] = 'X';
    map_tag[0] = 'X';
    assert(strcmp((char *)event.data.mapping_start.anchor, "map") == 0);
    assert(strcmp((char *)event.data.mapping_start.tag,
                "tag:yaml.org,2002:map") == 0);
    yaml_event_delete(&event);
    assert_zeroed_event(&event);

    assert(yaml_mapping_end_event_initialize(&event));
    assert(event.type == YAML_MAPPING_END_EVENT);
    yaml_event_delete(&event);
    assert_zeroed_event(&event);
}

int
main(void)
{
    test_stream_events();
    test_document_events();
    test_alias_and_scalar_events();
    test_collection_events();
    return 0;
}
