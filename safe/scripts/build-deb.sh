#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
safe_dir=$(cd -- "${script_dir}/.." && pwd)
repo_root=$(cd -- "${safe_dir}/.." && pwd)
out_dir="${safe_dir}/out/debs"

cleanup_root_artifacts() {
    shopt -s nullglob
    local paths=(
        "${repo_root}"/libyaml-0-2_*_*.deb
        "${repo_root}"/libyaml-0-2-dbgsym_*_*.ddeb
        "${repo_root}"/libyaml-dev_*_*.deb
        "${repo_root}"/libyaml-doc_*_all.deb
        "${repo_root}"/libyaml_*_*.changes
        "${repo_root}"/libyaml_*_*.buildinfo
    )
    if ((${#paths[@]} > 0)); then
        rm -f -- "${paths[@]}"
    fi
}

cleanup_debian_artifacts() {
    shopt -s nullglob
    local paths=(
        "${safe_dir}"/debian/.cargo-home
        "${safe_dir}"/debian/.debhelper
        "${safe_dir}"/debian/debhelper-build-stamp
        "${safe_dir}"/debian/files
        "${safe_dir}"/debian/*.debhelper.log
        "${safe_dir}"/debian/*.substvars
        "${safe_dir}"/debian/libyaml-0-2
        "${safe_dir}"/debian/libyaml-dev
        "${safe_dir}"/debian/libyaml-doc
        "${safe_dir}"/debian/tmp
    )
    if ((${#paths[@]} > 0)); then
        rm -rf -- "${paths[@]}"
    fi
}

mkdir -p "${out_dir}"
rm -rf "${out_dir:?}/"*
cleanup_root_artifacts
cleanup_debian_artifacts

(
    cd "${safe_dir}"
    dpkg-buildpackage -us -uc -b
)

mkdir -p "${out_dir}"

shopt -s nullglob
runtime_debs=("${repo_root}"/libyaml-0-2_*_*.deb)
dev_debs=("${repo_root}"/libyaml-dev_*_*.deb)
doc_debs=("${repo_root}"/libyaml-doc_*_all.deb)

if ((${#runtime_debs[@]} != 1)); then
    printf 'expected exactly one runtime package, found %s\n' "${#runtime_debs[@]}" >&2
    exit 1
fi
if ((${#dev_debs[@]} != 1)); then
    printf 'expected exactly one development package, found %s\n' "${#dev_debs[@]}" >&2
    exit 1
fi
if ((${#doc_debs[@]} != 1)); then
    printf 'expected exactly one documentation package, found %s\n' "${#doc_debs[@]}" >&2
    exit 1
fi

cp -f -- "${runtime_debs[0]}" "${out_dir}/libyaml-0-2.deb"
cp -f -- "${dev_debs[0]}" "${out_dir}/libyaml-dev.deb"
cp -f -- "${doc_debs[0]}" "${out_dir}/libyaml-doc.deb"

cleanup_root_artifacts
cleanup_debian_artifacts
