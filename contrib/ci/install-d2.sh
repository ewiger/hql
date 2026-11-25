#!/bin/sh
# Install `d2` on a Linux x86_64 CI runner, from the pinned release of the
# HyperMarkDown toolchain (https://github.com/ewiger/hypermarkdown-toolchain).
#
# The site's `d2` fences are drawn at build time by the `d2` binary; without
# it a diagram degrades to its source. The toolchain archive is one release
# asset, fetched by its URL and checked against the digest below, so the
# install makes no GitHub API request and cannot be rate-limited by one.
#
# The archive is kept in `~/.cache/hypermarkdown-toolchain`, which the
# workflows save in the CI cache, keyed on this file. A run that finds the
# archive there downloads nothing. The digest is checked on every run, on a
# cached archive as on a downloaded one.
#
# To move to another toolchain release, change `version` and `sha256`
# together. Both are in `toolchain.json` of ewiger/hypermarkdown, under
# `archives.linux-x86_64`.
#
# Only `d2` is installed. The archive also holds `hql` and `hmd-lsp`, and
# they stay out of PATH: this repository builds its own `hql`.
set -eu

version=0.6.0
sha256=6ee19ddc6b94c653b0df6680dcf18b75a1f3948af74f9cd04ac623b9fc5f4c58

name="hypermarkdown-toolchain-$version-linux-x86_64.tar.gz"
url="https://github.com/ewiger/hypermarkdown-toolchain/releases/download/v$version/$name"
bin="$HOME/.local/bin"
cache="$HOME/.cache/hypermarkdown-toolchain"
archive="$cache/$name"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

verify() {
  echo "$sha256  $archive" | sha256sum -c - >/dev/null 2>&1
}

mkdir -p "$cache"
if [ -f "$archive" ] && verify; then
  echo "toolchain $version: from the cache"
else
  rm -f "$archive"
  echo "toolchain $version: $url"
  curl -fsSL --retry 3 --retry-all-errors -o "$archive" "$url"
  if ! verify; then
    echo "digest mismatch for $name, expected $sha256" >&2
    rm -f "$archive"
    exit 1
  fi
fi
tar -xzf "$archive" -C "$tmp" bin/d2

mkdir -p "$bin"
install -m 755 "$tmp/bin/d2" "$bin/d2"
if [ -n "${GITHUB_PATH:-}" ]; then
  echo "$bin" >> "$GITHUB_PATH"
fi
"$bin/d2" --version
