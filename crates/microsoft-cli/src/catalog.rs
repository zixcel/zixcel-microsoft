use serde_json::{Value, json};
use zix_native_provider::{Error, Manifest};
pub fn manifest() -> zix_native_provider::Result<Manifest> {
    let mut m = crate::provider()?.manifest().clone();
    m.schema = "zix-extension/v2".into();
    m.timeout_ms = 120000;
    m.credential_environment.extend([
        "ZIX_MICROSOFT_EXCHANGE_TOKEN".into(),
        "ZIX_MICROSOFT_EXCHANGE_PROFILE".into(),
    ]);
    for (id, words, extras, effect) in [
        (
            "admin.organization",
            vec!["organization", "get"],
            vec![],
            "read",
        ),
        ("admin.licenses", vec!["licenses", "list"], vec![], "read"),
        ("admin.domains", vec!["domains", "list"], vec![], "read"),
        (
            "admin.domain-verification",
            vec!["domains", "verification-records"],
            vec!["domain"],
            "read",
        ),
        ("admin.users", vec!["users", "list"], vec![], "read"),
        ("admin.user", vec!["users", "inspect"], vec!["user"], "read"),
        (
            "admin.domain-dns",
            vec!["domains", "dns-records"],
            vec!["domain"],
            "read",
        ),
        (
            "exchange.snapshot",
            vec!["exchange", "snapshot"],
            vec![],
            "read",
        ),
        (
            "exchange.plan",
            vec!["exchange", "plan"],
            vec!["change-file"],
            "read",
        ),
        (
            "exchange.apply",
            vec!["exchange", "apply"],
            vec!["plan-file", "approval-sha256", "journal-directory"],
            "operator-write",
        ),
    ] {
        let mut args = vec![
            json!({"name":"tenant","required":true,"maximum":36}),
            json!({"name":"subject","required":true,"maximum":36}),
        ];
        for a in extras {
            args.push(
                json!({"name":a,"required":true,"maximum":if a=="approval-sha256"{64}else{2048}}),
            );
        }
        if id == "exchange.snapshot" {
            args.push(json!({"name":"mailbox","required":false,"maximum":36}));
            args.push(json!({"name":"change-file","required":false,"maximum":2048}));
        }
        if id == "admin.organization" {
            args.push(json!({"name":"resource","required":false,"maximum":32}));
            args.push(json!({"name":"identity","required":false,"maximum":36}));
        }
        if id == "exchange.snapshot" {
            for (name, max) in [
                ("resource", 32),
                ("kind", 32),
                ("folder", 256),
                ("trustee", 36),
            ] {
                args.push(json!({"name":name,"required":false,"maximum":max}));
            }
        }
        let exchange = id.starts_with("exchange.");
        if exchange {
            args.push(json!({"name":"runtime-profile","required":false,"maximum":2048}));
        }
        let permissions = if exchange {
            vec!["Exchange RBAC; delegated Graph identity verification."]
        } else {
            vec![
                "Delegated Graph scopes and roles: docs/microsoft-administration.md; groups: GroupMember.Read.All",
            ]
        };
        let env: Vec<&str> = if exchange {
            vec![
                "ZIX_MICROSOFT_ACCESS_TOKEN",
                "ZIX_MICROSOFT_EXCHANGE_TOKEN",
                "ZIX_MICROSOFT_EXCHANGE_PROFILE",
            ]
        } else {
            vec!["ZIX_MICROSOFT_ACCESS_TOKEN"]
        };
        m.commands.push(serde_json::from_value(json!({"id":id,"words":words,"arguments":args,"effect":effect,
            "description":"Tenant-bound Microsoft administration.",
            "required_permissions":permissions,"environment":{"credential_environment":env,"runtime_environment":[]}}))
            .map_err(|_|Error("invalid_admin_manifest"))?);
    }
    m.commands.push(serde_json::from_value(json!({"id":"admin.doctor","words":["admin","doctor"],"effect":"read",
        "description":"Report local setup prerequisites without tokens, login or remote calls.","required_permissions":[],"arguments":[],
        "environment":{"credential_environment":["ZIX_MICROSOFT_EXCHANGE_PROFILE"],"runtime_environment":[]}})).map_err(|_|Error("invalid_admin_manifest"))?);
    // The original exact-user read retains its smaller credential exposure and validation.
    m.commands[0].environment=Some(serde_json::from_value(json!({"credential_environment":["ZIX_MICROSOFT_ACCESS_TOKEN"],"runtime_environment":[]}))
        .map_err(|_|Error("invalid_admin_manifest"))?);
    crate::administration_catalog::extend(&mut m)?;
    m.validate()?;
    Ok(m)
}
pub fn catalog() -> zix_native_provider::Result<Value> {
    serde_json::to_value(manifest()?).map_err(|_| Error("invalid_manifest"))
}
pub fn binding() -> zix_native_provider::Result<zix_operation_contracts::Binding> {
    let bytes = zix_cli::files::bounded(
        std::fs::File::open("/proc/self/exe").map_err(|_| Error("executable_unavailable"))?,
        // Match the pinned native SDK executable-binding limit. Test binaries
        // retain line debug information and can exceed the old local 32 MiB cap.
        128 * 1024 * 1024,
    )?;
    Ok(zix_operation_contracts::Binding::for_catalog(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        &catalog()?,
        &zix_cli::files::digest(&bytes),
    ))
}
