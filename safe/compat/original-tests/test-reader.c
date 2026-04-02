#include <yaml.h>

#include <stdlib.h>
#include <stdio.h>

#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>

/*
 * The upstream reader test used yaml_parser_update_buffer() and direct access
 * to parser.buffer/parser.unread. Those internals are not part of libyaml's
 * public API, so this test now exercises reader behavior through
 * yaml_parser_load() and public document inspection instead.
 */

/*
 * Test cases are stolen from
 * http://www.cl.cam.ac.uk/~mgk25/ucs/examples/UTF-8-test.txt
 */

typedef struct {
    const char *title;
    const char *test;
    int result;
} test_case;

static test_case utf8_sequences[] = {
    /* {"title", "test 1|test 2|...|test N!", (0 or 1)}, */

    {"a simple test", "'test' is '\xd0\xbf\xd1\x80\xd0\xbe\xd0\xb2\xd0\xb5\xd1\x80\xd0\xba\xd0\xb0' in Russian!", 1},
    {"an empty line", "!", 1},

    {"u-0 is a control character", "\x00!", 0},
    {"u-80 is a control character", "\xc2\x80!", 0},
    {"u-800 is valid", "\xe0\xa0\x80!", 1},
    {"u-10000 is valid", "\xf0\x90\x80\x80!", 1},
    {"5 bytes sequences are not allowed", "\xf8\x88\x80\x80\x80!", 0},
    {"6 bytes sequences are not allowed", "\xfc\x84\x80\x80\x80\x80!", 0},

    {"u-7f is a control character", "\x7f!", 0},
    {"u-7FF is valid", "\xdf\xbf!", 1},
    {"u-FFFF is a control character", "\xef\xbf\xbf!", 0},
    {"u-1FFFFF is too large", "\xf7\xbf\xbf\xbf!", 0},
    {"u-3FFFFFF is 5 bytes", "\xfb\xbf\xbf\xbf\xbf!", 0},
    {"u-7FFFFFFF is 6 bytes", "\xfd\xbf\xbf\xbf\xbf\xbf!", 0},

    {"u-D7FF", "\xed\x9f\xbf!", 1},
    {"u-E000", "\xee\x80\x80!", 1},
    {"u-FFFD", "\xef\xbf\xbd!", 1},
    {"u-10FFFF", "\xf4\x8f\xbf\xbf!", 1},
    {"u-110000", "\xf4\x90\x80\x80!", 0},

    {"first continuation byte", "\x80!", 0},
    {"last continuation byte", "\xbf!", 0},

    {"2 continuation bytes", "\x80\xbf!", 0},
    {"3 continuation bytes", "\x80\xbf\x80!", 0},
    {"4 continuation bytes", "\x80\xbf\x80\xbf!", 0},
    {"5 continuation bytes", "\x80\xbf\x80\xbf\x80!", 0},
    {"6 continuation bytes", "\x80\xbf\x80\xbf\x80\xbf!", 0},
    {"7 continuation bytes", "\x80\xbf\x80\xbf\x80\xbf\x80!", 0},

    {"sequence of all 64 possible continuation bytes",
     "\x80|\x81|\x82|\x83|\x84|\x85|\x86|\x87|\x88|\x89|\x8a|\x8b|\x8c|\x8d|\x8e|\x8f|"
     "\x90|\x91|\x92|\x93|\x94|\x95|\x96|\x97|\x98|\x99|\x9a|\x9b|\x9c|\x9d|\x9e|\x9f|"
     "\xa0|\xa1|\xa2|\xa3|\xa4|\xa5|\xa6|\xa7|\xa8|\xa9|\xaa|\xab|\xac|\xad|\xae|\xaf|"
     "\xb0|\xb1|\xb2|\xb3|\xb4|\xb5|\xb6|\xb7|\xb8|\xb9|\xba|\xbb|\xbc|\xbd|\xbe|\xbf!", 0},
    {"32 first bytes of 2-byte sequences {0xc0-0xdf}",
     "\xc0 |\xc1 |\xc2 |\xc3 |\xc4 |\xc5 |\xc6 |\xc7 |\xc8 |\xc9 |\xca |\xcb |\xcc |\xcd |\xce |\xcf |"
     "\xd0 |\xd1 |\xd2 |\xd3 |\xd4 |\xd5 |\xd6 |\xd7 |\xd8 |\xd9 |\xda |\xdb |\xdc |\xdd |\xde |\xdf !", 0},
    {"16 first bytes of 3-byte sequences {0xe0-0xef}",
     "\xe0 |\xe1 |\xe2 |\xe3 |\xe4 |\xe5 |\xe6 |\xe7 |\xe8 |\xe9 |\xea |\xeb |\xec |\xed |\xee |\xef !", 0},
    {"8 first bytes of 4-byte sequences {0xf0-0xf7}", "\xf0 |\xf1 |\xf2 |\xf3 |\xf4 |\xf5 |\xf6 |\xf7 !", 0},
    {"4 first bytes of 5-byte sequences {0xf8-0xfb}", "\xf8 |\xf9 |\xfa |\xfb !", 0},
    {"2 first bytes of 6-byte sequences {0xfc-0xfd}", "\xfc |\xfd !", 0},

    {"sequences with last byte missing {u-0}",
     "\xc0|\xe0\x80|\xf0\x80\x80|\xf8\x80\x80\x80|\xfc\x80\x80\x80\x80!", 0},
    {"sequences with last byte missing {u-...FF}",
     "\xdf|\xef\xbf|\xf7\xbf\xbf|\xfb\xbf\xbf\xbf|\xfd\xbf\xbf\xbf\xbf!", 0},

    {"impossible bytes", "\xfe|\xff|\xfe\xfe\xff\xff!", 0},

    {"overlong sequences {u-2f}",
     "\xc0\xaf|\xe0\x80\xaf|\xf0\x80\x80\xaf|\xf8\x80\x80\x80\xaf|\xfc\x80\x80\x80\x80\xaf!", 0},

    {"maximum overlong sequences",
     "\xc1\xbf|\xe0\x9f\xbf|\xf0\x8f\xbf\xbf|\xf8\x87\xbf\xbf\xbf|\xfc\x83\xbf\xbf\xbf\xbf!", 0},

    {"overlong representation of the NUL character",
     "\xc0\x80|\xe0\x80\x80|\xf0\x80\x80\x80|\xf8\x80\x80\x80\x80|\xfc\x80\x80\x80\x80\x80!", 0},

    {"single UTF-16 surrogates",
     "\xed\xa0\x80|\xed\xad\xbf|\xed\xae\x80|\xed\xaf\xbf|\xed\xb0\x80|\xed\xbe\x80|\xed\xbf\xbf!", 0},

    {"paired UTF-16 surrogates",
     "\xed\xa0\x80\xed\xb0\x80|\xed\xa0\x80\xed\xbf\xbf|\xed\xad\xbf\xed\xb0\x80|"
     "\xed\xad\xbf\xed\xbf\xbf|\xed\xae\x80\xed\xb0\x80|\xed\xae\x80\xed\xbf\xbf|"
     "\xed\xaf\xbf\xed\xb0\x80|\xed\xaf\xbf\xed\xbf\xbf!", 0},

    {"other illegal code positions", "\xef\xbf\xbe|\xef\xbf\xbf!", 0},

    {NULL, NULL, 0}
};

