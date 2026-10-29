//! Storage Subsystem
//!
//! Provides credential representations and flash persistence.

pub mod creds;
pub mod flash;

pub use creds::{StorageError, WifiCredentials, MAX_PASS_LEN, MAX_SSID_LEN};
pub use flash::Storage;
