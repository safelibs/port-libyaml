#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: $0 <runtime-deb> <dev-deb> <doc-deb>" >&2
    exit 1
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
safe_dir=$(cd -- "${script_dir}/.." && pwd)
repo_root=$(cd -- "${safe_dir}/.." && pwd)
runtime_deb=$(cd -- "$(dirname -- "$1")" && pwd)/$(basename -- "$1")
dev_deb=$(cd -- "$(dirname -- "$2")" && pwd)/$(basename -- "$2")
doc_deb=$(cd -- "$(dirname -- "$3")" && pwd)/$(basename -- "$3")

for path in "${runtime_deb}" "${dev_deb}" "${doc_deb}"; do
    if [[ ! -f "${path}" ]]; then
        printf 'missing package artifact: %s\n' "${path}" >&2
        exit 1
    fi
done

if command -v docker >/dev/null 2>&1; then
    container_runtime=docker
elif command -v podman >/dev/null 2>&1; then
    container_runtime=podman
else
    echo "missing container runtime: docker or podman" >&2
    exit 1
fi

container_name=libyaml-safe-ubuntu-24.04

container_exists() {
    "${container_runtime}" ps -a --format '{{.Names}}' | grep -Fx "${container_name}" >/dev/null 2>&1
}

container_running() {
    "${container_runtime}" ps --format '{{.Names}}' | grep -Fx "${container_name}" >/dev/null 2>&1
}

if ! container_exists; then
    "${container_runtime}" run -d --name "${container_name}" ubuntu:24.04 sleep infinity >/dev/null
elif ! container_running; then
    "${container_runtime}" start "${container_name}" >/dev/null
fi

tar -C "${repo_root}" \
    --exclude='./.git' \
    --exclude='./safe/out' \
    --exclude='./safe/target' \
    -cf - safe \
    | "${container_runtime}" exec -i "${container_name}" sh -c 'rm -rf /src/libyaml-safe && mkdir -p /src/libyaml-safe && tar -xf - -C /src/libyaml-safe'

"${container_runtime}" cp "${runtime_deb}" "${container_name}:/tmp/libyaml-0-2.deb"
"${container_runtime}" cp "${dev_deb}" "${container_name}:/tmp/libyaml-dev.deb"
"${container_runtime}" cp "${doc_deb}" "${container_name}:/tmp/libyaml-doc.deb"

"${container_runtime}" exec -i "${container_name}" bash -s -- /tmp/libyaml-0-2.deb /tmp/libyaml-dev.deb /tmp/libyaml-doc.deb <<'EOF'
set -euo pipefail

runtime_deb=$1
dev_deb=$2
doc_deb=$3

export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y --no-install-recommends binutils build-essential pkg-config ripgrep
rm -f /etc/dpkg/dpkg.cfg.d/excludes
apt-get install -y --reinstall --allow-downgrades --no-install-recommends "${runtime_deb}" "${dev_deb}" "${doc_deb}"
ldconfig

multiarch="$(gcc -print-multiarch)" && test -e "/usr/lib/$multiarch/libyaml-0.so.2"
multiarch="$(gcc -print-multiarch)" && readelf -d "/usr/lib/$multiarch/libyaml-0.so.2" | rg -q 'Library soname: \[libyaml-0.so.2\]'
multiarch="$(gcc -print-multiarch)" && test -e "/usr/include/yaml.h" && test -e "/usr/lib/$multiarch/libyaml.so" && test -e "/usr/lib/$multiarch/libyaml.a" && test -e "/usr/lib/$multiarch/pkgconfig/yaml-0.1.pc"
pkg-config --exists yaml-0.1 && pkg-config --cflags --libs yaml-0.1
for path in /usr/share/doc-base/libyaml-doc.libyaml /usr/share/doc/libyaml-doc/changelog.Debian.gz /usr/share/doc/libyaml-doc/copyright /usr/share/doc/libyaml-dev/html/index.html /usr/share/doc/libyaml-dev/example-deconstructor-alt.c.gz /usr/share/doc/libyaml-dev/example-deconstructor.c.gz /usr/share/doc/libyaml-dev/example-reformatter-alt.c.gz /usr/share/doc/libyaml-dev/example-reformatter.c.gz /usr/share/doc/libyaml-dev/run-dumper.c.gz /usr/share/doc/libyaml-dev/run-emitter-test-suite.c.gz /usr/share/doc/libyaml-dev/run-emitter.c.gz /usr/share/doc/libyaml-dev/run-loader.c /usr/share/doc/libyaml-dev/run-parser-test-suite.c.gz /usr/share/doc/libyaml-dev/run-parser.c /usr/share/doc/libyaml-dev/run-scanner.c; do test -e "$path"; done
cd /src/libyaml-safe/safe && sh debian/tests/upstream-tests
cd /src/libyaml-safe/safe && PKGCONFIG="$(pkg-config --cflags --libs yaml-0.1)" && for i in compat/original-tests/test-version.c compat/original-tests/test-reader.c compat/original-tests/test-api.c; do item="/tmp/$(basename "$i" .c)"; gcc -pedantic -Wall -Werror -o "$item" "$i" $PKGCONFIG; "$item"; done
EOF
