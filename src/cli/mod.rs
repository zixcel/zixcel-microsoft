mod commands;
mod daily_report;
mod file_input;
mod output;

use serde_json::json;

pub(crate) fn run(arguments: impl Iterator<Item = String>) -> Result<(), String> {
    commands::dispatch(arguments)
}

pub(crate) fn print_error(message: &str) {
    let error = json!({
        "schema": "zixcel://cli-error/v1",
        "connector": zixcel_microsoft::CONNECTOR,
        "status": "error",
        "message": message
    });
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&error)
            .unwrap_or_else(|_| "{\"status\":\"error\"}".to_owned())
    );
}

pub(super) fn one_path(arguments: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let path = arguments.next().ok_or_else(usage)?;
    expect_no_more(arguments)?;
    Ok(path)
}

pub(super) fn expect_no_more(arguments: &mut impl Iterator<Item = String>) -> Result<(), String> {
    if arguments.next().is_some() {
        Err(usage())
    } else {
        Ok(())
    }
}

pub(super) fn usage() -> String {
    "usage: zixcel-microsoft <doctor|capabilities|validate <config>|plan <config>|mail-schema|mail-plan <request>|plan-authorized <config> <request>|sharepoint-daily-report ...>".to_owned()
}
