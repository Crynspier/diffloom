# Diff Match Patch compatibility contract v1

This document defines the compatibility boundary for Diffloom 0.1.0.

## Canonical text model

Native Diffloom text is UTF-8 and diff operations work over Unicode scalar values. Public positions and lengths are scalar-value counts.

Historical JavaScript and Java implementations commonly reason in UTF-16 code units. That representation can split supplementary characters differently from Diffloom's native model.

## Diff operations

The only operation types are:

- Delete
- Insert
- Equal

Empty operations are removed and adjacent operations of the same type are merged.

The native diff search is a deterministic Myers shortest-edit-script implementation with common prefix/suffix acceleration. A work limit can terminate search; unresolved content is represented as one delete followed by one insert so reconstruction remains correct.

## Matching

Exact substring matches are preferred. Otherwise Diffloom searches a bounded candidate interval and scores equal-length windows with normalized Levenshtein distance.

The threshold is the maximum normalized distance and distance bounds candidate positions around the requested location.

This is a native contract, not a claim to reproduce every historical Bitap scoring detail.

## Patch grammar

    @@ -old-start,old-length +new-start,new-length @@
     equal
    -delete
    +insert

Payload bytes are percent encoded as UTF-8. Header positions use the familiar one-based convention for non-empty ranges and zero for empty ranges.

## Patch creation

Changed regions receive configurable context on both sides. Nearby patches are merged and re-diffed against the merged source/target ranges.

The implementation favors stable, reviewable patches over reproducing every historical hunk-splitting heuristic.

## Patch application

Application builds the expected old chunk from equal and delete operations, locates it near the declared old position, prefers an exact match, then permits bounded approximate matching. Every patch produces a boolean application result.

## Interoperability

0.1.0 targets interoperability of patch grammar and payload encoding with historical Diff Match Patch implementations for ASCII and BMP text.

No universal byte-for-byte output identity is claimed for:

- UTF-16 surrogate-pair handling,
- heuristic cleanup,
- timing-dependent deadlines,
- approximate-match ranking,
- historical patch splitting.

## Provenance

Compatibility fixtures should identify the exact historical implementation/version/commit that produced the reference output.

## Stability

The Rust API is pre-1.0. Patch text is a compatibility surface and changes to its grammar require an explicit migration note.
