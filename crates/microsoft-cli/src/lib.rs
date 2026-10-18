pub fn provider() -> zix_native_provider::Result<zix_native_provider::Provider> {
    zix_native_provider::Provider::load(
        include_bytes!("../api/routes.json"),
        include_bytes!("../api/policy.json"),
        env!("CARGO_PKG_VERSION"),
    )
}
