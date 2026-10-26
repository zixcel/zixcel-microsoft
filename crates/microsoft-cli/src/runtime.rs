use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
use zix_native_provider::{Error, Request};
use zixcel_microsoft_admin::exchange::ExchangeProfile;
pub fn invoke(r: &Request) -> Result<Value, Error> {
    let arg = |name| {
        r.arguments
            .get(name)
            .map(String::as_str)
            .ok_or(Error("missing_argument"))
    };
    if r.operation == "runtime.inspect" {
        let p: ExchangeProfile = crate::input::read(arg("profile")?)?;
        return zixcel_microsoft_admin::exchange::inspect_runtime(
            &p,
            Duration::from_millis(r.limits.timeout_ms.saturating_sub(100)),
        )
        .map_err(|e| Error(e.0));
    }
    if r.operation != "runtime.configure" {
        return Err(Error("unknown_operation"));
    }
    let p = ExchangeProfile {
        pwsh: PathBuf::from(arg("pwsh")?),
        module_manifest: PathBuf::from(arg("module")?),
        working_directory: PathBuf::from(arg("directory")?),
        user_principal_name: arg("user")?.into(),
    };
    p.validate().map_err(|e| Error(e.0))?;
    let path = arg("output")?;
    crate::input::write_private_new(path, &p)?;
    Ok(
        json!({"profile":path,"stored":true,"credentials_stored":false,"authentication":"not_attempted"}),
    )
}
