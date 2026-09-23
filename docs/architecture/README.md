# Architecture

Diffloom has three thin layers.

1. diffloom-core contains all text semantics and owns correctness-sensitive algorithms.
2. diffloom is a dependency-light CLI wrapper for files and shell workflows.
3. diffloom-ffi exposes a narrow C ABI without leaking Rust data structures.

The core intentionally has no unsafe code and no runtime dependency other than the Rust standard library.
