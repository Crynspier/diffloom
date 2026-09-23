# Security

Diffloom accepts untrusted text and patch data. The principal risks are CPU and memory exhaustion, malformed patch input, and FFI misuse.

## Controls

- Core code forbids unsafe Rust.
- Diffing has a configurable work budget and maximum operation count.
- Matching has bounded search distance and work budget.
- Patch parsing validates syntax, percent encoding, and declared lengths.
- Patch application reports a result for every patch and never silently treats a failed match as success.
- The C ABI rejects null pointers and invalid UTF-8 and exposes one explicit string-free function.

Applications should also cap request sizes and the number of patches processed in a single request.

## Reporting

Please report security issues privately to the repository owner. Include a minimal reproducer, affected version, impact, and required input conditions.
