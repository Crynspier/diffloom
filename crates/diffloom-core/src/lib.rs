#![forbid(unsafe_code)]

//! Diffloom core: deterministic diff, approximate match, and patch primitives.
//!
//! Native text uses Unicode scalar values. Patch serialization follows the
//! established Diff Match Patch grammar while native positions and lengths are
//! Unicode-scalar counts.

//! Diffloom core: deterministic diff, approximate match, and patch primitives.
//!
//! The native engine works in Unicode scalar values. Patch serialization follows
//! the familiar Diff Match Patch line format, while 0.1.0 defines Unicode-scalar
//! positions as its canonical native model.

use std::fmt;

const DEFAULT_MAX_WORK: u64 = 4_000_000;
const DEFAULT_MAX_DIFFS: usize = 100_000;
const DEFAULT_MAX_INPUT_UNITS: usize = 1_000_000;
const DEFAULT_MATCH_DISTANCE: usize = 1_000;
const DEFAULT_MATCH_THRESHOLD: f64 = 0.5;
const PATCH_CONTEXT: usize = 4;
const PATCH_MERGE_DISTANCE: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Delete,
    Insert,
    Equal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diff {
    pub operation: Operation,
    pub text: String,
}

impl Diff {
    pub fn new(operation: Operation, text: impl Into<String>) -> Self {
        Self { operation, text: text.into() }
    }
}

/// Historical Diff Match Patch operation constants.
pub const DMP_DELETE: i8 = -1;
pub const DMP_INSERT: i8 = 1;
pub const DMP_EQUAL: i8 = 0;

/// Converts native operations to the traditional Diff Match Patch tuple form.
pub fn diffs_to_dmp_tuples(diffs: &[Diff]) -> Vec<(i8, String)> {
    diffs
        .iter()
        .map(|diff| {
            let operation = match diff.operation {
                Operation::Delete => DMP_DELETE,
                Operation::Insert => DMP_INSERT,
                Operation::Equal => DMP_EQUAL,
            };
            (operation, diff.text.clone())
        })
        .collect()
}

/// Converts traditional Diff Match Patch tuples into native operations.
pub fn diffs_from_dmp_tuples(
    tuples: &[(i8, &str)],
) -> Result<Vec<Diff>, InvalidDmpOperation> {
    let mut diffs = Vec::with_capacity(tuples.len());
    for (operation, text) in tuples {
        let operation = match *operation {
            DMP_DELETE => Operation::Delete,
            DMP_INSERT => Operation::Insert,
            DMP_EQUAL => Operation::Equal,
            value => return Err(InvalidDmpOperation(value)),
        };
        diffs.push(Diff::new(operation, *text));
    }
    merge_diffs(&mut diffs);
    Ok(diffs)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidDmpOperation(pub i8);

impl fmt::Display for InvalidDmpOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Diff Match Patch operation {}", self.0)
    }
}

