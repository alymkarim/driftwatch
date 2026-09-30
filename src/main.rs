use std::path::PathBuf;
use std::process::ExitCode;

use driftwatch::report::run;
use driftwatch::statistics::DEFAULT_BINS;

const USAGE: &str = "\
driftwatch — population stability checks for a scored population

USAGE:
    driftwatch <BASELINE_CSV> <CURRENT_CSV> [--bins N]

ARGS:
    <BASELINE_CSV>    reference population the model was validated on
    <CURRENT_CSV>     population scored since, same columns

OPTIONS:
    -b, --bins <N>    quantile bins per feature [default: 10]
    -h, --help        print this help
";

struct Args {
    baseline: PathBuf,
    current: PathBuf,
    bins: usize,
}

fn parse_args() -> Result<Args, String> {
    let mut positional = Vec::new();
    let mut bins = DEFAULT_BINS;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "-b" | "--bins" => {
                let value = args.next().ok_or("--bins needs a value")?;
                bins = value
                    .parse()
                    .map_err(|_| format!("--bins expects a number, got '{value}'"))?;
                if bins < 2 {
                    return Err("--bins must be at least 2".to_owned());
                }
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option '{other}'"));
            }
            other => positional.push(other.to_owned()),
        }
    }

    if positional.len() != 2 {
        return Err(format!(
            "expected 2 csv paths, got {}\n\n{USAGE}",
            positional.len()
        ));
    }

    Ok(Args {
        baseline: PathBuf::from(&positional[0]),
        current: PathBuf::from(&positional[1]),
        bins,
    })
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    match run(&args.baseline, &args.current, args.bins) {
        Ok(report) => {
            print!("{report}");
            // Non zero when anything needs review so this can sit in a pipeline
            // and hold a promotion.
            if report.needs_review() {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
