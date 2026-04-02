#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <stage-root>" >&2
    exit 1
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
safe_dir=$(cd -- "${script_dir}/.." && pwd)
stage_root=$1

multiarch() {
    local value
    value=$({ cc -print-multiarch || gcc -print-multiarch; } 2>/dev/null | head -n 1 || true)
    if [[ -n "${value}" ]]; then
        printf '%s\n' "${value}"
        return 0
    fi

    printf '%s-linux-gnu\n' "$(uname -m)"
}

version() {
    sed -n 's/^version = "\([^"]*\)"/\1/p' "${safe_dir}/Cargo.toml" | head -n 1
}

arch=$(multiarch)
target_dir=${CARGO_TARGET_DIR:-"${safe_dir}/target/stage-install"}
build_dir="${target_dir}/release"
stage_lib_dir="${stage_root}/usr/lib/${arch}"
pkgconfig_dir="${stage_lib_dir}/pkgconfig"

mkdir -p \
    "${stage_root}/usr/include" \
    "${stage_lib_dir}" \
    "${pkgconfig_dir}"

export CARGO_TARGET_DIR="${target_dir}"
cargo build --manifest-path "${safe_dir}/Cargo.toml" --release --locked --offline

install -m 0644 "${safe_dir}/include/yaml.h" "${stage_root}/usr/include/yaml.h"
install -m 0755 "${build_dir}/libyaml.so" "${stage_lib_dir}/libyaml-0.so.2"
ln -sfn "libyaml-0.so.2" "${stage_lib_dir}/libyaml.so"
install -m 0644 "${build_dir}/libyaml.a" "${stage_lib_dir}/libyaml.a"

sed \
    -e 's|@prefix@|/usr|g' \
    -e 's|@exec_prefix@|/usr|g' \
    -e 's|@includedir@|/usr/include|g' \
    -e "s|@libdir@|/usr/lib/${arch}|g" \
    -e "s|@PACKAGE_VERSION@|$(version)|g" \
    "${safe_dir}/pkgconfig/yaml-0.1.pc.in" \
    > "${pkgconfig_dir}/yaml-0.1.pc"

printf '%s\n' "${stage_root}"
