# Public API

## Core

DiffEngine returns normalized delete, insert, and equal operations.

DiffOptions:

- max_work
- max_diffs
- max_input_units
- semantic_cleanup

MatchOptions:

- threshold
- distance
- max_work
- max_input_units

PatchOptions:

- context
- merge_distance

PatchParseOptions:

- max_input_bytes
- max_patches
- max_line_bytes

ApplyResult contains the resulting text and one boolean for each input patch.

## Compatibility adapter

diffs_to_dmp_tuples and diffs_from_dmp_tuples map native operations to the traditional Diff Match Patch constants -1, +1, and 0.

## CLI

The diffloom binary provides file diffing, patch creation/application, approximate matching, help, and version reporting.

## C ABI

The FFI crate exports diffloom_abi_version, diffloom_diff, diffloom_patch_make, diffloom_patch_apply, and diffloom_free_string.

Returned strings are owned by Diffloom and must be released exactly once with diffloom_free_string.
