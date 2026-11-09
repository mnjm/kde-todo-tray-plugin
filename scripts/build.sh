#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --locked
mkdir -p package/contents/bin
cp target/release/markdown-todo package/contents/bin/markdown-todo
printf '%s\n' 'Built package/. See README.md for installation.'
