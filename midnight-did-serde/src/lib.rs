mod serde_rs;

pub use serde_rs::DefaultContractStateDeserializer;

#[cfg(feature = "js-cli")]
mod serde_cli;

#[cfg(feature = "js-cli")]
pub use serde_cli::CliContractStateDeserializer;

