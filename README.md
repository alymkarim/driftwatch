# driftwatch

Population stability checks for a scored population, written in Rust.

Given the population a model was validated on and the population it has since
scored, `driftwatch` reports how far the input distribution has moved, per
feature, using two established measures:

- **PSI** (Population Stability Index) — the banding most credit risk teams use
  to decide whether a shift is worth investigating.
- **KS** (two sample Kolmogorov–Smirnov) — distribution free, so it disagrees
  with PSI when movement is concentrated in one tail. Reporting both keeps a
  single summary statistic from hiding that.

## Why it is shaped like this

- **Quantile bins, not equal width.** Equal width binning collapses on skewed
  credit features: the bulk of a portfolio lands in one bucket and the tail,
  where the risk lives, disappears.
- **Outer bins are unbounded.** A current value that falls outside the baseline
  range still lands in a bin, so genuine tail movement is counted rather than
  silently dropped.
- **Empty input returns `0.0`.** A partial extract should not look like an
  emergency.
- **Missing columns are an error.** If a feature dropped out of the scoring run
  the report would understate the movement, so it fails loudly instead.

## Usage

```console
$ driftwatch baseline.csv current.csv
FEATURE                  PSI       KS  VERDICT
-------------------------------------------------
loan_amount           1.3285   0.4500  significant
debt_to_income        7.1418   0.7700  significant
credit_score_months   3.8405   0.7100  significant
utilisation           9.5123   0.8750  significant

largest shift: utilisation at psi 9.5123
```

```console
$ driftwatch baseline.csv current.csv --bins 20
```

## Exit codes

| Code | Meaning |
| ---- | ------- |
| `0`  | every feature stable or moderately shifted |
| `1`  | bad input, unreadable file, or a missing column |
| `2`  | at least one feature crossed into `significant` |

Exit `2` is what makes this usable in CI: a model promotion can be held when the
population it is being promoted into has moved.

## Development

```console
$ cargo test
$ cargo clippy --all-targets
$ cargo build --release
```

## Licence

MIT
