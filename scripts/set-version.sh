#!/usr/bin/env bash

set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <semver>" >&2
    exit 1
fi

version="$1"

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "expected a semantic version like 0.1.1, got: $version" >&2
    exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo_toml_tmp="$(mktemp "${TMPDIR:-/tmp}/tld-cargo-toml.XXXXXX")"
awk -v version="$version" '
    BEGIN { updated = 0 }
    !updated && /^version = "/ {
        print "version = \"" version "\""
        updated = 1
        next
    }
    { print }
' Cargo.toml >"$cargo_toml_tmp"
mv "$cargo_toml_tmp" Cargo.toml

cargo_lock_tmp="$(mktemp "${TMPDIR:-/tmp}/tld-cargo-lock.XXXXXX")"
awk -v version="$version" '
    BEGIN {
        in_tld_pkg = 0
        updated = 0
    }
    /^\[\[package\]\]$/ {
        in_tld_pkg = 0
    }
    /^name = "tld"$/ {
        in_tld_pkg = 1
        print
        next
    }
    in_tld_pkg && !updated && /^version = "/ {
        print "version = \"" version "\""
        updated = 1
        in_tld_pkg = 0
        next
    }
    { print }
' Cargo.lock >"$cargo_lock_tmp"
mv "$cargo_lock_tmp" Cargo.lock
