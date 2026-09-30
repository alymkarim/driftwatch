use std::fmt;

pub const DEFAULT_BINS: usize = 10;

/// Bins are quantile based rather than equal width. Equal width breaks down on
/// skewed credit features, where the bulk of the population lands in one bucket
/// and the tail is invisible.
const ZERO_BIN_EPSILON: f64 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Stable,
    Moderate,
    Significant,
}

impl Severity {
    /// Thresholds follow the bands credit risk teams have used for PSI since
    /// the early 2000s: below 0.1 a shift is noise, 0.1-0.25 warrants a look,
    /// above 0.25 the population has moved enough to suspect the model.
    pub fn from_psi(psi: f64) -> Self {
        if psi < 0.1 {
            Self::Stable
        } else if psi < 0.25 {
            Self::Moderate
        } else {
            Self::Significant
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Stable => "stable",
            Self::Moderate => "moderate",
            Self::Significant => "significant",
        };
        f.write_str(label)
    }
}

/// Population Stability Index between a baseline and a current sample.
///
/// Returns 0.0 when either sample is empty, which keeps a partial extract from
/// reporting a false emergency.
pub fn psi(baseline: &[f64], current: &[f64], bins: usize) -> f64 {
    if baseline.is_empty() || current.is_empty() {
        return 0.0;
    }

    let edges = quantile_edges(baseline, bins);
    let expected = shares(baseline, &edges);
    let actual = shares(current, &edges);

    expected
        .iter()
        .zip(&actual)
        .map(|(e, a)| {
            let e = e.max(ZERO_BIN_EPSILON);
            let a = a.max(ZERO_BIN_EPSILON);
            (a - e) * (a / e).ln()
        })
        .sum()
}

/// Two sample Kolmogorov-Smirnov statistic, the maximum gap between the
/// empirical CDFs. Reported alongside PSI because it is distribution free and
/// so disagrees with PSI when a shift is concentrated in one tail.
pub fn ks_statistic(baseline: &[f64], current: &[f64]) -> f64 {
    if baseline.is_empty() || current.is_empty() {
        return 0.0;
    }

    let mut a = baseline.to_vec();
    let mut b = current.to_vec();
    a.sort_by(f64::total_cmp);
    b.sort_by(f64::total_cmp);

    let (mut i, mut j) = (0usize, 0usize);
    let mut max_gap = 0.0f64;

    while i < a.len() && j < b.len() {
        let (lo, hi) = (a[i], b[j]);
        if lo < hi {
            i += 1;
        } else if hi < lo {
            j += 1;
        } else {
            i += 1;
            j += 1;
        }

        let gap = ((i as f64 / a.len() as f64) - (j as f64 / b.len() as f64)).abs();
        if gap > max_gap {
            max_gap = gap;
        }
    }

    max_gap
}

/// Interior cut points at evenly spaced quantiles of the baseline.
fn quantile_edges(baseline: &[f64], bins: usize) -> Vec<f64> {
    if bins < 2 {
        return Vec::new();
    }

    let mut sorted = baseline.to_vec();
    sorted.sort_by(f64::total_cmp);

    (1..bins)
        .map(|i| {
            let rank = i * sorted.len() / bins;
            sorted[rank.min(sorted.len() - 1)]
        })
        .collect()
}

/// Proportion of `sample` falling in each bin defined by `edges`.
///
/// The first bin is unbounded below and the last unbounded above so that
/// current values outside the baseline range are counted rather than dropped,
/// which is exactly the movement worth catching.
fn shares(sample: &[f64], edges: &[f64]) -> Vec<f64> {
    let mut counts = vec![0usize; edges.len() + 1];

    for value in sample {
        let bin = edges.partition_point(|edge| *edge < *value);
        counts[bin] += 1;
    }

    let total = sample.len() as f64;
    counts.into_iter().map(|c| c as f64 / total).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(n: usize) -> Vec<f64> {
        (0..n).map(|i| i as f64).collect()
    }

    #[test]
    fn identical_samples_have_no_drift() {
        let sample = ramp(500);
        assert!(psi(&sample, &sample, DEFAULT_BINS).abs() < 1e-9);
        assert_eq!(ks_statistic(&sample, &sample), 0.0);
    }

    #[test]
    fn shifted_population_shows_up() {
        let baseline = ramp(500);
        let shifted: Vec<f64> = ramp(500).into_iter().map(|v| v + 200.0).collect();

        assert!(
            psi(&baseline, &shifted, DEFAULT_BINS) > 0.25,
            "a clean shift should clear the significant band"
        );

        // The samples overlap on 300 of 500 values, so the widest gap between the
        // two CDFs is 200/500.
        let ks = ks_statistic(&baseline, &shifted);
        assert!((ks - 0.4).abs() < 1e-9, "expected ks of 0.4, got {ks}");
    }

    #[test]
    fn empty_input_is_not_an_emergency() {
        let sample = ramp(10);
        assert_eq!(psi(&[], &sample, DEFAULT_BINS), 0.0);
        assert_eq!(psi(&sample, &[], DEFAULT_BINS), 0.0);
    }

    #[test]
    fn a_collapsed_column_does_not_divide_by_zero() {
        let baseline = vec![7.0; 100];
        let current = vec![7.0, 7.0, 8.0, 9.0];

        let value = psi(&baseline, &current, 4);
        assert!(value.is_finite(), "psi must stay finite, got {value}");
    }

    #[test]
    fn severity_bands_follow_the_credit_risk_convention() {
        assert_eq!(Severity::from_psi(0.05), Severity::Stable);
        assert_eq!(Severity::from_psi(0.15), Severity::Moderate);
        assert_eq!(Severity::from_psi(0.4), Severity::Significant);
    }

    #[test]
    fn out_of_range_values_stay_in_the_report() {
        let baseline = ramp(100);
        let mut current = ramp(100);
        current.push(1_000.0);

        assert!(
            psi(&baseline, &current, DEFAULT_BINS) > 0.0,
            "a value beyond the baseline range must not be silently discarded"
        );
    }
}
