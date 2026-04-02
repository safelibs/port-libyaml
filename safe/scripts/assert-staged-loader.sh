#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <stage-root> <binary>" >&2
    exit 1
fi

stage_root=$1
binary=$2

multiarch() {
    local value
    value=$({ cc -print-multiarch || gcc -print-multiarch; } 2>/dev/null | head -n 1 || true)
    if [[ -n "${value}" ]]; then
        printf '%s\n' "${value}"
        return 0
    fi

    printf '%s-linux-gnu\n' "$(uname -m)"
}

arch=$(multiarch)
stage_lib_dir=$(cd -- "${stage_root}/usr/lib/${arch}" && pwd)
expected="${stage_lib_dir}/libyaml-0.so.2"

ldd_output=$(LD_LIBRARY_PATH="${stage_lib_dir}" ldd "${binary}")
resolved=$(printf '%s\n' "${ldd_output}" | awk '/libyaml-0\.so\.2/ { print $3; exit }')

if [[ -z "${resolved}" ]]; then
    echo "ldd did not resolve libyaml-0.so.2 for ${binary}" >&2
    printf '%s\n' "${ldd_output}" >&2
    exit 1
fi

if [[ "${resolved}" != "${expected}" ]]; then
    echo "unexpected libyaml loader path" >&2
    echo "expected: ${expected}" >&2
    echo "actual:   ${resolved}" >&2
    printf '%s\n' "${ldd_output}" >&2
    exit 1
fi
