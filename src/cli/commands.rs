use zixcel_microsoft::{
    build_authorized_plan, build_plan, capabilities, doctor, parse_config,
    parse_provider_plan_request, validation_report,
};

use super::daily_report;
use super::file_input::read_bounded_utf8;
use super::output::print_json;
use super::{expect_no_more, one_path, usage};

const MAX_CONFIG_BYTES: u64 = 1_048_576;

pub(super) fn dispatch(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let command = arguments.next().ok_or_else(usage)?;
    match command.as_str() {
        "doctor" => {
            expect_no_more(&mut arguments)?;
            print_json(&doctor())
        }
        "capabilities" => {
            expect_no_more(&mut arguments)?;
            print_json(&capabilities())
        }
        "validate" => {
            let path = one_path(&mut arguments)?;
            let config = load_config(&path)?;
            print_json(&validation_report(&config).map_err(|error| error.to_string())?)
        }
        "plan" => {
            let path = one_path(&mut arguments)?;
            let config = load_config(&path)?;
            print_json(&build_plan(&config).map_err(|error| error.to_string())?)
        }
        "plan-authorized" => {
            let config_path = arguments.next().ok_or_else(usage)?;
            let request_path = arguments.next().ok_or_else(usage)?;
            expect_no_more(&mut arguments)?;
            let config = load_config(&config_path)?;
            let source =
                read_bounded_utf8(&request_path, MAX_CONFIG_BYTES, "provider plan request")?;
            let request =
                parse_provider_plan_request(&source).map_err(|error| error.to_string())?;
            print_json(
                &build_authorized_plan(&config, &request).map_err(|error| error.to_string())?,
            )
        }
        "sharepoint-daily-report" => daily_report::dispatch(&mut arguments),
        "mail-plan" => {
            let path = one_path(&mut arguments)?;
            let source = read_bounded_utf8(&path, 16_384, "mail request")?;
            let request = zixcel_microsoft::mail::parse_mail_request(&source)
                .map_err(|error| error.to_string())?;
            print_json(
                &zixcel_microsoft::mail::build_mail_plan(&request)
                    .map_err(|error| error.to_string())?,
            )
        }
        "mail-schema" => {
            expect_no_more(&mut arguments)?;
            let schema: serde_json::Value =
                serde_json::from_str(zixcel_microsoft::mail::MAIL_REQUEST_SCHEMA)
                    .map_err(|_| "bundled mail schema is invalid".to_owned())?;
            print_json(&schema)
        }
        _ => Err(usage()),
    }
}

fn load_config(path: &str) -> Result<zixcel_microsoft::ConnectorConfig, String> {
    let source = read_bounded_utf8(path, MAX_CONFIG_BYTES, "configuration")?;
    parse_config(&source).map_err(|error| error.to_string())
}
