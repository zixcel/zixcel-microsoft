fn main() -> std::process::ExitCode {
    match zixcel_microsoft_cli::manifest() {
        Ok(m) => zix_native_provider::serve(m, zixcel_microsoft_cli::invoke::execute),
        Err(e) => {
            eprintln!("{{\"error\":\"{}\"}}", e.0);
            std::process::ExitCode::from(2)
        }
    }
}
