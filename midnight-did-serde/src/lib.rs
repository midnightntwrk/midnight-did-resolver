mod serde_impl;

#[cfg(feature = "js-cli")]
mod serde_cli;

#[cfg(feature = "js-cli")]
pub use serde_cli::CliContractStateDeserializer;
pub use serde_impl::DefaultContractStateDeserializer;
