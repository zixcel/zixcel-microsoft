use std::fmt;

/// Structured contract failure for request, plan, or offline acceptance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyReportError {
    pub field: &'static str,
    pub message: &'static str,
}

impl DailyReportError {
    pub(crate) const fn new(field: &'static str, message: &'static str) -> Self {
        Self { field, message }
    }
}

impl fmt::Display for DailyReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for DailyReportError {}
