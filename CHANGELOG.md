# Changelog

## 0.1.0 — 2026-09-23

Feature-complete first public implementation.

### Added

- Core Myers-style diff engine.
- Unicode-scalar canonical text model.
- Semantic cleanup and deterministic diff merging.
- Configurable work and output-size limits.
- Approximate matching.
- Patch creation, context, serialization, strict parsing, and application.
- Standalone CLI.
- C ABI with explicit allocation/free contract.
- Compatibility specification and fixtures.
- Unit and exhaustive small-input tests.
- Fuzz harnesses.
- Cross-platform CI.
- Security and release documentation.

### Compatibility

The patch grammar follows the established Diff Match Patch line format. Exact historical output identity is not guaranteed for all Unicode and heuristic cases; the boundary is documented in the compatibility specification.
