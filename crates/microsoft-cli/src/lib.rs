pub mod catalog;
mod doctor;
mod exchange;
mod exchange_reconciliation;
mod input;
pub mod invoke;
pub use catalog::manifest;
pub fn provider() -> zix_native_provider::Result<zix_native_provider::Provider> {
    zix_native_provider::Provider::load(
        include_bytes!("../api/routes.json"),
        include_bytes!("../api/policy.json"),
        env!("CARGO_PKG_VERSION"),
    )
}

mod administration_catalog;
mod auth;
mod configuration;
mod configuration_reconciliation;
mod management;
mod runtime;

mod reconciliation;

pub fn guidance() -> zix_native_provider::Result<zix_native_provider::GuidanceCatalog> {
    let manifest = manifest()?;
    zix_native_provider::guidance::catalog(&manifest, include_bytes!("../api/guidance.json"))
}