impl std::error::Error for InvalidDmpOperation {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkLimitReason {
    WorkBudget,
    DiffLimit,
    InputLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRun {
    pub diffs: Vec<Diff>,
    pub limited: Option<WorkLimitReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffOptions {
    pub max_work: u64,
    pub max_diffs: usize,
    pub max_input_units: usize,
    pub semantic_cleanup: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            max_work: DEFAULT_MAX_WORK,
            max_diffs: DEFAULT_MAX_DIFFS,
            max_input_units: DEFAULT_MAX_INPUT_UNITS,
            semantic_cleanup: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DiffEngine {
    options: DiffOptions,
}

impl Default for DiffEngine {
    fn default() -> Self {
        Self::new(DiffOptions::default())
    }
}

impl DiffEngine {
    pub fn new(options: DiffOptions) -> Self {
        Self { options }
    }

    pub fn options(&self) -> DiffOptions {
        self.options
    }

    pub fn diff(&self, old: &str, new: &str) -> Vec<Diff> {
        self.diff_with_status(old, new).diffs
    }

    pub fn diff_with_status(&self, old: &str, new: &str) -> DiffRun {
        let old_units: Vec<char> = old.chars().collect();
        let new_units: Vec<char> = new.chars().collect();

        if old_units.len() > self.options.max_input_units
            || new_units.len() > self.options.max_input_units
        {
            return DiffRun {
                diffs: vec![
                    Diff::new(Operation::Delete, old),
                    Diff::new(Operation::Insert, new),
                ],
                limited: Some(WorkLimitReason::InputLimit),
            };
        }

        if old_units == new_units {
            return DiffRun {
                diffs: vec_or_empty_equal(old),
                limited: None,
            };
        }

        let mut budget = Budget::new(self.options.max_work);
        let (prefix, suffix) = common_prefix_suffix(&old_units, &new_units);
        let old_mid_end = old_units.len() - suffix;
        let new_mid_end = new_units.len() - suffix;
        let old_mid = &old_units[prefix..old_mid_end];
        let new_mid = &new_units[prefix..new_mid_end];

        let mut body = if old_mid.is_empty() {
            vec![Diff::new(Operation::Insert, chars_to_string(new_mid))]
        } else if new_mid.is_empty() {
            vec![Diff::new(Operation::Delete, chars_to_string(old_mid))]
        } else {
            myers_diff(old_mid, new_mid, &mut budget).unwrap_or_else(|| {
                vec![
                    Diff::new(Operation::Delete, chars_to_string(old_mid)),
                    Diff::new(Operation::Insert, chars_to_string(new_mid)),
                ]
            })
        };

        let limited = budget.exhausted.then_some(WorkLimitReason::WorkBudget);

        if prefix > 0 {
            body.insert(0, Diff::new(Operation::Equal, chars_to_string(&old_units[..prefix])));
        }
        if suffix > 0 {
            body.push(Diff::new(
                Operation::Equal,
                chars_to_string(&old_units[old_mid_end..]),
            ));
        }

        merge_diffs(&mut body);
        if self.options.semantic_cleanup {
            semantic_cleanup(&mut body);
            merge_diffs(&mut body);
        }

        if body.len() > self.options.max_diffs {
            return DiffRun {
                diffs: vec![
                    Diff::new(Operation::Delete, old),
                    Diff::new(Operation::Insert, new),
                ],
                limited: Some(WorkLimitReason::DiffLimit),
            };
        }

        DiffRun {
            diffs: body,
            limited,
        }
    }
}

fn vec_or_empty_equal(s: &str) -> Vec<Diff> {
    if s.is_empty() {
        Vec::new()
    } else {
        vec![Diff::new(Operation::Equal, s)]
    }
}

#[derive(Debug)]
struct Budget {
    remaining: u64,
    exhausted: bool,
}

impl Budget {
    fn new(limit: u64) -> Self {
        Self {
            remaining: limit,
            exhausted: false,
        }
    }

    fn step(&mut self, amount: u64) -> bool {
        if self.remaining < amount {
            self.exhausted = true;
            false
        } else {
            self.remaining -= amount;
            true
        }
    }
}

fn common_prefix_suffix(a: &[char], b: &[char]) -> (usize, usize) {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let max_suffix = a.len().saturating_sub(prefix).min(b.len().saturating_sub(prefix));
    let mut suffix = 0;
    while suffix < max_suffix && a[a.len() - 1 - suffix] == b[b.len() - 1 - suffix] {
        suffix += 1;
    }
    (prefix, suffix)
}

fn chars_to_string(chars: &[char]) -> String {
    chars.iter().collect()
}

fn myers_diff(a: &[char], b: &[char], budget: &mut Budget) -> Option<Vec<Diff>> {
    let n = a.len() as isize;
    let m = b.len() as isize;
    let max_d = (n + m) as usize;
    let width = max_d.saturating_mul(2).saturating_add(3);
    let offset = max_d as isize + 1;

    let mut v = vec![0isize; width];
    v[(offset + 1) as usize] = 0;
    let mut trace: Vec<Vec<isize>> = Vec::with_capacity(max_d + 1);

    for d in 0..=max_d {
        if !budget.step(width as u64) {
            return None;
        }
        let mut next = vec![0isize; width];
        for k in (-(d as isize)..=(d as isize)).step_by(2) {
            let x = if k == -(d as isize)
                || (k != d as isize
                    && v[(k - 1 + offset) as usize] < v[(k + 1 + offset) as usize])
            {
                v[(k + 1 + offset) as usize]
            } else {
                v[(k - 1 + offset) as usize] + 1
            };

            let mut x = x;
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                if !budget.step(1) {
                    return None;
                }
                x += 1;
                y += 1;
            }

            next[(k + offset) as usize] = x;
            if x >= n && y >= m {
                trace.push(next);
                return backtrack(a, b, &trace, d);
            }
        }
        trace.push(next.clone());
        v = next;
    }
    None
}

fn backtrack(a: &[char], b: &[char], trace: &[Vec<isize>], final_d: usize) -> Option<Vec<Diff>> {
    let max_d = (trace.first()?.len().saturating_sub(3)) / 2;
    let offset = max_d as isize + 1;
    let mut x = a.len() as isize;
    let mut y = b.len() as isize;
    let mut reversed: Vec<(Operation, char)> = Vec::new();

    for d in (1..=final_d).rev() {
        let prev = &trace[d - 1];
        let k = x - y;
        let prev_k = if k == -(d as isize)
            || (k != d as isize
                && prev[(k - 1 + offset) as usize] < prev[(k + 1 + offset) as usize])
        {
            k + 1
        } else {
            k - 1
        };
        let prev_x = prev[(prev_k + offset) as usize];
        let prev_y = prev_x - prev_k;

        while x > prev_x && y > prev_y {
            reversed.push((Operation::Equal, a[(x - 1) as usize]));
            x -= 1;
            y -= 1;
        }

        if x == prev_x {
            if y <= 0 {
                return None;
            }
            reversed.push((Operation::Insert, b[(y - 1) as usize]));
            y -= 1;
        } else {
            if x <= 0 {
                return None;
            }
            reversed.push((Operation::Delete, a[(x - 1) as usize]));
            x -= 1;
        }
    }

    while x > 0 && y > 0 {
        reversed.push((Operation::Equal, a[(x - 1) as usize]));
        x -= 1;
        y -= 1;
    }
    while x > 0 {
        reversed.push((Operation::Delete, a[(x - 1) as usize]));
        x -= 1;
    }
    while y > 0 {
        reversed.push((Operation::Insert, b[(y - 1) as usize]));
        y -= 1;
    }

    reversed.reverse();
    let mut diffs: Vec<Diff> = Vec::new();
    for (op, ch) in reversed {
        match diffs.last_mut() {
            Some(last) if last.operation == op => last.text.push(ch),
            _ => diffs.push(Diff::new(op, ch.to_string())),
        }
    }
    Some(diffs)
}

fn merge_diffs(diffs: &mut Vec<Diff>) {
    let mut merged: Vec<Diff> = Vec::with_capacity(diffs.len());
    for diff in diffs.drain(..) {
        if diff.text.is_empty() {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if last.operation == diff.operation {
                last.text.push_str(&diff.text);
                continue;
            }
        }
        merged.push(diff);
    }
    *diffs = merged;
}

fn semantic_cleanup(diffs: &mut [Diff]) {
    if diffs.len() < 3 {
        return;
    }

    let mut i = 1;
    while i + 1 < diffs.len() {
        if diffs[i].operation == Operation::Delete
            && diffs[i + 1].operation == Operation::Insert
        {
            let delete = diffs[i].text.clone();
            let insert = diffs[i + 1].text.clone();

            let prefix = delete
                .chars()
                .zip(insert.chars())
                .take_while(|(a, b)| a == b)
                .count();

            if prefix > 0 {
                let prefix_text: String = delete.chars().take(prefix).collect();
                diffs[i - 1].text.push_str(&prefix_text);
                diffs[i].text = delete.chars().skip(prefix).collect();
                diffs[i + 1].text = insert.chars().skip(prefix).collect();
            }

            let dc: Vec<char> = diffs[i].text.chars().collect();
            let ic: Vec<char> = diffs[i + 1].text.chars().collect();
            let suffix = dc
                .iter()
                .rev()
                .zip(ic.iter().rev())
                .take_while(|(a, b)| a == b)
                .count();

            if suffix > 0 {
                let suffix_text: String = dc[dc.len() - suffix..].iter().collect();
                diffs[i].text = dc[..dc.len() - suffix].iter().collect();
                diffs[i + 1].text = ic[..ic.len() - suffix].iter().collect();
                diffs[i + 1].text.push_str(&suffix_text);
            }
        }
        i += 1;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MatchOptions {
    pub threshold: f64,
    pub distance: usize,
    pub max_work: u64,
    pub max_input_units: usize,
}

impl Default for MatchOptions {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_MATCH_THRESHOLD,
            distance: DEFAULT_MATCH_DISTANCE,
            max_work: DEFAULT_MAX_WORK,
            max_input_units: DEFAULT_MAX_INPUT_UNITS,
        }
    }
}

pub fn match_main(
    text: &str,
    pattern: &str,
    location: usize,
    options: MatchOptions,
) -> Option<usize> {
    if text.chars().count() > options.max_input_units
        || pattern.chars().count() > options.max_input_units
    {
        return None;
    }

    let text_units: Vec<char> = text.chars().collect();
    let pattern_units: Vec<char> = pattern.chars().collect();

    if pattern_units.is_empty() {
        return Some(location.min(text_units.len()));
    }
    if text_units.is_empty() || pattern_units.len() > text_units.len() {
        return None;
    }

    if let Some(exact) = find_subsequence(&text_units, &pattern_units, location) {
        return Some(exact);
    }

    let start = location.saturating_sub(options.distance);
    let end = (location + options.distance).min(text_units.len());
    let mut best: Option<(usize, f64)> = None;
    let mut budget = Budget::new(options.max_work);

    for candidate in start..=end {
        if candidate + pattern_units.len() > text_units.len() {
            break;
        }
        let score = levenshtein(
            &text_units[candidate..candidate + pattern_units.len()],
            &pattern_units,
            &mut budget,
        )? as f64
            / pattern_units.len() as f64;

        if score <= options.threshold {
            match best {
                Some((best_pos, best_score))
                    if best_score < score
                        || (best_score == score
                            && best_pos.abs_diff(location) <= candidate.abs_diff(location)) => {}
                _ => best = Some((candidate, score)),
            }
        }
        if budget.exhausted {
            return None;
        }
    }

    best.map(|(pos, _)| pos)
}

fn find_subsequence(text: &[char], pattern: &[char], location: usize) -> Option<usize> {
    let location = location.min(text.len());
    text[location..]
        .windows(pattern.len())
        .position(|w| w == pattern)
        .map(|p| p + location)
        .or_else(|| text[..location].windows(pattern.len()).position(|w| w == pattern))
}

fn levenshtein(a: &[char], b: &[char], budget: &mut Budget) -> Option<usize> {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];

    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            if !budget.step(1) {
                return None;
            }
            let cost = usize::from(ca != cb);
            curr[j + 1] = (curr[j] + 1)
                .min(prev[j + 1] + 1)
                .min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    Some(prev[b.len()])
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Patch {
    pub old_start: usize,
    pub old_len: usize,
    pub new_start: usize,
    pub new_len: usize,
    pub diffs: Vec<Diff>,
}

impl Patch {
    pub fn new(
        old_start: usize,
        old_len: usize,
        new_start: usize,
        new_len: usize,
        diffs: Vec<Diff>,
    ) -> Self {
        Self {
            old_start,
            old_len,
            new_start,
            new_len,
            diffs,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatchOptions {
    pub context: usize,
    pub merge_distance: usize,
}

impl Default for PatchOptions {
    fn default() -> Self {
        Self {
            context: PATCH_CONTEXT,
            merge_distance: PATCH_MERGE_DISTANCE,
        }
    }
}

pub fn patch_make(old: &str, new: &str, options: DiffOptions) -> Vec<Patch> {
    patch_make_with_options(old, new, PatchOptions::default(), options)
}

pub fn patch_make_with_options(
    old: &str,
    new: &str,
    patch_options: PatchOptions,
    diff_options: DiffOptions,
) -> Vec<Patch> {
    let old_len = old.chars().count();
    let new_len = new.chars().count();
    if old_len > diff_options.max_input_units || new_len > diff_options.max_input_units {
        return vec![Patch::new(
            0,
            old_len,
            0,
            new_len,
            vec![
                Diff::new(Operation::Delete, old),
                Diff::new(Operation::Insert, new),
            ],
        )];
    }

    let diffs = DiffEngine::new(diff_options).diff(old, new);
    let old_units: Vec<char> = old.chars().collect();
    let new_units: Vec<char> = new.chars().collect();

    let mut changes = Vec::<(usize, usize, usize, usize)>::new();
    let mut old_pos = 0usize;
    let mut new_pos = 0usize;
    let mut in_change = false;
    let mut change_start_old = 0usize;
    let mut change_start_new = 0usize;
    let mut change_end_old = 0usize;
    let mut change_end_new = 0usize;

    for diff in &diffs {
        match diff.operation {
            Operation::Equal => {
                let n = diff.text.chars().count();
                if in_change {
                    changes.push((
                        change_start_old,
                        change_end_old,
                        change_start_new,
                        change_end_new,
                    ));
                    in_change = false;
                }
                old_pos += n;
                new_pos += n;
            }
            Operation::Delete => {
                let n = diff.text.chars().count();
                if !in_change {
                    in_change = true;
                    change_start_old = old_pos;
                    change_start_new = new_pos;
                }
                old_pos += n;
                change_end_old = old_pos;
                change_end_new = new_pos;
            }
            Operation::Insert => {
                let n = diff.text.chars().count();
                if !in_change {
                    in_change = true;
                    change_start_old = old_pos;
                    change_start_new = new_pos;
                }
                new_pos += n;
                change_end_old = old_pos;
                change_end_new = new_pos;
            }
        }
    }
    if in_change {
        changes.push((
            change_start_old,
            change_end_old,
            change_start_new,
            change_end_new,
        ));
    }

    if changes.is_empty() {
        return Vec::new();
    }

    let mut patches = Vec::new();
    for (start_old, end_old, start_new, end_new) in changes {
        let context_start_old = start_old.saturating_sub(patch_options.context);
        let context_end_old = (end_old + patch_options.context).min(old_units.len());
        let context_start_new = start_new.saturating_sub(patch_options.context);
        let context_end_new = (end_new + patch_options.context).min(new_units.len());

        let old_part = chars_to_string(&old_units[context_start_old..context_end_old]);
        let new_part = chars_to_string(&new_units[context_start_new..context_end_new]);
        let sub_diffs = DiffEngine::new(diff_options).diff(&old_part, &new_part);

        patches.push(Patch::new(
            context_start_old,
            context_end_old - context_start_old,
            context_start_new,
            context_end_new - context_start_new,
            sub_diffs,
        ));
    }

    merge_patches(
        patches,
        patch_options.merge_distance,
        &old_units,
        &new_units,
        diff_options,
    )
}


fn merge_patches(
    mut patches: Vec<Patch>,
    distance: usize,
    old_units: &[char],
    new_units: &[char],
    diff_options: DiffOptions,
) -> Vec<Patch> {
    if patches.len() < 2 {
        return patches;
    }

    let mut merged: Vec<Patch> = Vec::with_capacity(patches.len());
    for patch in patches.drain(..) {
        if let Some(last) = merged.last() {
            let old_end = last.old_start + last.old_len;
            if patch.old_start <= old_end + distance {
                let old_start = last.old_start.min(patch.old_start);
                let old_end = old_end.max(patch.old_start + patch.old_len);
                let new_start = last.new_start.min(patch.new_start);
                let new_end = (last.new_start + last.new_len)
                    .max(patch.new_start + patch.new_len);
                let old_part = chars_to_string(&old_units[old_start..old_end]);
                let new_part = chars_to_string(&new_units[new_start..new_end]);
                let diffs = DiffEngine::new(diff_options).diff(&old_part, &new_part);
                let replacement = Patch::new(
                    old_start,
                    old_end - old_start,
                    new_start,
                    new_end - new_start,
                    diffs,
                );
                let _ = merged.pop();
                merged.push(replacement);
                continue;
            }
        }
        merged.push(patch);
    }
    merged
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatchParseOptions {
    pub max_input_bytes: usize,
    pub max_patches: usize,
    pub max_line_bytes: usize,
}

impl Default for PatchParseOptions {
    fn default() -> Self {
        Self {
            max_input_bytes: 16 * 1024 * 1024,
            max_patches: 10_000,
            max_line_bytes: 1024 * 1024,
        }
    }
}

#[derive(Debug)]
pub enum PatchParseError {
    InvalidHeader(String),
    InvalidDiffLine(String),
    InvalidPercentEncoding,
    InvalidLength,
    LimitExceeded,
}

impl fmt::Display for PatchParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader(value) => write!(f, "invalid patch header: {value}"),
            Self::InvalidDiffLine(value) => write!(f, "invalid patch diff line: {value}"),
            Self::InvalidPercentEncoding => write!(f, "invalid percent encoding"),
            Self::InvalidLength => write!(f, "patch length does not match encoded content"),
            Self::LimitExceeded => write!(f, "patch input exceeded configured resource limits"),
        }
    }
}

impl std::error::Error for PatchParseError {}

pub fn patch_to_text(patches: &[Patch]) -> String {
    let mut out = String::new();
    for patch in patches {
        out.push_str("@@ -");
        out.push_str(&header_position(patch.old_start, patch.old_len));
        out.push_str(" +");
        out.push_str(&header_position(patch.new_start, patch.new_len));
        out.push_str(" @@\n");
        for diff in &patch.diffs {
            let prefix = match diff.operation {
                Operation::Delete => '-',
                Operation::Insert => '+',
                Operation::Equal => ' ',
            };
            out.push(prefix);
            out.push_str(&percent_encode(&diff.text));
            out.push('\n');
        }
    }
    out
}

fn header_position(start: usize, len: usize) -> String {
    if len == 0 {
        start.to_string()
    } else {
        format!("{},{}", start + 1, len)
    }
}

pub fn patch_from_text(input: &str) -> Result<Vec<Patch>, PatchParseError> {
    patch_from_text_with_options(input, PatchParseOptions::default())
}

pub fn patch_from_text_with_options(
    input: &str,
    options: PatchParseOptions,
) -> Result<Vec<Patch>, PatchParseError> {
    if input.len() > options.max_input_bytes {
        return Err(PatchParseError::LimitExceeded);
    }

    let mut patches = Vec::new();
    let mut current: Option<Patch> = None;

    for raw in input.lines() {
        let line = raw.trim_end_matches('\r');
        if line.len() > options.max_line_bytes {
            return Err(PatchParseError::LimitExceeded);
        }
        if line.starts_with("@@ ") {
            if let Some(patch) = current.take() {
                patches.push(patch);
                if patches.len() > options.max_patches {
                    return Err(PatchParseError::LimitExceeded);
                }
            }
            current = Some(parse_header(line)?);
            continue;
        }
        if line.is_empty() {
            continue;
        }

        let patch = current
            .as_mut()
            .ok_or_else(|| PatchParseError::InvalidDiffLine(line.to_string()))?;
        let (prefix, payload) = line.split_at(1);
        let operation = match prefix {
            "-" => Operation::Delete,
            "+" => Operation::Insert,
            " " => Operation::Equal,
            _ => return Err(PatchParseError::InvalidDiffLine(line.to_string())),
        };
        patch.diffs.push(Diff::new(operation, percent_decode(payload)?));
    }

    if let Some(patch) = current.take() {
        patches.push(patch);
        if patches.len() > options.max_patches {
            return Err(PatchParseError::LimitExceeded);
        }
    }

    for patch in &patches {
        let mut old_len = 0usize;
        let mut new_len = 0usize;
        for diff in &patch.diffs {
            let n = diff.text.chars().count();
            match diff.operation {
                Operation::Delete => old_len += n,
                Operation::Insert => new_len += n,
                Operation::Equal => {
                    old_len += n;
                    new_len += n;
                }
            }
        }
        if old_len != patch.old_len || new_len != patch.new_len {
            return Err(PatchParseError::InvalidLength);
        }
    }

    Ok(patches)
}

fn parse_header(line: &str) -> Result<Patch, PatchParseError> {
    let body = line
        .strip_prefix("@@ -")
        .and_then(|value| value.strip_suffix(" @@"))
        .ok_or_else(|| PatchParseError::InvalidHeader(line.to_string()))?;
    let (old_range, new_range) = body
        .split_once(" +")
        .ok_or_else(|| PatchParseError::InvalidHeader(line.to_string()))?;
    let (old_start, old_len) = parse_header_range(old_range)?;
    let (new_start, new_len) = parse_header_range(new_range)?;
    Ok(Patch::new(
        old_start,
        old_len,
        new_start,
        new_len,
        Vec::new(),
    ))
}

fn parse_header_range(s: &str) -> Result<(usize, usize), PatchParseError> {
    let (start_text, len_text) = s
        .split_once(',')
        .map_or((s, None), |(start, len)| (start, Some(len)));

    let start = start_text
        .parse::<usize>()
        .map_err(|_| PatchParseError::InvalidHeader(s.to_string()))?;

    match len_text {
        None if start == 0 => Ok((0, 0)),
        None => Ok((start - 1, 1)),
        Some("0") => Ok((start, 0)),
        Some(len) => {
            let len = len
                .parse::<usize>()
                .map_err(|_| PatchParseError::InvalidHeader(s.to_string()))?;
            Ok((start.saturating_sub(1), len))
        }
    }
}

fn percent_encode(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(&mut out, "%{:02X}", byte);
            }
        }
    }
    out
}

fn percent_decode(input: &str) -> Result<String, PatchParseError> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(PatchParseError::InvalidPercentEncoding);
            }
            let hi = hex(bytes[i + 1]).ok_or(PatchParseError::InvalidPercentEncoding)?;
            let lo = hex(bytes[i + 2]).ok_or(PatchParseError::InvalidPercentEncoding)?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| PatchParseError::InvalidPercentEncoding)
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyResult {
    pub text: String,
    pub applied: Vec<bool>,
}