static test_case boms[] = {
    /* {"title", "test!", length}, */

    {"no bom (utf-8)", "Hi is \xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82!", 13},
    {"bom (utf-8)", "\xef\xbb\xbfHi is \xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82!", 13},
    {"bom (utf-16-le)", "\xff\xfeH\x00i\x00 \x00i\x00s\x00 \x00\x1f\x04@\x04""8\x04""2\x04""5\x04""B\x04!", 13},
    {"bom (utf-16-be)", "\xfe\xff\x00H\x00i\x00 \x00i\x00s\x00 \x04\x1f\x04@\x04""8\x04""2\x04""5\x04""B!", 13},
    {NULL, NULL, 0}
};

static const unsigned char bom_original[] =
    "Hi is \xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82";

static void
print_parser_error(yaml_parser_t *parser)
{
    if (!parser->error) {
        printf("(no error)\n");
        return;
    }

    if (parser->error == YAML_READER_ERROR) {
        if (parser->problem_value != -1) {
            printf("(reader error: %s: #%X at %ld)\n",
                    parser->problem, parser->problem_value,
                    (long)parser->problem_offset);
        }
        else {
            printf("(reader error: %s at %ld)\n",
                    parser->problem, (long)parser->problem_offset);
        }
        return;
    }

    if (parser->problem) {
        printf("(parser error: %s at line %ld, column %ld)\n",
                parser->problem,
                (long)parser->problem_mark.line + 1,
                (long)parser->problem_mark.column + 1);
    }
    else {
        printf("(parser error type %d)\n", parser->error);
    }
}

static int
load_document(const unsigned char *input, size_t length, yaml_parser_t *parser,
        yaml_document_t *document)
{
    assert(yaml_parser_initialize(parser));
    yaml_parser_set_input_string(parser, input, length);
    return yaml_parser_load(parser, document);
}

static int
check_utf8_segment(const unsigned char *input, size_t length, int expect_success)
{
    yaml_parser_t parser;
    yaml_document_t document;
    yaml_node_t *root = NULL;
    int ok;
    int failed = 0;

    ok = load_document(input, length, &parser, &document);

    if (expect_success) {
        if (!ok) {
            failed = 1;
        }
        else {
            root = yaml_document_get_root_node(&document);
            if (!length) {
                if (root) {
                    failed = 1;
                    printf("\t\t- (expected an empty stream)\n");
                }
            }
            else if (!root || root->type != YAML_SCALAR_NODE) {
                failed = 1;
                printf("\t\t- (expected a scalar document)\n");
            }
        }
    }
    else {
        if (ok) {
            failed = 1;
            printf("\t\t- (expected a reader error, but the input loaded successfully)\n");
        }
        else if (parser.error != YAML_READER_ERROR) {
            failed = 1;
            printf("\t\t- (expected a reader error, got error type %d)\n",
                    parser.error);
        }
    }

    if (!failed) {
        printf("\t\t+ ");
        print_parser_error(&parser);
    }

    if (ok) {
        yaml_document_delete(&document);
    }
    yaml_parser_delete(&parser);

    return failed;
}

