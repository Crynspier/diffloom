# Contributing

Correctness is the primary acceptance criterion.

For behavioral changes, add a regression test and update the compatibility specification when semantics visible to callers change. Do not change a fixture merely to make a reference implementation agree with Diffloom without first classifying the difference.

## Local checks

    cargo fmt --all -- --check
    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo build --workspace

## Compatibility lab

Reference outputs must be pinned to an exact historical implementation version or commit. Every mismatch is classified as one of:

- required compatibility,
- intentional native-mode behavior,
- historical bug/quirk,
- or Diffloom defect.

## Fuzzing

From an installed cargo-fuzz toolchain:

    cargo fuzz run diff
    cargo fuzz run patch_parse
    cargo fuzz run patch_apply
