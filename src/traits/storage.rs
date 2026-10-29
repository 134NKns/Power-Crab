//! Storage Traits

use crate::storage::{StorageError, WifiCredentials};

/// Abstract interface for persistent Wi-Fi credential storage
pub trait WifiStorage {
    /// Load saved Wi-Fi credentials from storage.
    fn load_wifi(&mut self) -> Result<Option<WifiCredentials>, StorageError>;

    /// Save Wi-Fi credentials to storage.
    fn save_wifi(&mut self, creds: &WifiCredentials) -> Result<(), StorageError>;

    /// Erase saved Wi-Fi credentials from storage.
    fn erase_wifi(&mut self) -> Result<(), StorageError>;
}
