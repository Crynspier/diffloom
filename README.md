# Diffloom

A modern, compatibility-first diff, match, and patch engine.

Diffloom provides a small native Rust core for computing text diffs, locating approximate matches, and creating and applying patches. It also includes a lightweight command-line interface and a C ABI for integration from other languages.

## Features

- Unicode-scalar diff operations with common prefix/suffix acceleration.
- Configurable work, input, and output limits.
- Approximate matching with configurable threshold, search distance, work budget, and input size.
- Patch creation, serialization, parsing, context, merging, and application.
- Diff Match Patch-style patch text format.
- Command-line interface for common file workflows.
- C ABI with explicit string ownership and ABI versioning.
- Cross-platform CI, regression tests, exhaustive small-input tests, and fuzz targets.

## Compatibility

Diffloom's native text model uses Unicode scalar values. The patch text format follows the established Diff Match Patch grammar and targets interoperability with existing implementations.

Behavioral differences from historical ports can occur in Unicode indexing, cleanup heuristics, approximate matching, and patch construction. These semantics are documented in docs/spec/DMP-COMPATIBILITY-V1.md.

## Rust

    use diffloom_core::{DiffEngine, DiffOptions};

    let engine = DiffEngine::new(DiffOptions::default());
    let diffs = engine.diff("hello", "hullo");

## CLI

    diffloom diff old.txt new.txt
    diffloom patch make old.txt new.txt > change.patch
    diffloom patch apply change.patch old.txt new.txt
    diffloom match "hello brave world" "brave" 0

## C ABI

    char *diffloom_diff(const char *old_text, const char *new_text);
    char *diffloom_patch_make(const char *old_text, const char *new_text);
    char *diffloom_patch_apply(const char *patch_text, const char *input_text);
    void diffloom_free_string(char *value);

Returned strings are owned by Diffloom and must be released with diffloom_free_string.

## Development

    cargo fmt --all -- --check
    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo build --workspace
