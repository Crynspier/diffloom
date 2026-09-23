use diffloom_core::{
    patch_apply, patch_from_text, patch_make, patch_to_text, DiffEngine, DiffOptions,
    MatchOptions, Operation,
};

fn apply_diffs(old: &str, diffs: &[diffloom_core::Diff]) -> String {
    let old_chars: Vec<char> = old.chars().collect();
    let mut cursor = 0usize;
    let mut out = String::new();

    for diff in diffs {
        match diff.operation {
            Operation::Equal => {
                let len = diff.text.chars().count();
                assert_eq!(&old_chars[cursor..cursor + len], &diff.text.chars().collect::<Vec<_>>());
                cursor += len;
                out.push_str(&diff.text);
            }
            Operation::Delete => cursor += diff.text.chars().count(),
            Operation::Insert => out.push_str(&diff.text),
        }
    }

    assert_eq!(cursor, old_chars.len());
    out
}

#[test]
fn representative_invariants_hold() {
    let cases = [
        ("", "hello"),
        ("hello", ""),
        ("hello world", "hello brave world"),
        ("The quick brown fox", "The quick red fox"),
        ("zažółć gęślą", "zażółć gęślą!"),
        ("abcabcabc", "xbcabcabz"),
    ];

    for (old, new) in cases {
        let diffs = DiffEngine::new(DiffOptions::default()).diff(old, new);
        assert_eq!(apply_diffs(old, &diffs), new);

        let patches = patch_make(old, new, DiffOptions::default());
        let serialized = patch_to_text(&patches);
        let parsed = patch_from_text(&serialized).expect("generated patch must parse");
        let applied = patch_apply(&parsed, old, MatchOptions::default())
            .expect("patch application must not fail");
        assert!(applied.applied.iter().all(|value| *value));
        assert_eq!(applied.text, new);
    }
}
