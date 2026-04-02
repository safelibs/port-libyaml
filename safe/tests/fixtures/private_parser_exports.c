#include <yaml.h>

#include <assert.h>
#include <stddef.h>
#include <string.h>

extern int yaml_parser_update_buffer(yaml_parser_t *parser, size_t length);
extern int yaml_parser_fetch_more_tokens(yaml_parser_t *parser);

#define MAX_FILE_SIZE (~(size_t)0 / 2)

static void
test_private_helper_exports(void)
{
    static const unsigned char input[] = "key: value\n";
    yaml_parser_t parser;

    assert(yaml_parser_initialize(&parser));
    yaml_parser_set_input_string(&parser, input, sizeof(input)-1);
    assert(yaml_parser_update_buffer(&parser, 1));
    assert(yaml_parser_fetch_more_tokens(&parser));
    assert(parser.token_available);
    assert(parser.tokens.head != parser.tokens.tail);
    assert(parser.tokens.head->type == YAML_STREAM_START_TOKEN);
    yaml_parser_delete(&parser);
}

static void
test_oversized_input_reader_error(void)
{
    static const unsigned char input[] = "a";
    yaml_parser_t parser;

    assert(yaml_parser_initialize(&parser));
    yaml_parser_set_input_string(&parser, input, sizeof(input)-1);
    parser.offset = MAX_FILE_SIZE;
    assert(!yaml_parser_update_buffer(&parser, 1));
    assert(parser.error == YAML_READER_ERROR);
    assert(parser.problem);
    assert(strcmp(parser.problem, "input is too long") == 0);
    assert(parser.problem_offset == MAX_FILE_SIZE);
    assert(parser.problem_value == -1);
    yaml_parser_delete(&parser);
}

int
main(void)
{
    test_private_helper_exports();
    test_oversized_input_reader_error();
    return 0;
}
