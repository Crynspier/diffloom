use diffloom_core::{DiffEngine, DiffOptions};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let old = "alpha beta gamma ".repeat(200);
    let new = "alpha beta delta ".repeat(200);
    let engine = DiffEngine::new(DiffOptions::default());

    let start = Instant::now();
    let mut count = 0usize;
    for _ in 0..50 {
        let diffs = engine.diff(black_box(&old), black_box(&new));
        count += diffs.len();
    }

    println!(
        "50 iterations: {:?}, total diff records: {}",
        start.elapsed(),
        count
    );
}
