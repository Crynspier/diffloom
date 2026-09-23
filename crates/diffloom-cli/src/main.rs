use diffloom_core::{
    match_main, patch_apply, patch_from_text, patch_make, patch_to_text, DiffEngine, DiffOptions,
    MatchOptions, Operation,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn usage() {
    println!(
        "Diffloom 0.1.0

Commands:
  diff OLD_FILE NEW_FILE
  patch make OLD_FILE NEW_FILE
  patch apply PATCH_FILE INPUT_FILE OUTPUT_FILE
  match TEXT PATTERN [LOCATION]
  version"
    );
}

fn print_diffs(diffs: &[diffloom_core::Diff]) {
    if diffs.is_empty() {
        println!("(no changes)");
        return;
    }

    for diff in diffs {
        let prefix = match diff.operation {
            Operation::Delete => '-',
            Operation::Insert => '+',
            Operation::Equal => ' ',
        };
        println!("{prefix}{}", diff.text);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = match args.next() {
        Some(value) => value,
        None => {
            usage();
            return Ok(());
        }
    };

    match command.as_str() {
        "diff" => {
            let old_path = args.next().ok_or_else(|| "missing OLD_FILE".to_string())?;
            let new_path = args.next().ok_or_else(|| "missing NEW_FILE".to_string())?;
            let old = fs::read_to_string(&old_path).map_err(|e| format!("{old_path}: {e}"))?;
            let new = fs::read_to_string(&new_path).map_err(|e| format!("{new_path}: {e}"))?;
            let result = DiffEngine::new(DiffOptions::default()).diff_with_status(&old, &new);
            print_diffs(&result.diffs);
            if result.limited.is_some() {
                eprintln!("diffloom: resource limit reached; using a safe coarse replacement");
            }
            Ok(())
        }
        "patch" => match args.next().as_deref() {
            Some("make") => {
                let old_path = args.next().ok_or_else(|| "missing OLD_FILE".to_string())?;
                let new_path = args.next().ok_or_else(|| "missing NEW_FILE".to_string())?;
                let old = fs::read_to_string(&old_path).map_err(|e| format!("{old_path}: {e}"))?;
                let new = fs::read_to_string(&new_path).map_err(|e| format!("{new_path}: {e}"))?;
                let patches = patch_make(&old, &new, DiffOptions::default());
                print!("{}", patch_to_text(&patches));
                Ok(())
            }
            Some("apply") => {
                let patch_path = args.next().ok_or_else(|| "missing PATCH_FILE".to_string())?;
                let input_path = args.next().ok_or_else(|| "missing INPUT_FILE".to_string())?;
                let output_path = args.next().ok_or_else(|| "missing OUTPUT_FILE".to_string())?;
                let patch = fs::read_to_string(&patch_path).map_err(|e| format!("{patch_path}: {e}"))?;
                let input = fs::read_to_string(&input_path).map_err(|e| format!("{input_path}: {e}"))?;
                let parsed = patch_from_text(&patch).map_err(|e| e.to_string())?;
                let result = patch_apply(&parsed, &input, MatchOptions::default())
                    .map_err(|e| e.to_string())?;
                if result.applied.iter().any(|value| !value) {
                    return Err("one or more patches did not apply".to_string());
                }
                fs::write(&output_path, result.text).map_err(|e| format!("{output_path}: {e}"))?;
                Ok(())
            }
            _ => Err("expected patch make or patch apply".to_string()),
        },
        "match" => {
            let text = args.next().ok_or_else(|| "missing TEXT".to_string())?;
            let pattern = args.next().ok_or_else(|| "missing PATTERN".to_string())?;
            let location = args
                .next()
                .map(|value| value.parse::<usize>().map_err(|_| "invalid LOCATION".to_string()))
                .transpose()?
                .unwrap_or(0);
            match match_main(&text, &pattern, location, MatchOptions::default()) {
                Some(position) => println!("{position}"),
                None => println!("-1"),
            }
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("diffloom 0.1.0");
            Ok(())
        }
        "help" | "--help" | "-h" => {
            usage();
            Ok(())
        }
        _ => Err(format!("unknown command: {command}")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("diffloom: {error}");
            ExitCode::from(2)
        }
    }
}