#[derive(Debug)]
pub enum PatchApplyError {
    SearchLimit,
}

impl fmt::Display for PatchApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SearchLimit => write!(f, "patch matching exceeded configured work budget"),
        }
    }
}

impl std::error::Error for PatchApplyError {}

pub fn patch_apply(
    patches: &[Patch],
    text: &str,
    match_options: MatchOptions,
) -> Result<ApplyResult, PatchApplyError> {
    if patches.len() > 10_000 {
        return Err(PatchApplyError::SearchLimit);
    }
    let mut current = text.to_owned();
    let mut applied = Vec::with_capacity(patches.len());
    let mut delta: i128 = 0;

    for patch in patches {
        let old_text: String = patch
            .diffs
            .iter()
            .filter(|diff| diff.operation != Operation::Insert)
            .map(|diff| diff.text.as_str())
            .collect();
        let new_text: String = patch
            .diffs
            .iter()
            .filter(|diff| diff.operation != Operation::Delete)
            .map(|diff| diff.text.as_str())
            .collect();

        let expected = ((patch.old_start as i128) + delta)
            .max(0)
            .min(current.chars().count() as i128) as usize;
        let found = if old_text.is_empty() {
            Some(expected)
        } else {
            match_main(&current, &old_text, expected, match_options)
        };

        match found {
            Some(position) => {
                let mut chars: Vec<char> = current.chars().collect();
                let start = position.min(chars.len());
                let end = start
                    .saturating_add(old_text.chars().count())
                    .min(chars.len());
                chars.splice(start..end, new_text.chars());
                current = chars.into_iter().collect();
                delta += patch.new_len as i128 - patch.old_len as i128;
                applied.push(true);
            }
            None => applied.push(false),
        }
    }

    Ok(ApplyResult {
        text: current,
        applied,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reconstruct(diffs: &[Diff], old: &str) -> String {
        let mut out = String::new();
        let mut cursor = 0usize;
        let old_chars: Vec<char> = old.chars().collect();
        for diff in diffs {
            match diff.operation {
                Operation::Equal => {
                    out.push_str(&diff.text);
                    cursor += diff.text.chars().count();
                }
                Operation::Delete => cursor += diff.text.chars().count(),
                Operation::Insert => out.push_str(&diff.text),
            }
        }
        assert_eq!(cursor, old_chars.len());
        out
    }

    #[test]
    fn diff_reconstructs_target() {
        let old = "The quick brown fox.";
        let new = "The fast brown fox!";
        let diffs = DiffEngine::default().diff(old, new);
        assert_eq!(reconstruct(&diffs, old), new);
    }

    #[test]
    fn empty_inputs() {
        assert!(DiffEngine::default().diff("", "").is_empty());
        assert_eq!(
            DiffEngine::default().diff("", "abc"),
            vec![Diff::new(Operation::Insert, "abc")]
        );
        assert_eq!(
            DiffEngine::default().diff("abc", ""),
            vec![Diff::new(Operation::Delete, "abc")]
        );
    }

    #[test]
    fn unicode_is_scalar_safe() {
        let old = "zażółć 😀";
        let new = "zażółć 😎";
        let diffs = DiffEngine::default().diff(old, new);
        assert_eq!(reconstruct(&diffs, old), new);
    }

    #[test]
    fn match_exact_and_approximate() {
        let options = MatchOptions::default();
        assert_eq!(
            match_main("hello brave world", "brave", 0, options),
            Some(6)
        );
        assert_eq!(
            match_main("hello brve world", "brave", 6, options),
            Some(6)
        );
    }

    #[test]
    fn patch_roundtrip() {
        let old = "alpha beta gamma";
        let new = "alpha brave beta gamma!";
        let patches = patch_make(old, new, DiffOptions::default());
        let text = patch_to_text(&patches);
        let parsed = patch_from_text(&text).unwrap();
        let result = patch_apply(&parsed, old, MatchOptions::default()).unwrap();
        assert!(result.applied.iter().all(|x| *x));
        assert_eq!(result.text, new);
    }

    #[test]
    fn dmp_tuple_adapter_roundtrips() {
        let diffs = DiffEngine::default().diff("abc", "axc");
        let tuples = diffs_to_dmp_tuples(&diffs);
        assert_eq!(
            tuples.iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            vec![DMP_EQUAL, DMP_DELETE, DMP_INSERT, DMP_EQUAL]
        );
        let refs: Vec<(i8, &str)> =
            tuples.iter().map(|(op, text)| (*op, text.as_str())).collect();
        assert_eq!(diffs_from_dmp_tuples(&refs).unwrap(), diffs);
        assert!(diffs_from_dmp_tuples(&[(7, "x")]).is_err());
    }

    #[test]
    fn standard_ascii_patch_vector_matches_format() {
        let patches = patch_make("abc", "axc", DiffOptions::default());
        assert_eq!(
            patch_to_text(&patches),
            "@@ -1,3 +1,3 @@\n a\n-b\n+x\n c\n"
        );
    }

    #[test]
    fn patch_parser_rejects_invalid_lengths() {
        let bad = "@@ -1,3 +1,3 @@\n ab\n";
        assert!(matches!(
            patch_from_text(bad),
            Err(PatchParseError::InvalidLength)
        ));
    }

    #[test]
    fn parser_empty_range_is_zero_length() {
        let parsed = patch_from_text("@@ -0 +1,3 @@\n+abc\n").unwrap();
        assert_eq!(parsed[0].old_start, 0);
        assert_eq!(parsed[0].old_len, 0);
    }

    #[test]
    fn patch_parser_enforces_limits() {
        let options = PatchParseOptions {
            max_input_bytes: 4,
            ..PatchParseOptions::default()
        };
        assert!(matches!(
            patch_from_text_with_options("@@ -0 +1 @@\n+a\n", options),
            Err(PatchParseError::LimitExceeded)
        ));
    }

    #[test]
    fn match_input_limit_is_safe() {
        let options = MatchOptions {
            max_input_units: 2,
            ..MatchOptions::default()
        };
        assert_eq!(match_main("abc", "a", 0, options), None);
    }

    #[test]
    fn parser_line_limit_is_safe() {
        let options = PatchParseOptions {
            max_line_bytes: 2,
            ..PatchParseOptions::default()
        };
        assert!(matches!(
            patch_from_text_with_options("@@ -1 +1 @@\n+a\n", options),
            Err(PatchParseError::LimitExceeded)
        ));
    }

    #[test]
    fn work_budget_is_safe() {
        let options = DiffOptions {
            max_work: 1,
            ..DiffOptions::default()
        };
        let run = DiffEngine::new(options).diff_with_status("abcdefgh", "ijklmnop");
        assert!(run.limited.is_some());
        assert_eq!(reconstruct(&run.diffs, "abcdefgh"), "ijklmnop");
    }

    #[test]
    fn input_limit_is_safe() {
        let options = DiffOptions {
            max_input_units: 2,
            ..DiffOptions::default()
        };
        let run = DiffEngine::new(options).diff_with_status("abc", "xyz");
        assert_eq!(run.limited, Some(WorkLimitReason::InputLimit));
        assert_eq!(reconstruct(&run.diffs, "abc"), "xyz");
    }

    #[test]
    fn insertion_and_deletion_patches_roundtrip() {
        for (old, new) in [("", "abc"), ("abc", ""), ("abc", "abXYZc")] {
            let patches = patch_make(old, new, DiffOptions::default());
            let encoded = patch_to_text(&patches);
            let parsed = patch_from_text(&encoded).unwrap();
            let applied = patch_apply(&parsed, old, MatchOptions::default()).unwrap();
            assert!(applied.applied.iter().all(|value| *value));
            assert_eq!(applied.text, new);
        }
    }

    #[test]
    fn multiple_distant_patches_apply_in_order() {
        let old = "AAAAA middle text that is deliberately long BBBBB";
        let new = "ZZZZZ middle text that is deliberately long YYYYY";
        let patches = patch_make(old, new, DiffOptions::default());
        assert!(patches.len() >= 2);
        let applied = patch_apply(&patches, old, MatchOptions::default()).unwrap();
        assert!(applied.applied.iter().all(|value| *value));
        assert_eq!(applied.text, new);
    }

    #[test]
    fn exhaustive_small_roundtrips() {
        let alphabet = ['a', 'b'];
        for len_a in 0..=4 {
            for len_b in 0..=4 {
                let mut as_ = Vec::new();
                let mut bs_ = Vec::new();
                gen_strings(&alphabet, len_a, &mut as_, &mut Vec::new());
                gen_strings(&alphabet, len_b, &mut bs_, &mut Vec::new());
                for a in &as_ {
                    for b in &bs_ {
                        let old: String = a.iter().collect();
                        let new: String = b.iter().collect();
                        let diffs = DiffEngine::default().diff(&old, &new);
                        assert_eq!(reconstruct(&diffs, &old), new);
                        let patches = patch_make(&old, &new, DiffOptions::default());
                        let applied =
                            patch_apply(&patches, &old, MatchOptions::default()).unwrap();
                        assert_eq!(applied.text, new);
                    }
                }
            }
        }
    }

    fn gen_strings(
        alphabet: &[char],
        len: usize,
        output: &mut Vec<Vec<char>>,
        current: &mut Vec<char>,
    ) {
        if current.len() == len {
            output.push(current.clone());
            return;
        }
        for ch in alphabet {
            current.push(*ch);
            gen_strings(alphabet, len, output, current);
            current.pop();
        }
    }
}
