#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "usage: $0 <stage-root> [expected-symbol-file]" >&2
    exit 1
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
safe_dir=$(cd -- "${script_dir}/.." && pwd)
stage_root=$1
expected_file=${2:-"${safe_dir}/compat/upstream/libyaml-0-2.symbols"}

multiarch() {
    local value
    value=$({ cc -print-multiarch || gcc -print-multiarch; } 2>/dev/null | head -n 1 || true)
    if [[ -n "${value}" ]]; then
        printf '%s\n' "${value}"
        return 0
    fi

    printf '%s-linux-gnu\n' "$(uname -m)"
}

extract_expected() {
    local input=$1
    if [[ $(basename -- "${input}") == "libyaml-0-2.symbols" ]]; then
        awk '
            /^[[:space:]]*yaml_/ {
                symbol = $1
                sub(/@.*/, "", symbol)
                print symbol
            }
        ' "${input}"
    else
        awk '
            NF > 0 && $1 !~ /^#/ {
                print $1
            }
        ' "${input}"
    fi
}

arch=$(multiarch)
library="${stage_root}/usr/lib/${arch}/libyaml-0.so.2"

if [[ ! -f "${library}" ]]; then
    echo "staged library not found: ${library}" >&2
    exit 1
fi

tmpdir=$(mktemp -d)
trap 'rm -rf "${tmpdir}"' EXIT

extract_expected "${expected_file}" | sort -u > "${tmpdir}/expected"
nm -D --defined-only --format=posix "${library}" \
    | awk 'NF > 0 { symbol = $1; sub(/@.*/, "", symbol); print symbol }' \
    | sort -u > "${tmpdir}/actual"

if ! diff -u "${tmpdir}/expected" "${tmpdir}/actual"; then
    echo "defined dynamic symbols do not match expected export set" >&2
    exit 1
fi
