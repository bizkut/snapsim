//! Check visible SNAP turn resolutions in JSONL recordings.
use snapsim::replay::{
    check_file, check_file_with_coverage, coverage::CoverageReport, report::Report,
};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str =
    "usage: snapsim-replaycheck [-h] [--seeds N] [--coverage-json PATH] FILE [FILE ...]";

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{USAGE}\nsnapsim-replaycheck: error: {message}");
    ExitCode::from(2)
}

fn repr(value: &str) -> String {
    let quote = if value.contains('\'') && !value.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut result = String::new();
    result.push(quote);
    for c in value.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c == quote => {
                result.push('\\');
                result.push(c);
            }
            c if c.is_control() && (c as u32) <= 0xff => {
                use std::fmt::Write as _;
                write!(&mut result, "\\x{:02x}", c as u32).expect("write to string");
            }
            c => result.push(c),
        }
    }
    result.push(quote);
    result
}

// argparse recognizes negative decimal-looking values as arguments, not options.
fn negative_number(text: &str) -> bool {
    let Some(value) = text.strip_prefix('-') else {
        return false;
    };
    let mut parts = value.split('.');
    let integer = parts.next().unwrap_or("");
    let fraction = parts.next();
    parts.next().is_none()
        && integer.chars().all(|c| c.is_ascii_digit())
        && fraction.is_none_or(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()))
        && (!integer.is_empty() || fraction.is_some())
}

fn path_display(path: &Path) -> String {
    // pathlib.Path normalizes redundant separators and '.' components in labels.
    let mut result = PathBuf::new();
    for component in path.components() {
        if component != Component::CurDir {
            result.push(component);
        }
    }
    if result.as_os_str().is_empty() {
        ".".into()
    } else {
        result.display().to_string()
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let mut files = Vec::new();
    let mut seeds = 128_i128;
    let mut coverage_path: Option<PathBuf> = None;
    let mut positional_only = false;
    let mut positional_closed = false;
    let mut unknown = Vec::new();
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy();
        if !positional_only && text == "--" {
            positional_only = true;
            continue;
        }
        if !positional_only && (text == "-h" || text == "--help") {
            println!(
                "{USAGE}\n\nCheck visible SNAP turn resolutions.\n\npositional arguments:\n  FILE\n\noptions:\n  -h, --help           show this help message and exit\n  --seeds N            number of simulation seeds\n  --coverage-json PATH write per-definition evidence as JSON"
            );
            return ExitCode::SUCCESS;
        }
        let mut parts = text.splitn(2, '=');
        let name = parts.next().unwrap_or("");
        if !positional_only && name == "--coverage-json" {
            positional_closed |= !files.is_empty();
            let value = match parts.next() {
                Some(value) => PathBuf::from(value),
                None => match args.next() {
                    Some(value) => PathBuf::from(value),
                    None => return usage_error("argument --coverage-json: expected one argument"),
                },
            };
            coverage_path = Some(value);
            continue;
        }
        let seed_flag = !positional_only && name.starts_with("--s") && "--seeds".starts_with(name);
        if seed_flag {
            positional_closed |= !files.is_empty();
            let value = match parts.next() {
                Some(value) => value.to_owned(),
                None => match args.next() {
                    Some(value) => {
                        let value = value.to_string_lossy().into_owned();
                        if value.starts_with('-') && value != "-" && !negative_number(&value) {
                            return usage_error("argument --seeds: expected one argument");
                        }
                        value
                    }
                    None => return usage_error("argument --seeds: expected one argument"),
                },
            };
            match value.trim().parse::<i128>() {
                Ok(value) => seeds = value,
                Err(_) => {
                    return usage_error(&format!(
                        "argument --seeds: invalid int value: {}",
                        repr(&value)
                    ));
                }
            }
        } else if !positional_only
            && text.starts_with('-')
            && text != "-"
            && !negative_number(&text)
        {
            positional_closed |= !files.is_empty();
            unknown.push(text.into_owned());
        } else if positional_closed {
            unknown.push(text.into_owned());
        } else {
            files.push(PathBuf::from(arg));
        }
    }
    if files.is_empty() {
        return usage_error("the following arguments are required: FILE");
    }
    if !unknown.is_empty() {
        return usage_error(&format!("unrecognized arguments: {}", unknown.join(" ")));
    }
    if seeds < 1 {
        return usage_error("--seeds must be positive");
    }
    let seeds = match usize::try_from(seeds) {
        Ok(value) => value,
        Err(_) => return usage_error("--seeds exceeds the supported range"),
    };
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut report = Report::default();
    let mut coverage = coverage_path.as_ref().map(|_| CoverageReport::default());
    let mut rejected = false;
    for path in files {
        let label = path_display(&path);
        let rows = match if coverage.is_some() {
            check_file_with_coverage(&path, seeds)
        } else {
            check_file(&path, seeds)
        } {
            Ok(rows) => rows,
            Err(error) => {
                eprintln!("{label}: {error}");
                rejected = true;
                continue;
            }
        };
        if let Some(coverage) = &mut coverage {
            coverage.add(&rows);
        }
        if let Err(error) = report
            .file(&mut output, &label, &rows)
            .and_then(|()| output.flush())
        {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    }
    if let Err(error) = report.finish(&mut output) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    if let Some((path, coverage)) = coverage_path.zip(coverage) {
        let result = std::fs::File::create(path).and_then(|file| {
            serde_json::to_writer_pretty(file, &coverage).map_err(io::Error::other)
        });
        if let Err(error) = result {
            eprintln!("writing coverage: {error}");
            return ExitCode::FAILURE;
        }
    }
    if rejected {
        return ExitCode::from(2);
    }
    if report.has_inconsistent() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
