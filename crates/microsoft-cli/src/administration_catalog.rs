use serde_json::json;
use zix_native_provider::{Error, Manifest};
pub fn extend(m: &mut Manifest) -> Result<(), Error> {
    m.credential_environment
        .push("ZIX_MICROSOFT_INITIAL_PASSWORD".into());
    m.runtime_environment
        .push("DBUS_SESSION_BUS_ADDRESS".into());
    for group in ["domains", "users", "licenses", "dkim", "mail"] {
        for phase in ["plan", "apply"] {
            let mut args = scope();
            for name in if phase == "plan" {
                vec!["change-file"]
            } else {
                vec!["plan-file", "approval-sha256", "journal-directory"]
            } {
                args.push(arg(
                    name,
                    true,
                    if name == "approval-sha256" { 64 } else { 2048 },
                ));
            }
            if group == "dkim" {
                args.push(arg("runtime-profile", false, 2048));
            }
            let mut env = vec!["ZIX_MICROSOFT_ACCESS_TOKEN"];
            if group == "dkim" {
                env.extend([
                    "ZIX_MICROSOFT_EXCHANGE_TOKEN",
                    "ZIX_MICROSOFT_EXCHANGE_PROFILE",
                ]);
            }
            if group == "users" && phase == "apply" {
                env.push("ZIX_MICROSOFT_INITIAL_PASSWORD");
            }
            add(
                m,
                &format!("management.{group}.{phase}"),
                vec![group, phase],
                args,
                if phase == "apply" {
                    "operator-write"
                } else {
                    "read"
                },
                env,
                vec![],
            )?;
        }
    }
    let mut args = scope();
    args.push(arg("domain", true, 253));
    args.push(arg("runtime-profile", false, 2048));
    add(
        m,
        "management.dkim.inspect",
        vec!["dkim", "inspect"],
        args,
        "read",
        vec![
            "ZIX_MICROSOFT_ACCESS_TOKEN",
            "ZIX_MICROSOFT_EXCHANGE_TOKEN",
            "ZIX_MICROSOFT_EXCHANGE_PROFILE",
        ],
        vec![],
    )?;
    let mut args = scope();
    args.push(arg("correlation", true, 36));
    add(
        m,
        "management.mail.inspect",
        vec!["mail", "probe-inspect"],
        args,
        "read",
        vec!["ZIX_MICROSOFT_ACCESS_TOKEN"],
        vec![],
    )?;
    add(
        m,
        "auth.configure",
        vec!["auth", "configure"],
        vec![
            arg("tenant", true, 36),
            arg("client-id", true, 36),
            arg("collection", true, 64),
            arg("graph-permissions", false, 2048),
            arg("subject", false, 36),
            arg("output", true, 2048),
        ],
        "read",
        vec![],
        vec![],
    )?;
    for phase in ["start", "poll", "refresh", "status", "logout"] {
        add(
            m,
            &format!("auth.{phase}"),
            vec!["auth", phase],
            vec![arg("profile", true, 2048), arg("resource", true, 16)],
            "read",
            vec![],
            vec!["DBUS_SESSION_BUS_ADDRESS"],
        )?;
    }
    let args = [
        ("pwsh", 2048),
        ("module", 2048),
        ("directory", 2048),
        ("user", 320),
        ("output", 2048),
    ]
    .into_iter()
    .map(|(n, s)| arg(n, true, s))
    .collect();
    add(
        m,
        "runtime.configure",
        vec!["runtime", "configure"],
        args,
        "read",
        vec![],
        vec![],
    )?;
    add(
        m,
        "runtime.inspect",
        vec!["runtime", "inspect"],
        vec![arg("profile", true, 2048)],
        "read",
        vec![],
        vec![],
    )?;
    Ok(())
}
fn arg(name: &str, required: bool, max: usize) -> serde_json::Value {
    json!({"name":name,"required":required,"maximum":max})
}
fn scope() -> Vec<serde_json::Value> {
    vec![arg("tenant", true, 36), arg("subject", true, 36)]
}
fn add(
    m: &mut Manifest,
    id: &str,
    words: Vec<&str>,
    args: Vec<serde_json::Value>,
    effect: &str,
    credentials: Vec<&str>,
    runtime: Vec<&str>,
) -> Result<(), Error> {
    m.commands.push(serde_json::from_value(json!({"id":id,"words":words,"effect":effect,"arguments":args,
        "description":"Explicit Microsoft authentication, runtime setup or tenant-scoped administration; writes require reviewed one-use plans.",
        "required_permissions":if id.starts_with("auth.")||id.starts_with("runtime."){vec![]}else{vec!["Delegated Graph scopes and Entra roles per selected operation; Exchange RBAC for DKIM. No permission is inferred or granted."]},
        "environment":{"credential_environment":credentials,"runtime_environment":runtime}})).map_err(|_|Error("invalid_admin_manifest"))?);
    Ok(())
}
