# driftwatch

A small tool to check if the distribution of scored features has shifted between a baseline population and a current one. It computes Population Stability Index (PSI) and the two sample Kolmogorov Smirnov statistic (KS) per feature, flags which ones need a review, and can be used to gate a model promotion in CI.

## Why

When a model is validated on a baseline set, the people or cases it scores later tend to look different over time. This change, known as population drift, can erode model reliability without any code change. Driftwatch helps spot that early by comparing two CSV files before pushing a new model or continuing to score new data.

## What it does

* Reads two CSV files (baseline and current) with numeric features
* Computes PSI and KS for every column that exists in the baseline
* Ranks the largest shift and gives a clear verdict per feature (stable, moderate, or significant)
* Exits with a non zero code when any feature shows significant drift so CI can block a promotion

## Installation

Build from source with Cargo:

```bash
cargo build --release
```

The binary will be at `./target/release/driftwatch`.

## Usage

```bash
driftwatch <baseline.csv> <current.csv> [--bins <N>]
```

Example:

```bash
driftwatch baseline.csv current.csv --bins 10
```

## Output

A simple table like:

```
FEATURE                 PSI     KS      VERDICT
---------------------  ------  ------  ---------
loan_amount            1.329   0.450   significant
debt_to_income         7.142   0.770   significant
credit_score_months    3.841   0.710   significant
utilisation            9.512   0.875   significant

largest shift: utilisation at psi 9.5123
```

## Exit codes

Exit code 0 means no significant drift. Exit code 2 means at least one feature shows significant drift, so review before promotion. Exit code 1 means an input error such as a missing file, bad CSV, or non numeric values.

## Notes on the maths

* PSI uses quantile bins (not equal width) to avoid collapsing long tails, with unbounded outer bins so out of range values are still counted.
* KS is computed with a two pointer pass over sorted values to avoid extra allocations.
* Severity bands follow a common credit risk convention. PSI < 0.10 is stable, 0.10 to < 0.25 is moderate, and >= 0.25 is significant.

## Development

```bash
cargo test
cargo clippy --all-targets
cargo fmt --check
cargo build --release
```

## License

MIT License. See [LICENSE](LICENSE) for details.