static int
check_scalar_case(const char *title, const unsigned char *input,
        size_t input_length, const unsigned char *expected_value,
        size_t expected_length)
{
    yaml_parser_t parser;
    yaml_document_t document;
    yaml_node_t *root = NULL;
    int ok;
    int failed = 0;

    printf("\t%s: ", title);

    ok = load_document(input, input_length, &parser, &document);
    if (!ok) {
        printf("- ");
        print_parser_error(&parser);
        yaml_parser_delete(&parser);
        return 1;
    }

    root = yaml_document_get_root_node(&document);
    if (!root || root->type != YAML_SCALAR_NODE) {
        printf("- (expected a scalar document)\n");
        failed = 1;
    }
    else if (root->data.scalar.length != expected_length) {
        printf("- (length=%ld while expected length=%ld)\n",
                (long)root->data.scalar.length, (long)expected_length);
        failed = 1;
    }
    else if (memcmp(root->data.scalar.value, expected_value, expected_length) != 0) {
        printf("- (decoded scalar does not match the expected value)\n");
        failed = 1;
    }
    else {
        printf("+\n");
    }

    yaml_document_delete(&document);
    yaml_parser_delete(&parser);

    return failed;
}

int
check_utf8_sequences(void)
{
    int failed = 0;
    int k;

    printf("checking utf-8 sequences...\n");
    for (k = 0; utf8_sequences[k].test; k++) {
        const char *title = utf8_sequences[k].title;
        int expect_success = utf8_sequences[k].result;
        const char *start = utf8_sequences[k].test;
        const char *end = start;

        printf("\t%s:\n", title);
        while (1) {
            while (*end != '|' && *end != '!') {
                end++;
            }
            failed += check_utf8_segment((const unsigned char *)start,
                    (size_t)(end - start), expect_success);
            if (*end == '!') {
                break;
            }
            start = ++end;
        }
        printf("\n");
    }
    printf("checking utf-8 sequences: %d fail(s)\n", failed);
    return failed;
}

int
check_boms(void)
{
    int failed = 0;
    int k;
    size_t expected_length = strlen((const char *)bom_original);

    printf("checking boms...\n");
    for (k = 0; boms[k].test; k++) {
        const char *start = boms[k].test;
        const char *end = start;

        while (*end != '!') {
            end++;
        }

        failed += check_scalar_case(boms[k].title, (const unsigned char *)start,
                (size_t)(end - start), bom_original, expected_length);
    }
    printf("checking boms: %d fail(s)\n", failed);
    return failed;
}

#define LONG 100000

int
check_long_utf8(void)
{
    int k = 0;
    int j;
    int failed;
    unsigned char *input = (unsigned char *)malloc(3 + LONG*2);
    unsigned char *expected = (unsigned char *)malloc(LONG*2);

    assert(input);
    assert(expected);

    printf("checking a long utf8 sequence...\n");

    input[k++] = '\xef';
    input[k++] = '\xbb';
    input[k++] = '\xbf';
    for (j = 0; j < LONG; j++) {
        unsigned char lo = (j % 2) ? '\x90' : '\xaf';
        input[k++] = '\xd0';
        input[k++] = lo;
        expected[j*2] = '\xd0';
        expected[j*2 + 1] = lo;
    }

    failed = check_scalar_case("decoded scalar", input, (size_t)(3 + LONG*2),
            expected, (size_t)(LONG*2));

    free(expected);
    free(input);

    printf("checking a long utf8 sequence: %d fail(s)\n", failed);
    return failed;
}

int
check_long_utf16(void)
{
    int k = 0;
    int j;
    int failed;
    unsigned char *input = (unsigned char *)malloc(2 + LONG*2);
    unsigned char *expected = (unsigned char *)malloc(LONG*2);

    assert(input);
    assert(expected);

    printf("checking a long utf16 sequence...\n");

    input[k++] = '\xff';
    input[k++] = '\xfe';
    for (j = 0; j < LONG; j++) {
        input[k++] = (j % 2) ? '\x10' : '/';
        input[k++] = '\x04';
        expected[j*2] = '\xd0';
        expected[j*2 + 1] = (j % 2) ? '\x90' : '\xaf';
    }

    failed = check_scalar_case("decoded scalar", input, (size_t)(2 + LONG*2),
            expected, (size_t)(LONG*2));

    free(expected);
    free(input);

    printf("checking a long utf16 sequence: %d fail(s)\n", failed);
    return failed;
}

int
main(void)
{
    return check_utf8_sequences()
         + check_boms()
         + check_long_utf8()
         + check_long_utf16();
}
