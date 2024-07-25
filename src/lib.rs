pub mod api;
pub mod config;
pub mod crypto;
pub mod error;
pub mod opts;
pub mod privatebin;
pub mod util;

#[cfg(feature = "uniffi")]
mod uniffi_custom_types;

pub use api::API;
pub use error::{PasteError, PbResult};
pub use opts::Opts;
pub use privatebin::{DecryptedPaste, PasteFormat};
pub use util::check_filesize;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
