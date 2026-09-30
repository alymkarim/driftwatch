use std::fmt;
use std::path::Path;

use crate::dataset::Dataset;
use crate::statistics::{Severity, ks_statistic, psi};

pub struct FeatureReport {
    pub name: String,
    pub psi: f64,
    pub ks: f64,
    pub severity: Severity,
}

pub struct DriftReport {
    pub features: Vec<FeatureReport>,
}

impl DriftReport {
    /// Compares every feature in `baseline` against `current`.
    ///
    /// Extra columns in `current` are ignored. A missing one is an error, since
    /// that would mean a feature dropped out of the scoring run and the report
    /// would understate the movement.
    pub fn compare(
        baseline: &Dataset,
        current: &Dataset,
        bins: usize,
    ) -> Result<Self, crate::DatasetError> {
        let mut features = Vec::new();

        for name in baseline.column_names() {
            let base = baseline.column(name)?;
            let cur = current.column(name)?;

            let psi = psi(&base, &cur, bins);
            features.push(FeatureReport {
                name: name.to_owned(),
                psi,
                ks: ks_statistic(&base, &cur),
                severity: Severity::from_psi(psi),
            });
        }

        Ok(Self { features })
    }

    pub fn worst(&self) -> Option<&FeatureReport> {
        self.features.iter().max_by(|a, b| a.psi.total_cmp(&b.psi))
    }

    pub fn needs_review(&self) -> bool {
        self.features
            .iter()
            .any(|f| f.severity == Severity::Significant)
    }
}

impl fmt::Display for DriftReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = self
            .features
            .iter()
            .map(|feat| feat.name.len())
            .max()
            .unwrap_or(4)
            .max(4);

        writeln!(
            f,
            "{:<width$}  {:>7}  {:>7}  VERDICT",
            "FEATURE", "PSI", "KS"
        )?;
        let rule = "-".repeat(width + 30);
        writeln!(f, "{rule}")?;

        for feat in &self.features {
            writeln!(
                f,
                "{:<width$}  {:>7.4}  {:>7.4}  {}",
                feat.name, feat.psi, feat.ks, feat.severity
            )?;
        }

        if let Some(worst) = self.worst() {
            writeln!(f)?;
            writeln!(f, "largest shift: {} at psi {:.4}", worst.name, worst.psi)?;
        }

        Ok(())
    }
}

pub fn run(
    baseline_path: &Path,
    current_path: &Path,
    bins: usize,
) -> Result<DriftReport, crate::DatasetError> {
    let baseline = Dataset::from_path(baseline_path)?;
    let current = Dataset::from_path(current_path)?;
    DriftReport::compare(&baseline, &current, bins)
}
