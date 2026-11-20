#!/bin/sh
# Build the browser module of the playground (HQL-0061) and put it where the
# site reads it: doc/public/assets/wasm/. The folder is a build output and is
# not committed; `mkdocs build` publishes whatever is in it.
#
# Needs: the wasm32-unknown-unknown target (`rustup target add`), wasm-pack,
# and a clang that emits wasm for SQLite's C source. Ubuntu's clang does;
# Apple's does not, so on macOS Homebrew's LLVM is used when it is installed.
set -eu

root=$(cd "$(dirname "$0")/../.." && pwd)
out="$root/doc/public/assets/wasm"

if [ -z "${CC_wasm32_unknown_unknown:-}" ] && [ -x /opt/homebrew/opt/llvm/bin/clang ]; then
    export CC_wasm32_unknown_unknown=/opt/homebrew/opt/llvm/bin/clang
    export AR_wasm32_unknown_unknown=/opt/homebrew/opt/llvm/bin/llvm-ar
fi

wasm-pack build --target web --release --no-typescript \
    "$root/crates/hql-wasm" --out-dir "$root/target/playground"

mkdir -p "$out"
cp "$root/target/playground/hql_wasm.js" "$root/target/playground/hql_wasm_bg.wasm" "$out/"

# The size a reader downloads, against the budget of HQL-0061.
size=$(gzip -9 -c "$out/hql_wasm_bg.wasm" | wc -c | tr -d ' ')
budget=4194304
echo "hql_wasm_bg.wasm: $size bytes gzip-compressed (budget $budget)"
if [ "$size" -gt $((budget + budget / 4)) ]; then
    echo "the module is more than 25 % over its budget" >&2
    exit 1
fi
