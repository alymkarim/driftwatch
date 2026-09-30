use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum DatasetError {
    Io(std::io::Error),
    Csv(csv::Error),
    Empty,
    RaggedRow {
        line: usize,
        expected: usize,
        found: usize,
    },
    NonNumeric {
        column: String,
        line: usize,
        value: String,
    },
    MissingColumn {
        name: String,
        available: usize,
    },
}

impl fmt::Display for DatasetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "could not read input: {e}"),
            Self::Csv(e) => write!(f, "malformed csv: {e}"),
            Self::Empty => write!(f, "no data rows found"),
            Self::RaggedRow {
                line,
                expected,
                found,
            } => {
                write!(f, "line {line}: expected {expected} fields, found {found}")
            }
            Self::NonNumeric {
                column,
                line,
                value,
            } => {
                write!(
                    f,
                    "line {line}, column '{column}': '{value}' is not numeric"
                )
            }
            Self::MissingColumn { name, available } => {
                write!(
                    f,
                    "column '{name}' is not in the baseline ({available} columns found)"
                )
            }
        }
    }
}

impl std::error::Error for DatasetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Csv(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for DatasetError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<csv::Error> for DatasetError {
    fn from(e: csv::Error) -> Self {
        Self::Csv(e)
    }
}

pub struct Dataset {
    header: Vec<String>,
    rows: Vec<Vec<f64>>,
}

impl Dataset {
    pub fn from_path(path: &Path) -> Result<Self, DatasetError> {
        let mut reader = csv::Reader::from_path(path)?;
        let header: Vec<String> = reader
            .headers()?
            .iter()
            .map(str::trim)
            .map(str::to_owned)
            .collect();

        if header.is_empty() {
            return Err(DatasetError::Empty);
        }

        let mut rows = Vec::new();
        for (offset, record) in reader.records().enumerate() {
            let record = record?;
            let line = offset + 2;

            if record.len() != header.len() {
                return Err(DatasetError::RaggedRow {
                    line,
                    expected: header.len(),
                    found: record.len(),
                });
            }

            let mut row = Vec::with_capacity(header.len());
            for (name, field) in header.iter().zip(record.iter()) {
                let value = field.trim();
                let parsed = value.parse::<f64>().map_err(|_| DatasetError::NonNumeric {
                    column: name.clone(),
                    line,
                    value: value.to_owned(),
                })?;
                row.push(parsed);
            }
            rows.push(row);
        }

        if rows.is_empty() {
            return Err(DatasetError::Empty);
        }

        Ok(Self { header, rows })
    }

    pub fn column_names(&self) -> impl Iterator<Item = &str> {
        self.header.iter().map(String::as_str)
    }

    /// Values for `name`, in file order.
    ///
    /// A column present in the baseline but absent from the comparison set is a
    /// hard error rather than a silently skipped check, since a dropped feature
    /// would quietly remove it from the drift report.
    pub fn column(&self, name: &str) -> Result<Vec<f64>, DatasetError> {
        let idx = self.header.iter().position(|h| h == name).ok_or_else(|| {
            DatasetError::MissingColumn {
                name: name.to_owned(),
                available: self.header.len(),
            }
        })?;

        Ok(self.rows.iter().map(|row| row[idx]).collect())
    }
}
