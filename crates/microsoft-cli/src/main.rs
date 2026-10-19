fn main() -> std::process::ExitCode {
    zix_native_provider::entry(
        include_bytes!("../api/routes.json"),
        include_bytes!("../api/policy.json"),
        env!("CARGO_PKG_VERSION"),
    )
}
