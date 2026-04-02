#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

IMAGE_TAG="libyaml-original-smoke:latest"

for tool in docker python3; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf 'missing required host tool: %s\n' "$tool" >&2
    exit 1
  fi
done

if [[ ! -d original ]]; then
  printf 'missing original source tree\n' >&2
  exit 1
fi

if [[ ! -f dependents.json ]]; then
  printf 'missing dependents.json\n' >&2
  exit 1
fi

python3 - <<'PY'
import json
from pathlib import Path

expected = [
    "libnetplan1",
    "python3-yaml",
    "ruby-psych",
    "php8.3-yaml",
    "suricata",
    "stubby",
    "ser2net",
    "h2o",
    "libcamera0.2",
    "libappstream5",
    "crystal",
]

data = json.loads(Path("dependents.json").read_text(encoding="utf-8"))
actual = [entry["name"] for entry in data["dependents"]]

if actual != expected:
    raise SystemExit(
        f"unexpected dependents.json contents: expected {expected}, found {actual}"
    )
PY

docker build -t "$IMAGE_TAG" -f - . <<'DOCKERFILE'
FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      autoconf \
      automake \
      appstream \
      build-essential \
      ca-certificates \
      crystal \
      h2o \
      libcamera-ipa \
      libcamera-tools \
      libtool \
      netplan.io \
      php8.3-cli \
      php8.3-yaml \
      pkg-config \
      python3 \
      python3-yaml \
      ruby \
      ruby-psych \
      ser2net \
      strace \
      stubby \
      suricata \
 && rm -rf /var/lib/apt/lists/*

COPY original /src/libyaml

RUN cd /src/libyaml \
 && ./bootstrap \
 && ./configure --prefix=/usr/local \
 && make -j"$(nproc)" \
 && make check \
 && make install \
 && ldconfig

WORKDIR /work
DOCKERFILE

docker run --rm -i "$IMAGE_TAG" bash <<'EOF'
set -euo pipefail

export LD_LIBRARY_PATH="/usr/local/lib:/usr/local/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PATH="/usr/local/bin:$PATH"

log() {
  printf '==> %s\n' "$1"
}

require_contains() {
  local file="$1"
  local needle="$2"

  if ! grep -F "$needle" "$file" >/dev/null 2>&1; then
    printf 'missing expected text in %s: %s\n' "$file" "$needle" >&2
    printf '--- %s ---\n' "$file" >&2
    cat "$file" >&2
    exit 1
  fi
}

run_ser2net() {
  local status
  set +e
  timeout 10 ser2net -c /tmp/ser2net.yaml -n -d >/tmp/ser2net.log 2>&1
  status=$?
  set -e

  if [[ "$status" != "0" && "$status" != "124" ]]; then
    printf 'ser2net exited with unexpected status %s\n' "$status" >&2
    cat /tmp/ser2net.log >&2
    exit 1
  fi
}

mkdir -p /tmp/libyaml-smoke
cd /tmp/libyaml-smoke

log "netplan.io"
mkdir -p root/etc/netplan
chmod 700 root/etc/netplan
cat > root/etc/netplan/01-smoke.yaml <<'YAML'
network:
  version: 2
  renderer: networkd
  ethernets:
    eth0:
      dhcp4: true
YAML
chmod 600 root/etc/netplan/01-smoke.yaml
netplan get --root-dir "$(pwd)/root" all > /tmp/netplan-get.log
require_contains /tmp/netplan-get.log "renderer: networkd"
netplan set --root-dir "$(pwd)/root" --origin-hint smoke ethernets.eth0.dhcp6=false
require_contains "$(pwd)/root/etc/netplan/smoke.yaml" "dhcp6: false"
netplan generate --root-dir "$(pwd)/root" >/tmp/netplan-generate.log 2>&1

log "python3-yaml"
python3 <<'PY'
import yaml

assert yaml.__with_libyaml__
data = yaml.load("a: 1\nb:\n  - x\n  - y\n", Loader=yaml.CLoader)
assert data == {"a": 1, "b": ["x", "y"]}
emitted = yaml.dump(data, Dumper=yaml.CDumper, sort_keys=True)
assert "a: 1" in emitted
assert "- x" in emitted
PY

log "ruby-psych"
ruby <<'RUBY'
require "psych"

abort "missing libyaml" unless Psych.libyaml_version
data = Psych.safe_load("---\na: 1\nb:\n  - x\n")
abort "bad parse" unless data == { "a" => 1, "b" => ["x"] }
emitted = Psych.dump(data)
abort "missing emit output" unless emitted.include?("a: 1")
RUBY

log "php8.3-yaml"
php <<'PHP'
<?php
if (!function_exists('yaml_parse') || !function_exists('yaml_emit')) {
    fwrite(STDERR, "yaml extension is unavailable\n");
    exit(1);
}
$data = yaml_parse("---\na: 1\nb:\n  - x\n");
if (!is_array($data) || $data['a'] !== 1 || $data['b'][0] !== 'x') {
    fwrite(STDERR, "yaml_parse returned unexpected data\n");
    exit(1);
}
$emitted = yaml_emit($data);
if (strpos($emitted, "a: 1") === false) {
    fwrite(STDERR, "yaml_emit output missing expected scalar\n");
    exit(1);
}
PHP

log "suricata"
mkdir -p /tmp/suricata-logs /tmp/suricata-rules
: > /tmp/suricata-rules/suricata.rules
sed 's#/var/lib/suricata/rules#/tmp/suricata-rules#g' /etc/suricata/suricata.yaml > /tmp/suricata.yaml
suricata -T -c /tmp/suricata.yaml -l /tmp/suricata-logs >/tmp/suricata.log 2>&1

log "stubby"
stubby -i -C /etc/stubby/stubby.yml >/tmp/stubby.log 2>&1

log "ser2net"
cat > /tmp/ser2net.yaml <<'YAML'
%YAML 1.1
---
connection: &smoke
  accepter: tcp,localhost,23001
  enable: off
  connector: serialdev,
            /dev/null,
            9600n81,local
YAML
run_ser2net

log "h2o"
h2o -t -c /etc/h2o/h2o.conf >/tmp/h2o.log 2>&1

log "libcamera0.2"
strace -f -e trace=openat cam --list >/tmp/libcamera.log 2>/tmp/libcamera.strace
require_contains /tmp/libcamera.strace "/usr/local/lib/libyaml-0.so.2"

log "libappstream5"
cat > /tmp/appstream.metainfo.xml <<'XML'
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>org.example.LibyamlSmoke</id>
  <metadata_license>MIT</metadata_license>
  <project_license>MIT</project_license>
  <name>Libyaml Smoke</name>
  <summary>Smoke test metadata</summary>
  <description>
    <p>Smoke test metadata.</p>
  </description>
  <launchable type="desktop-id">org.example.LibyamlSmoke.desktop</launchable>
  <releases>
    <release version="1.0.0" date="2026-04-01">
      <description>
        <p>Initial release.</p>
      </description>
    </release>
  </releases>
</component>
XML
appstreamcli metainfo-to-news --format yaml /tmp/appstream.metainfo.xml /tmp/appstream.news.yml >/tmp/appstream-metainfo.log 2>&1
require_contains /tmp/appstream.news.yml "Version: 1.0.0"
appstreamcli news-to-metainfo --format yaml /tmp/appstream.news.yml /tmp/appstream.metainfo.xml /tmp/appstream-roundtrip.xml >/tmp/appstream-news.log 2>&1
require_contains /tmp/appstream-roundtrip.xml "release type=\"stable\" version=\"1.0.0\""

log "crystal"
cat > /tmp/crystal-smoke.cr <<'CRYSTAL'
require "yaml"

record Example, name : String, count : Int32 do
  include YAML::Serializable
end

example = Example.from_yaml("name: demo\ncount: 3\n")
raise "bad parse" unless example.name == "demo" && example.count == 3
emitted = example.to_yaml
raise "bad emit" unless emitted.includes?("count: 3")
puts emitted
CRYSTAL
crystal run /tmp/crystal-smoke.cr >/tmp/crystal.log 2>&1
require_contains /tmp/crystal.log "count: 3"
EOF
