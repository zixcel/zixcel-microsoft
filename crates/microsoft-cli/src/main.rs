fn main() -> std::process::ExitCode {
    let result = (|| {
        let manifest = zixcel_microsoft_cli::manifest()?;
        let catalog = zixcel_microsoft_cli::guidance()?;
        Ok::<_, zix_native_provider::Error>((manifest, catalog))
    })();
    match result {
        Ok((manifest, catalog)) => zix_native_provider::serve_guided(
            manifest,
            catalog,
            zixcel_microsoft_cli::invoke::execute,
        ),
        Err(error) => {
            eprintln!("{{\"error\":\"{}\"}}", error.0);
            std::process::ExitCode::from(2)
        }
    }
}
