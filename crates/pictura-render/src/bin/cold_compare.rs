//! Compare two cold-composite report cohorts.
//!
//! Usage: `cold_compare <baseline-file-or-dir> <candidate-file-or-dir>`.
//! Exits 0 improved, 2 regressed, 3 inconclusive, 4 insufficient, 5 invalid,
//! and 1 on usage or I/O failure.

use std::path::Path;
use std::process::ExitCode;

use pictura_render::cold_profile::{compare, Report};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("usage: cold_compare <baseline-file-or-dir> <candidate-file-or-dir>");
        return ExitCode::from(1);
    }
    let baseline = match load(Path::new(&args[0])) {
        Ok(reports) => reports,
        Err(error) => {
            eprintln!("cold_compare: {error}");
            return ExitCode::from(1);
        }
    };
    let candidate = match load(Path::new(&args[1])) {
        Ok(reports) => reports,
        Err(error) => {
            eprintln!("cold_compare: {error}");
            return ExitCode::from(1);
        }
    };
    let comparison = compare(&baseline, &candidate);
    print!("{}", comparison.render());
    ExitCode::from(comparison.outcome.exit_code() as u8)
}

fn load(path: &Path) -> Result<Vec<Report>, String> {
    if path.is_dir() {
        let mut files: Vec<_> = std::fs::read_dir(path)
            .map_err(|error| format!("read {}: {error}", path.display()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "report"))
            .collect();
        files.sort();
        if files.is_empty() {
            return Err(format!("no .report files in {}", path.display()));
        }
        files.iter().map(|file| Report::read(file)).collect()
    } else {
        Report::read(path).map(|report| vec![report])
    }
}
