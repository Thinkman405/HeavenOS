#!/usr/bin/env bash
# Compiles ffi_test against the freshly-built media_ffi shared library and
# runs it — the real, independent-of-Rust proof that the C ABI genuinely
# links, as opposed to Rust calling its own extern "C" functions. Run
# `cargo build -p media-ffi` first. Linux companion to build_and_run.bat.
set -euo pipefail
cd "$(dirname "$0")"
gcc -Wall main.c -L../../target/debug -lmedia_ffi -lm -Wl,-rpath,../../target/debug -o ffi_test
./ffi_test
