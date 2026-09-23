# Migration

A migration from a historical Diff Match Patch port should start with patch data.

1. Preserve existing patch text.
2. Parse it through the compatibility API.
3. Run the application corpus with strict success checks.
4. Compare outputs for ASCII/BMP fixtures.
5. Investigate supplementary-Unicode and heuristic differences separately.

The native Rust API should be introduced only after the caller has selected explicit semantics for text positions and approximate matching.
