use super::DailyReportError;
use super::artifact_contract::DepartmentDailyReportV1;
use super::identity::{contract_reference, identifier, identifier_list};
use super::time::{date, date_time};

impl DepartmentDailyReportV1 {
    pub(crate) fn validate(&self) -> Result<(), DailyReportError> {
        contract_reference("artifact.schema", &self.schema)?;
        for (field, value) in [
            (
                "artifact.operational_report_id",
                &self.operational_report_id,
            ),
            ("artifact.headquarters_id", &self.headquarters_id),
            ("artifact.department_id", &self.department_id),
            ("artifact.reporting_team_id", &self.reporting_team_id),
            ("artifact.owner_account_id", &self.owner_account_id),
        ] {
            identifier(field, value)?;
        }
        date("artifact.reporting_date", &self.reporting_date)?;
        for (field, value) in [
            ("artifact.window_started_at", &self.window_started_at),
            ("artifact.window_ended_at", &self.window_ended_at),
            ("artifact.generated_at", &self.generated_at),
        ] {
            date_time(field, value)?;
        }
        identifier_list("artifact.source_case_ids", &self.source_case_ids, false)?;
        identifier_list("artifact.source_event_ids", &self.source_event_ids, false)?;
        identifier_list("artifact.source_task_ids", &self.source_task_ids, true)?;
        identifier_list("artifact.source_report_ids", &self.source_report_ids, true)?;
        identifier_list(
            "artifact.source_artifact_ids",
            &self.source_artifact_ids,
            true,
        )?;
        identifier_list(
            "artifact.escalated_decision_ids",
            &self.escalated_decision_ids,
            false,
        )?;
        if self.exceptions.len() > 1_024 {
            return Err(DailyReportError::new(
                "artifact.exceptions",
                "must contain at most 1024 exceptions",
            ));
        }
        for exception in &self.exceptions {
            exception.validate()?;
        }
        let summary_length = self.summary.chars().count();
        if summary_length == 0 || summary_length > 1_000 {
            return Err(DailyReportError::new(
                "artifact.summary",
                "must contain between 1 and 1000 characters",
            ));
        }
        Ok(())
    }
}
