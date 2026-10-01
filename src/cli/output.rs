use serde::Serialize;

pub(super) fn print_json(value: &impl Serialize) -> Result<(), String> {
    let rendered =
        serde_json::to_string_pretty(value).map_err(|_| "JSON serialization failed".to_owned())?;
    println!("{rendered}");
    Ok(())
}
