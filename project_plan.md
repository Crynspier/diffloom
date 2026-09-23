# Diffloom Project Plan

Diffloom is a compatibility-first modernization project for text diff, match, and patch tooling.

## Goal

Provide a small, dependable, maintained replacement for historical Diff Match Patch-style tooling while making compatibility boundaries explicit.

## Architecture

diffloom-core contains diffing, matching, patching, compatibility adapters, and safety controls. The CLI is a thin wrapper. The FFI crate exposes a narrow C ABI.

## Core semantics

Native operations use Unicode scalar values. Diff search uses common prefix/suffix acceleration and Myers-style shortest-edit-script behavior with bounded work and safe fallback.

## Matching

Exact matching is attempted first. Approximate matching uses bounded candidate search and normalized Levenshtein distance with explicit threshold, distance, work, and input-size limits.

## Patching

Patch creation adds context and merges nearby hunks. Serialization uses Diff Match Patch-style headers and operation prefixes. Parsing validates ranges, encoding, declared lengths, line size, total input size, and patch count. Application tracks coordinate delta across successful hunks.

## Compatibility

The compatibility surface covers DMP operation constants, tuple conversion, patch headers, operation markers, percent-decoded UTF-8 payloads, and an ASCII/BMP interoperability target.

Known divergence surfaces include UTF-16 indexing, cleanup heuristics, approximate-match ranking, timing behavior, and historical hunk splitting.

## Testing

Unit tests, exhaustive small-input reconstruction tests, patch round trips, regression vectors, and fuzz targets cover the core. CI verifies Linux, Windows, and macOS.

## Security

Resource limits, strict patch validation, patch-count limits, and explicit C ownership contracts address the primary CPU, memory, malformed-input, and FFI risks.

## Milestones

0.1 — core: implemented.
0.2 — compatibility surface: implemented.
0.3 — matching and patch creation: implemented.
0.4 — patch serialization, parsing, and application: implemented.
0.5 — hardening, security controls, and fuzzing: implemented.
0.6 — cross-platform engineering and MSRV coverage: implemented.
0.7 — CLI: implemented.
0.8 — C ABI: implemented.
0.9 — compatibility and release infrastructure: implemented.
1.0 feature scope — implemented in the 0.1.0 tree.

Semver 1.0.0 remains an API-stability and external-integration evidence gate rather than a missing feature milestone.

> Make Diffloom boring to depend on.
