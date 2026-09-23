# Benchmarks

The benchmark smoke program is intentionally dependency-free.

Run:

    cargo bench -p diffloom-core --bench basic

Benchmark output is observational rather than an API guarantee. The correctness acceptance bar is reconstruction and patch invariants, not a single throughput number.
