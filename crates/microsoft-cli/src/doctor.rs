use serde_json::{Value, json};
use zix_native_provider::Error;
pub fn report() -> Result<Value, Error> {
    let profile = match std::env::var("ZIX_MICROSOFT_EXCHANGE_PROFILE") {
        Err(_) => "not_configured".to_string(),
        Ok(path) => {
            match crate::input::read::<zixcel_microsoft_admin::exchange::ExchangeProfile>(&path) {
                Ok(p) => match p.validate() {
                    Ok(()) => "paths_and_permissions_valid_not_live_tested".into(),
                    Err(e) => e.0.into(),
                },
                Err(e) => e.0.into(),
            }
        }
    };
    Ok(
        json!({"schema":"zix-microsoft-admin-doctor/v1","version":env!("CARGO_PKG_VERSION"),
        "exchange_profile":profile,"exchange_module_version":zixcel_microsoft_admin::exchange::EXCHANGE_MODULE_VERSION,
        "authentication":"not_attempted","permissions":"not_verified","credentials_read":false,"remote_requests":false,
        "containment":"must_be_enforced_before_child_start","implicit_installation":false}),
    )
}
