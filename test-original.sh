#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

IMAGE_TAG="libyaml-safe-smoke:latest"

for tool in docker python3; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf 'missing required host tool: %s\n' "$tool" >&2
    exit 1
  fi
done

if [[ ! -d safe ]]; then
  printf 'missing safe source tree\n' >&2
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
    "libyaml-libyaml-perl",
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

RUN sed -i 's/^Types: deb$/Types: deb deb-src/' /etc/apt/sources.list.d/ubuntu.sources \
 && apt-get update \
 && apt-get install -y --no-install-recommends \
      appstream \
      binutils \
      build-essential \
      ca-certificates \
      cargo \
      crystal \
      debhelper \
      dh-buildinfo \
      doxygen \
      dpkg-dev \
      fakeroot \
      h2o \
      libcamera-dev \
      libcamera-ipa \
      libyaml-libyaml-perl \
      netplan.io \
      php8.3-cli \
      php8.3-yaml \
      pkg-config \
      python3 \
      python3-yaml \
      ripgrep \
      rustc \
      ruby \
      ruby-psych \
      ser2net \
      strace \
      stubby \
      suricata

RUN mkdir -p /tmp/libcamera-src \
 && cd /tmp/libcamera-src \
 && apt-get source libcamera

RUN cat > /tmp/libcamera-yaml-smoke.cpp <<'CPP'
#include <iostream>
#include <memory>

#include <libcamera/base/file.h>
#include "libcamera/internal/yaml_parser.h"

int main()
{
    libcamera::File file("/usr/share/libcamera/ipa/ipu3/uncalibrated.yaml");
    if (file.open(libcamera::File::OpenModeFlag::ReadOnly) == false) {
        std::cerr << "open failed\n";
        return 1;
    }

    std::unique_ptr<libcamera::YamlObject> root = libcamera::YamlParser::parse(file);
    if (root.get() == nullptr) {
        std::cerr << "parse failed\n";
        return 1;
    }

    if (root->contains("algorithms") == false) {
        std::cerr << "missing algorithms\n";
        return 1;
    }

    const libcamera::YamlObject &algorithms = (*root)["algorithms"];
    if (algorithms.size() == 0) {
        std::cerr << "empty algorithms\n";
        return 1;
    }

    std::cout << "algorithms=" << algorithms.size() << "\n";
    return 0;
}
CPP

RUN LIBCAMERA_SRC_DIR="$(echo /tmp/libcamera-src/libcamera-*/)" \
 && c++ -std=c++17 -DLIBCAMERA_BASE_PRIVATE \
      -I"${LIBCAMERA_SRC_DIR}include" \
      $(pkg-config --cflags libcamera) \
      /tmp/libcamera-yaml-smoke.cpp \
      -o /usr/local/bin/libcamera-yaml-smoke \
      $(pkg-config --libs libcamera) \
 && /usr/local/bin/libcamera-yaml-smoke >/tmp/libcamera-yaml-build.log \
 && rm -rf /tmp/libcamera-src /tmp/libcamera-yaml-smoke.cpp /tmp/libcamera-yaml-build.log

COPY safe /src/libyaml-safe/safe
COPY original /src/libyaml-safe/original

RUN cd /src/libyaml-safe \
 && bash safe/scripts/stage-install.sh /tmp/libyaml-safe-install \
 && bash safe/scripts/verify-link-objects.sh /tmp/libyaml-safe-install \
 && rm -f /etc/dpkg/dpkg.cfg.d/excludes \
 && bash safe/scripts/build-deb.sh \
 && apt-get install -y --allow-downgrades --no-install-recommends \
      /src/libyaml-safe/safe/out/debs/libyaml-0-2.deb \
      /src/libyaml-safe/safe/out/debs/libyaml-dev.deb \
      /src/libyaml-safe/safe/out/debs/libyaml-doc.deb \
 && ldconfig \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /work
DOCKERFILE

docker run --rm -i "$IMAGE_TAG" bash <<'EOF'
set -euo pipefail

log() {
  printf '==> %s\n' "$1"
}

require_contains() {
  local file="$1"
  local needle="$2"

  if ! grep -F -- "$needle" "$file" >/dev/null 2>&1; then
    printf 'missing expected text in %s: %s\n' "$file" "$needle" >&2
    printf -- '--- %s ---\n' "$file" >&2
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
multiarch="$(gcc -print-multiarch)"

test -e "/usr/lib/$multiarch/libyaml-0.so.2"
pkg-config --exists yaml-0.1

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
strace -f -e trace=openat libcamera-yaml-smoke >/tmp/libcamera.log 2>/tmp/libcamera.strace
require_contains /tmp/libcamera.log "algorithms="
require_contains /tmp/libcamera.strace "/usr/share/libcamera/ipa/ipu3/uncalibrated.yaml"

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
pkg-config --libs yaml-0.1 >/tmp/crystal-pkgconfig.log 2>&1
require_contains /tmp/crystal-pkgconfig.log "-lyaml"
crystal build /tmp/crystal-smoke.cr -o /tmp/crystal-smoke >/tmp/crystal-build.log 2>&1
ldd /tmp/crystal-smoke >/tmp/crystal-ldd.log 2>&1
require_contains /tmp/crystal-ldd.log "libyaml-0.so.2 =>"
strace -f -e trace=openat /tmp/crystal-smoke >/tmp/crystal.log 2>/tmp/crystal.strace
require_contains /tmp/crystal.log "count: 3"

log "libyaml-libyaml-perl"
perl >/tmp/perl-yaml.log <<'PERL'
use strict;
use warnings;
use YAML::XS qw(Load Dump);

my $version = join('.', YAML::XS::LibYAML::libyaml_version());
die "missing libyaml version\n" unless $version =~ /^\d+\.\d+\.\d+$/;

my $data = Load("---\na: 1\nb:\n  - x\n  - y\n");
die "bad parse\n" unless $data->{a} == 1 && $data->{b}[1] eq 'y';
my $emitted = Dump($data);
die "missing emit output\n" unless index($emitted, "a: 1") >= 0;
print "libyaml=$version\n$emitted";
PERL
require_contains /tmp/perl-yaml.log "libyaml="
require_contains /tmp/perl-yaml.log "a: 1"
EOF
