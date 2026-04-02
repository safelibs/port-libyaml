#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <stage-root>" >&2
    exit 1
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
safe_dir=$(cd -- "${script_dir}/.." && pwd)
repo_root=$(cd -- "${safe_dir}/.." && pwd)
stage_root=$(cd -- "$1" && pwd)
compiler=${CC:-cc}

multiarch() {
    local value
    value=$({ cc -print-multiarch || gcc -print-multiarch; } 2>/dev/null | head -n 1 || true)
    if [[ -n "${value}" ]]; then
        printf '%s\n' "${value}"
        return 0
    fi

    printf '%s-linux-gnu\n' "$(uname -m)"
}

fail() {
    printf '%s\n' "$*" >&2
    exit 1
}

require_contains() {
    local file=$1
    local needle=$2
    if ! grep -F -- "${needle}" "${file}" >/dev/null 2>&1; then
        printf 'missing expected text in %s: %s\n' "${file}" "${needle}" >&2
        printf -- '--- %s ---\n' "${file}" >&2
        cat "${file}" >&2
        exit 1
    fi
}

require_not_contains() {
    local file=$1
    local needle=$2
    if grep -F -- "${needle}" "${file}" >/dev/null 2>&1; then
        printf 'unexpected text in %s: %s\n' "${file}" "${needle}" >&2
        printf -- '--- %s ---\n' "${file}" >&2
        cat "${file}" >&2
        exit 1
    fi
}

require_count() {
    local file=$1
    local needle=$2
    local expected=$3
    local actual
    actual=$(grep -F -c -- "${needle}" "${file}" || true)
    if [[ "${actual}" != "${expected}" ]]; then
        printf 'unexpected count for %s in %s: expected %s, found %s\n' \
            "${needle}" "${file}" "${expected}" "${actual}" >&2
        printf -- '--- %s ---\n' "${file}" >&2
        cat "${file}" >&2
        exit 1
    fi
}

compare_text_files() {
    local label=$1
    local lhs=$2
    local rhs=$3
    if ! cmp -s -- "${lhs}" "${rhs}"; then
        printf 'mismatched %s outputs\n' "${label}" >&2
        diff -u -- "${lhs}" "${rhs}" >&2 || true
        exit 1
    fi
}

compile_source_compat() {
    local source=$1
    local output=$2

    "${compiler}" \
        -std=c11 \
        -pedantic \
        -Wall \
        -Werror \
        -I"${stage_incdir}" \
        "${source}" \
        -L"${stage_libdir}" \
        -Wl,-rpath,"${stage_libdir}" \
        -lyaml \
        -o "${output}"
}

compile_link_compat() {
    local source=$1
    local object=$2
    local output=$3

    "${compiler}" \
        -std=c11 \
        -pedantic \
        -Wall \
        -Werror \
        -c \
        -I"${orig_incdir}" \
        "${source}" \
        -o "${object}"
    "${compiler}" \
        "${object}" \
        -L"${stage_libdir}" \
        -Wl,-rpath,"${stage_libdir}" \
        -lyaml \
        -o "${output}"
}

run_and_capture() {
    local binary=$1
    local stdout_file=$2
    local stderr_file=$3
    local status_file=$4
    shift 4

    bash "${safe_dir}/scripts/assert-staged-loader.sh" "${stage_root}" "${binary}"

    local status
    set +e
    if [[ -n "${RUN_STDIN_FILE:-}" ]]; then
        LD_LIBRARY_PATH="${stage_libdir}" "${binary}" "$@" <"${RUN_STDIN_FILE}" >"${stdout_file}" 2>"${stderr_file}"
        status=$?
    else
        LD_LIBRARY_PATH="${stage_libdir}" "${binary}" "$@" >"${stdout_file}" 2>"${stderr_file}"
        status=$?
    fi
    set -e
    printf '%s\n' "${status}" > "${status_file}"
}

enforce_case_expectations() {
    local name=$1
    local stdout_file=$2
    local stderr_file=$3
    local status_file=$4

    if [[ $(<"${status_file}") != "0" ]]; then
        printf '%s exited with failure\n' "${name}" >&2
        printf -- '--- stdout ---\n' >&2
        cat "${stdout_file}" >&2
        printf -- '--- stderr ---\n' >&2
        cat "${stderr_file}" >&2
        exit 1
    fi

    case "${name}" in
        run-scanner.c)
            require_count "${stdout_file}" "SUCCESS (" 3
            require_not_contains "${stdout_file}" "FAILURE"
            ;;
        run-parser.c)
            require_count "${stdout_file}" "SUCCESS (" 3
            require_not_contains "${stdout_file}" "FAILURE"
            ;;
        run-parser-test-suite.c)
            require_contains "${stdout_file}" "+STR"
            require_contains "${stdout_file}" "-STR"
            ;;
        run-loader.c)
            require_count "${stdout_file}" "SUCCESS (" 3
            require_not_contains "${stdout_file}" "FAILURE"
            ;;
        run-emitter.c)
            require_count "${stdout_file}" "PASSED (length:" 2
            require_not_contains "${stdout_file}" "FAILED"
            ;;
        run-emitter-test-suite.c)
            require_contains "${stdout_file}" "a: 1"
            ;;
        run-dumper.c)
            require_count "${stdout_file}" "PASSED (length:" 2
            require_not_contains "${stdout_file}" "FAILED"
            ;;
        example-reformatter.c)
            require_contains "${stdout_file}" "foo:"
            require_contains "${stdout_file}" "bar"
            ;;
        example-reformatter-alt.c)
            require_contains "${stdout_file}" "foo:"
            require_contains "${stdout_file}" "baz"
            ;;
        example-deconstructor.c|example-deconstructor-alt.c)
            require_contains "${stdout_file}" "STREAM-START"
            require_contains "${stdout_file}" "STREAM-END"
            ;;
    esac
}

run_case() {
    local name=$1
    local source="${orig_tests_dir}/${name}"
    local stem=${name%.c}
    local case_dir="${tmpdir}/${stem}"
    local reference_binary="${case_dir}/${stem}-source"
    local object_file="${case_dir}/${stem}.o"
    local link_binary="${case_dir}/${stem}-link"
    local -a args=()

    mkdir -p "${case_dir}"
    RUN_STDIN_FILE=

    case "${name}" in
        test-version.c|test-reader.c|test-api.c)
            ;;
        run-scanner.c|run-parser.c|run-loader.c)
            args=(
                "${orig_examples_dir}/anchors.yaml"
                "${orig_examples_dir}/json.yaml"
                "${orig_examples_dir}/mapping.yaml"
            )
            ;;
        run-parser-test-suite.c)
            args=(
                "--flow"
                "keep"
                "${orig_examples_dir}/anchors.yaml"
            )
            ;;
        run-emitter.c)
            args=(
                "${orig_examples_dir}/anchors.yaml"
                "${orig_examples_dir}/json.yaml"
            )
            ;;
        run-emitter-test-suite.c)
            RUN_STDIN_FILE="${case_dir}/stdin"
            printf '+STR\n+DOC\n+MAP\n=VAL :a\n=VAL :1\n-MAP\n-DOC\n-STR\n' > "${RUN_STDIN_FILE}"
            ;;
        run-dumper.c)
            args=(
                "${orig_examples_dir}/anchors.yaml"
                "${orig_examples_dir}/mapping.yaml"
            )
            ;;
        example-reformatter.c)
            RUN_STDIN_FILE="${case_dir}/stdin"
            printf 'foo: [bar, {x: y}]\n' > "${RUN_STDIN_FILE}"
            ;;
        example-reformatter-alt.c)
            RUN_STDIN_FILE="${case_dir}/stdin"
            printf 'foo: [bar, baz]\n' > "${RUN_STDIN_FILE}"
            ;;
        example-deconstructor.c|example-deconstructor-alt.c)
            RUN_STDIN_FILE="${case_dir}/stdin"
            printf 'foo: bar\n' > "${RUN_STDIN_FILE}"
            ;;
        *)
            fail "unsupported matrix entry: ${name}"
            ;;
    esac

    printf '==> %s\n' "${name}"
    compile_source_compat "${source}" "${reference_binary}"
    compile_link_compat "${source}" "${object_file}" "${link_binary}"

    run_and_capture \
        "${reference_binary}" \
        "${case_dir}/reference.stdout" \
        "${case_dir}/reference.stderr" \
        "${case_dir}/reference.status" \
        "${args[@]}"
    run_and_capture \
        "${link_binary}" \
        "${case_dir}/link.stdout" \
        "${case_dir}/link.stderr" \
        "${case_dir}/link.status" \
        "${args[@]}"

    compare_text_files "${name} stdout" "${case_dir}/reference.stdout" "${case_dir}/link.stdout"
    compare_text_files "${name} stderr" "${case_dir}/reference.stderr" "${case_dir}/link.stderr"
    compare_text_files "${name} exit status" "${case_dir}/reference.status" "${case_dir}/link.status"

    enforce_case_expectations \
        "${name}" \
        "${case_dir}/reference.stdout" \
        "${case_dir}/reference.stderr" \
        "${case_dir}/reference.status"
}

arch=$(multiarch)
stage_incdir="${stage_root}/usr/include"
stage_libdir="${stage_root}/usr/lib/${arch}"
orig_incdir="${repo_root}/original/include"
orig_tests_dir="${repo_root}/original/tests"
orig_examples_dir="${repo_root}/original/examples"

if [[ ! -d "${stage_incdir}" ]]; then
    fail "staged include directory not found: ${stage_incdir}"
fi
if [[ ! -d "${stage_libdir}" ]]; then
    fail "staged library directory not found: ${stage_libdir}"
fi
stage_libdir=$(cd -- "${stage_libdir}" && pwd)
if [[ ! -f "${stage_libdir}/libyaml-0.so.2" ]]; then
    fail "staged runtime library not found: ${stage_libdir}/libyaml-0.so.2"
fi
if [[ ! -d "${orig_incdir}" || ! -d "${orig_tests_dir}" || ! -d "${orig_examples_dir}" ]]; then
    fail "original compatibility artifacts are missing under ${repo_root}/original"
fi

tmpdir=$(mktemp -d)
trap 'rm -rf "${tmpdir}"' EXIT

for entry in \
    test-version.c \
    test-reader.c \
    test-api.c \
    run-scanner.c \
    run-parser.c \
    run-parser-test-suite.c \
    run-loader.c \
    run-emitter.c \
    run-emitter-test-suite.c \
    run-dumper.c \
    example-reformatter.c \
    example-reformatter-alt.c \
    example-deconstructor.c \
    example-deconstructor-alt.c
do
    run_case "${entry}"
done
